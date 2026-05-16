# Structure & Flow

Architecture reference for the Stellar Rug-Pull Detector — what the contracts do, how they store data, what events they emit, and exactly how the backend should integrate with each one.

---

## 1. Contract Overview

```
contracts/
├── verification-registry      # Source of truth for asset/issuer risk data
├── community-reporting        # Decentralized scam reporting + voting
├── staking-reputation         # Token staking → reputation scores
└── decentralized-alert-feed   # Trustless on-chain alert publishing
```

All four are Soroban contracts (Rust, `soroban-sdk 22`), deployed independently on Stellar. The backend is the primary writer; the frontend reads via the backend API.

---

## 2. Contract Details

---

### 2.1 `verification-registry`

**Role:** The on-chain registry of known assets and issuers. The risk engine writes scores here; the frontend reads them.

#### Storage

| Key | Type | Description |
|---|---|---|
| `Admin` | `Address` | Contract admin (instance storage) |
| `Asset(asset_code)` | `AssetRecord` | Per-asset risk data (persistent) |
| `Issuer(address)` | `IssuerRecord` | Per-issuer reputation data (persistent) |

#### Data Structures

```
AssetRecord {
    asset_code: String,
    issuer:     Address,
    verified:   bool,
    risk_score: u32,    // 0–100, higher = riskier
    flags:      u32,    // bitmask: 1=freeze, 2=clawback, 4=auth_required
}

IssuerRecord {
    issuer:         Address,
    approved:       bool,
    reputation:     i32,   // can go negative
    reported_count: u32,
}
```

#### Functions

| Function | Auth | Description |
|---|---|---|
| `initialize(admin)` | — | One-time setup |
| `register_asset(code, issuer, score, flags)` | Admin | Create/update asset record |
| `verify_asset(code)` | Admin | Mark asset as verified |
| `update_risk_score(code, score)` | Admin | Update risk score (0–100) |
| `get_asset(code)` | Public | Read asset record |
| `register_issuer(issuer)` | Admin | Register an issuer |
| `approve_issuer(issuer)` | Admin | Mark issuer as approved |
| `adjust_reputation(issuer, delta)` | Admin | Add/subtract reputation (signed delta) |
| `get_issuer(issuer)` | Public | Read issuer record |

#### Events Emitted

| Event | Payload |
|---|---|
| `asset_registered` | `asset_code` |
| `asset_verified` | `asset_code` |
| `issuer_registered` | `issuer_address` |

#### What's NOT yet implemented (remaining 50%)
- Batch asset registration
- Risk score history / changelog
- Cross-contract read from `community-reporting` to auto-increment `reported_count`
- Public flag to mark asset as suspected rug without admin

---

### 2.2 `community-reporting`

**Role:** Anyone can file a scam report. The community votes to confirm or reject it. Auto-resolves at 5 votes; admin can override.

#### Storage

| Key | Type | Description |
|---|---|---|
| `Admin` | `Address` | Contract admin (instance) |
| `ReportCount` | `u64` | Auto-incrementing ID counter (instance) |
| `Report(id)` | `Report` | Full report record (persistent) |
| `Votes(id)` | `VoteTally` | Vote counts per report (persistent) |
| `UserVoted(id, voter)` | `bool` | Duplicate vote guard (temporary storage) |

#### Data Structures

```
Report {
    id:          u64,
    reporter:    Address,
    asset_code:  String,
    description: String,
    status:      ReportStatus,  // Pending | Confirmed | Rejected
    created_at:  u64,           // ledger timestamp
}

VoteTally {
    confirm_votes: u32,
    reject_votes:  u32,
}
```

#### Functions

| Function | Auth | Description |
|---|---|---|
| `initialize(admin)` | — | One-time setup |
| `submit_report(reporter, code, desc)` | Reporter | File a new scam report, returns `report_id` |
| `vote(voter, report_id, confirm)` | Voter | Cast one vote; auto-resolves at 5 |
| `resolve_report(report_id, confirmed)` | Admin | Manual override resolution |
| `get_report(id)` | Public | Read report |
| `get_votes(id)` | Public | Read vote tally |
| `report_count()` | Public | Total reports filed |

#### Events Emitted

| Event | Payload |
|---|---|
| `report_submitted` | `(report_id, asset_code)` |

#### Auto-resolve Logic

```
if confirm_votes >= 5  → status = Confirmed
if reject_votes >= 5   → status = Rejected
```

Duplicate vote protection uses **temporary storage** (expires with ledger TTL — intentional; prevents permanent state bloat while still blocking same-session double votes).

#### What's NOT yet implemented (remaining 50%)
- Reputation-weighted voting (integrate with `staking-reputation`)
- Reward reporters when report is confirmed
- Slash false reporters when report is rejected
- Pagination for `get_reports_by_asset`

---

### 2.3 `staking-reputation`

**Role:** Users stake a SAC token to earn reputation. Reputation is used to weight votes in `community-reporting` and gate access to publishing in `decentralized-alert-feed`.

#### Storage

| Key | Type | Description |
|---|---|---|
| `Admin` | `Address` | Contract admin (instance) |
| `StakeToken` | `Address` | SAC token contract address (instance) |
| `Stake(staker)` | `StakeRecord` | Active stake per user (persistent) |
| `Reputation(address)` | `i64` | Reputation score per address (persistent) |

#### Data Structures

```
StakeRecord {
    staker:       Address,
    amount:       i128,
    staked_at:    u64,   // ledger timestamp
    locked_until: u64,   // ledger timestamp — cannot unstake before this
}
```

#### Functions

| Function | Auth | Description |
|---|---|---|
| `initialize(admin, stake_token)` | — | One-time setup with SAC token address |
| `stake(staker, amount, lock_duration)` | Staker | Transfer tokens in, accrue reputation |
| `unstake(staker)` | Staker | Return tokens after lock expires |
| `slash(staker, penalty)` | Admin | Reduce reputation (penalize bad actors) |
| `reward(staker, bonus)` | Admin | Increase reputation (reward good actors) |
| `get_stake(staker)` | Public | Read stake record |
| `get_reputation(addr)` | Public | Read reputation score |

#### Reputation Formula

```
rep_delta = stake_amount / 1_000_000   // 1 rep per 1 token (7-decimal SAC)
```

Reputation is **not removed on unstake** — it persists and can only be changed by admin slash/reward. This is intentional: reputation reflects historical commitment, not current stake.

#### Events Emitted

| Event | Payload |
|---|---|
| `staked` | `(staker, amount, locked_until)` |
| `unstaked` | `(staker, amount)` |
| `slashed` | `(staker, penalty)` |

#### What's NOT yet implemented (remaining 50%)
- Minimum stake threshold to participate in voting
- Stake confiscation on slash (currently only reduces reputation)
- Multiple concurrent stakes per address
- Reputation decay over time

---

### 2.4 `decentralized-alert-feed`

**Role:** Authorized publishers post on-chain alerts about risky assets. The backend risk engine is the primary publisher. Alerts are immutable once published; only `active` flag can be toggled.

#### Storage

| Key | Type | Description |
|---|---|---|
| `Admin` | `Address` | Contract admin (instance) |
| `AlertCount` | `u64` | Auto-incrementing ID (instance) |
| `Alert(id)` | `Alert` | Full alert record (persistent) |
| `Publisher(address)` | `bool` | Authorized publisher flag (persistent) |

#### Data Structures

```
Alert {
    id:         u64,
    publisher:  Address,
    asset_code: String,
    severity:   AlertSeverity,  // Info | Warning | Critical
    message:    String,
    timestamp:  u64,
    active:     bool,
}
```

#### Functions

| Function | Auth | Description |
|---|---|---|
| `initialize(admin)` | — | One-time setup |
| `add_publisher(publisher)` | Admin | Authorize a publisher address |
| `remove_publisher(publisher)` | Admin | Revoke publisher access |
| `is_publisher(publisher)` | Public | Check authorization |
| `publish_alert(publisher, code, severity, msg)` | Publisher | Post a new alert, returns `alert_id` |
| `deactivate_alert(caller, alert_id)` | Publisher or Admin | Mark alert inactive |
| `get_alert(id)` | Public | Read alert |
| `alert_count()` | Public | Total alerts published |

#### Events Emitted

| Event | Payload |
|---|---|
| `publisher_added` | `publisher_address` |
| `alert_published` | `(alert_id, asset_code, severity)` |

#### What's NOT yet implemented (remaining 50%)
- Alert expiry / TTL
- Alert categories (rug, wash_trade, fake_asset, etc.)
- Subscriber notification hooks
- Pagination / filtering by asset or severity

---

## 3. Contract Interaction Map

```
                    ┌─────────────────────────┐
                    │     Backend / Risk Engine│
                    └────────────┬────────────┘
                                 │
          ┌──────────────────────┼──────────────────────┐
          │                      │                      │
          ▼                      ▼                      ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────┐
│ verification-    │  │ decentralized-   │  │ staking-reputation   │
│ registry         │  │ alert-feed       │  │                      │
│                  │  │                  │  │ stake() → rep accrues│
│ register_asset() │  │ publish_alert()  │  │ slash() / reward()   │
│ update_risk_     │  │ (backend is      │  │                      │
│   score()        │  │  authorized      │  └──────────┬───────────┘
│ approve_issuer() │  │  publisher)      │             │
└──────────────────┘  └──────────────────┘             │ reputation
                                                        │ weight
                    ┌───────────────────────────────────▼──┐
                    │         community-reporting           │
                    │                                       │
                    │  submit_report()  ← any user          │
                    │  vote()           ← stakers (future)  │
                    │  resolve_report() ← admin             │
                    └───────────────────────────────────────┘
```

---

## 4. Backend Integration Guide

### 4.1 Stack

```
NestJS (TypeScript)  ←→  Stellar SDK (@stellar/stellar-sdk)
                     ←→  Soroban RPC (soroban-client)
                     ←→  PostgreSQL  (mirror of on-chain state)
                     ←→  Redis       (alert cache, rate limiting)
                     ←→  Kafka       (event streaming pipeline)
```

### 4.2 Service Responsibilities

```
backend/
├── stellar-indexer/          # Polls Horizon + streams Soroban events
├── risk-engine/              # Computes risk scores, calls verification-registry
├── soroban-event-listener/   # Listens for contract events, writes to Kafka
├── scoring-service/          # Aggregates scores, triggers alert-feed publishes
├── notification-service/     # Consumes Kafka alerts → Telegram/Discord/email
└── api-gateway/              # REST + WebSocket API for frontend
```

### 4.3 Risk Engine → `verification-registry`

The risk engine is the **only writer** to `verification-registry`. Flow:

```
1. stellar-indexer detects new asset or issuer activity
2. risk-engine computes composite score (see weights below)
3. risk-engine calls:
     verification-registry.register_asset(code, issuer, score, flags)
     OR
     verification-registry.update_risk_score(code, new_score)
4. If issuer is clean:
     verification-registry.approve_issuer(issuer)
     verification-registry.adjust_reputation(issuer, +delta)
5. If issuer is flagged:
     verification-registry.adjust_reputation(issuer, -delta)
```

**Risk score computation (backend-side):**

| Factor | Weight | Source |
|---|---|---|
| Issuer concentration | 25% | Horizon account data |
| LP ownership | 20% | AMM pool queries |
| Wallet clustering | 15% | Neo4j graph engine |
| Sudden minting | 15% | Ledger change stream |
| Wash trading | 10% | Trade history analysis |
| Trustline manipulation | 10% | Trustline delta monitoring |
| Fake metadata | 5% | `stellar.toml` parser |

```typescript
// Pseudocode — risk-engine/src/scorer.ts
async function computeRiskScore(assetCode: string, issuer: string): Promise<number> {
  const [concentration, lpOwnership, clustering, minting, washTrading, trustlines, metadata] =
    await Promise.all([
      getIssuerConcentration(issuer),
      getLpOwnership(assetCode),
      getWalletClustering(issuer),
      getMintingActivity(issuer),
      getWashTradingScore(assetCode),
      getTrustlineManipulation(assetCode),
      getMetadataScore(issuer),
    ]);

  return Math.round(
    concentration  * 0.25 +
    lpOwnership    * 0.20 +
    clustering     * 0.15 +
    minting        * 0.15 +
    washTrading    * 0.10 +
    trustlines     * 0.10 +
    metadata       * 0.05
  );
}
```

### 4.4 Scoring Service → `decentralized-alert-feed`

When a risk score crosses a threshold, the scoring service publishes an on-chain alert:

```
score 0–39   → no alert
score 40–69  → publish Info alert
score 70–84  → publish Warning alert
score 85–100 → publish Critical alert
```

```typescript
// Pseudocode — scoring-service/src/alerter.ts
async function maybePublishAlert(assetCode: string, score: number) {
  const severity = score >= 85 ? 'Critical'
                 : score >= 70 ? 'Warning'
                 : score >= 40 ? 'Info'
                 : null;

  if (!severity) return;

  await sorobanClient.invokeContract({
    contractId: ALERT_FEED_CONTRACT_ID,
    method: 'publish_alert',
    args: [publisherAddress, assetCode, severity, buildMessage(assetCode, score)],
    signerKeypair: backendKeypair,
  });
}
```

The backend's `publisherAddress` must be pre-authorized via `add_publisher()` by the admin.

### 4.5 Soroban Event Listener → Kafka

Listen for contract events and fan them out to downstream services:

```typescript
// soroban-event-listener/src/listener.ts
const filters = [
  { contractId: REGISTRY_CONTRACT_ID,  topics: ['asset_registered', 'asset_verified'] },
  { contractId: REPORTING_CONTRACT_ID, topics: ['report_submitted'] },
  { contractId: ALERT_FEED_CONTRACT_ID, topics: ['alert_published'] },
  { contractId: STAKING_CONTRACT_ID,   topics: ['staked', 'slashed'] },
];

// On each event → publish to Kafka topic: `soroban.events`
// notification-service consumes → sends Telegram/Discord/email
```

### 4.6 Community Reporting Integration

Users submit reports via the frontend → backend → contract:

```
Frontend form submit
  → POST /api/reports  (api-gateway)
  → api-gateway calls community-reporting.submit_report()
  → returns report_id
  → stored in PostgreSQL as mirror

Voting:
  → POST /api/reports/:id/vote
  → api-gateway calls community-reporting.vote()
  → on Confirmed: risk-engine re-scores the asset
  → on Confirmed: verification-registry.update_risk_score() called
```

### 4.7 Staking Integration

```
User stakes via frontend:
  → POST /api/stake  { amount, lock_duration }
  → api-gateway calls staking-reputation.stake()
  → reputation stored on-chain

Backend reads reputation before weighting votes:
  → staking-reputation.get_reputation(voter_address)
  → used to weight community-reporting votes (future)

Admin slash flow (when false report confirmed):
  → risk-engine detects false report
  → calls staking-reputation.slash(reporter, penalty)
```

### 4.8 PostgreSQL Mirror Schema

The backend mirrors on-chain state for fast API queries:

```sql
-- assets table mirrors verification-registry
CREATE TABLE assets (
  asset_code   TEXT PRIMARY KEY,
  issuer       TEXT NOT NULL,
  verified     BOOLEAN DEFAULT FALSE,
  risk_score   INTEGER DEFAULT 0,
  flags        INTEGER DEFAULT 0,
  updated_at   TIMESTAMPTZ DEFAULT NOW()
);

-- reports table mirrors community-reporting
CREATE TABLE reports (
  id           BIGINT PRIMARY KEY,
  reporter     TEXT NOT NULL,
  asset_code   TEXT NOT NULL,
  description  TEXT,
  status       TEXT DEFAULT 'Pending',
  created_at   TIMESTAMPTZ
);

-- alerts table mirrors decentralized-alert-feed
CREATE TABLE alerts (
  id           BIGINT PRIMARY KEY,
  publisher    TEXT NOT NULL,
  asset_code   TEXT NOT NULL,
  severity     TEXT NOT NULL,
  message      TEXT,
  timestamp    TIMESTAMPTZ,
  active       BOOLEAN DEFAULT TRUE
);

-- reputation table mirrors staking-reputation
CREATE TABLE reputation (
  address      TEXT PRIMARY KEY,
  score        BIGINT DEFAULT 0,
  stake_amount NUMERIC DEFAULT 0,
  updated_at   TIMESTAMPTZ DEFAULT NOW()
);
```

---

## 5. Full Data Flow

```
Stellar Network
      │
      │  new asset / trade / mint / trustline event
      ▼
stellar-indexer ──────────────────────────────────────────┐
      │                                                    │
      │  raw ledger data                                   │
      ▼                                                    ▼
risk-engine                                    soroban-event-listener
      │                                                    │
      │  compute score                                     │  contract events
      ▼                                                    ▼
verification-registry (write)                           Kafka
      │                                                    │
      │  score >= threshold                                │
      ▼                                                    ▼
scoring-service ──→ decentralized-alert-feed (write)  notification-service
                                                           │
                                                           ▼
                                              Telegram / Discord / Email


User submits report (frontend)
      │
      ▼
api-gateway ──→ community-reporting.submit_report()
                      │
                      │  5 votes confirmed
                      ▼
              risk-engine re-scores asset
                      │
                      ▼
              verification-registry.update_risk_score()
                      │
                      ▼
              scoring-service → alert-feed (if score jumped)
```

---

## 6. Environment Variables (Backend)

```env
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
HORIZON_URL=https://horizon-testnet.stellar.org
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

REGISTRY_CONTRACT_ID=C...
REPORTING_CONTRACT_ID=C...
STAKING_CONTRACT_ID=C...
ALERT_FEED_CONTRACT_ID=C...

BACKEND_ADMIN_SECRET=S...       # signs admin contract calls
BACKEND_PUBLISHER_SECRET=S...   # signs alert-feed publish calls

DATABASE_URL=postgresql://...
REDIS_URL=redis://...
KAFKA_BROKERS=localhost:9092
```

---

## 7. What's Built vs What's Next

### Built (contracts — 50%)

| Feature | Contract | Status |
|---|---|---|
| Asset registration + risk score | verification-registry | ✅ |
| Asset verification flag | verification-registry | ✅ |
| Issuer registration + approval | verification-registry | ✅ |
| Issuer reputation ±delta | verification-registry | ✅ |
| Scam report submission | community-reporting | ✅ |
| Community voting (1 per address) | community-reporting | ✅ |
| Auto-resolve at 5 votes | community-reporting | ✅ |
| Admin manual resolve | community-reporting | ✅ |
| Token staking with time lock | staking-reputation | ✅ |
| Reputation accrual on stake | staking-reputation | ✅ |
| Slash / reward by admin | staking-reputation | ✅ |
| Publisher allowlist | decentralized-alert-feed | ✅ |
| Alert publishing (Info/Warning/Critical) | decentralized-alert-feed | ✅ |
| Alert deactivation | decentralized-alert-feed | ✅ |

### Remaining (contracts — 50%)

| Feature | Contract | Notes |
|---|---|---|
| Reputation-weighted voting | community-reporting | Reads from staking-reputation |
| Reporter slash on false report | community-reporting + staking | Cross-contract call |
| Stake confiscation on slash | staking-reputation | Token transfer on slash |
| Risk score history | verification-registry | Append-only changelog |
| Alert categories + TTL | decentralized-alert-feed | Enum + expiry field |
| Batch operations | all | Gas efficiency |

### Backend (not yet started)

| Service | Priority |
|---|---|
| stellar-indexer | High — feeds everything |
| risk-engine | High — core scoring logic |
| soroban-event-listener | High — contract event bridge |
| api-gateway (REST) | High — frontend dependency |
| scoring-service | Medium |
| notification-service | Medium |
| wallet-clustering-engine | Low (Phase 3) |
| ml-anomaly-engine | Low (Phase 3) |
