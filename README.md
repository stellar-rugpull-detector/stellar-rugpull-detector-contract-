# Stellar Rug-Pull Detector

A real-time risk intelligence platform for the Stellar ecosystem. Scans tokens, issuers, liquidity pools, and DEX activity to detect scams, rug pulls, wash trading, and supply manipulation — before users interact with risky assets.

> Think **TokenSniffer + RugCheck + DEXTools**, built natively on Stellar + Soroban.

---

## Overview

Stellar's DeFi ecosystem is growing fast with Soroban smart contracts, AMMs, and cross-chain asset issuance — but on-chain security tooling is still nascent. This project provides a deep analytics and risk layer on top of the Stellar network, combining on-chain heuristics, ML anomaly detection, graph analysis, and community reporting into a unified platform.

**What it detects:**
- Rug pulls (soft and hard), exit liquidity setups
- Wash trading and fake volume
- Whale concentration and insider wallet clusters
- Sudden minting/burning by issuers
- Fake assets (copycat USDC/USDT, missing `stellar.toml`)
- Trustline manipulation and bot-created accounts
- Honeypot behavior and suspicious contract interactions

---

## Repository Structure

This is a monorepo with three top-level concerns:

```
stellar-rugpull-detector/
├── contracts/          # Soroban smart contracts (Rust) ← this repo
├── backend/            # Indexer, risk engine, API, ML services
└── frontend/           # Next.js dashboard + browser extension
```

---

## Smart Contracts

Located in `contracts/`. A Cargo workspace with four Soroban contracts written in Rust.

### Contracts

| Contract | Purpose |
|---|---|
| [`verification-registry`](contracts/verification-registry/) | Stores verified assets, approved issuers, and on-chain risk scores |
| [`community-reporting`](contracts/community-reporting/) | Scam report submission, community voting, DAO moderation |
| [`staking-reputation`](contracts/staking-reputation/) | Token staking for reputation, slash/reward mechanics |
| [`decentralized-alert-feed`](contracts/decentralized-alert-feed/) | Trustless on-chain alert publishing by authorized publishers |

### Contract Details

#### `verification-registry`
- Admin-controlled asset and issuer registry
- Tracks `risk_score` (0–100), `flags` bitmask (freeze/clawback/auth), and `verified` status
- Issuer reputation adjustable with positive/negative deltas
- Events emitted on registration and verification

#### `community-reporting`
- Any address can submit a scam report against an asset
- Community votes confirm or reject reports (one vote per address)
- Auto-resolves at 5 votes; admin can override at any time
- Duplicate vote protection via temporary storage

#### `staking-reputation`
- Users stake a configured SAC token with a time lock
- Reputation accrues proportionally to stake amount (1 rep per token)
- Admin can slash (penalize) or reward reputation
- Unstake blocked until lock period expires

#### `decentralized-alert-feed`
- Admin manages an allowlist of authorized publishers
- Publishers post alerts with severity: `Info`, `Warning`, or `Critical`
- Alerts can be deactivated by the publisher or admin
- Sequential alert IDs with full on-chain history

---

## Getting Started

### Prerequisites

- Rust (stable) — [install](https://rustup.rs)
- `wasm32-unknown-unknown` target

```bash
rustup target add wasm32-unknown-unknown
```

### Build

```bash
cd contracts
cargo build
```

### Test

```bash
cd contracts
cargo test
```

All 19 tests should pass:

```
running 6 tests  ← verification-registry
running 4 tests  ← community-reporting
running 4 tests  ← staking-reputation
running 5 tests  ← decentralized-alert-feed

test result: ok. 19 passed; 0 failed
```

### Build for Deployment (WASM)

```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
```

WASM artifacts will be in `contracts/target/wasm32-unknown-unknown/release/`.

---

## Risk Scoring Model

Each asset receives a composite risk score weighted across:

| Factor | Weight |
|---|---|
| Issuer concentration | 25% |
| LP ownership | 20% |
| Wallet clustering | 15% |
| Sudden minting | 15% |
| Wash trading | 10% |
| Trustline manipulation | 10% |
| Fake metadata | 5% |

Scores are stored on-chain in `verification-registry` and updated by the risk engine backend.

---

## Full System Architecture

```
Stellar Network / Soroban / DEX
          │
    Blockchain Indexer
    (Horizon + RPC + Events)
          │
  ┌───────┼───────┐
  ▼       ▼       ▼
Risk    ML/AI   Graph
Engine  Models  Engine
  └───────┼───────┘
          ▼
   Scoring Service
          ▼
  REST + GraphQL API
   WebSocket Streams
          ▼
  Frontend Dashboard
```

**Backend services:** Stellar indexer · Soroban event listener · Risk engine · Wallet clustering (Neo4j) · Liquidity monitor · ML anomaly engine · Notification service · API gateway

**Frontend:** Next.js dashboard · Real-time alerts · Holder graphs · Whale heatmaps · Wallet lookup

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart Contracts | Rust + Soroban SDK |
| Backend | NestJS / FastAPI |
| ML | Python (Isolation Forest, XGBoost, GNNs) |
| Graph DB | Neo4j |
| Analytics | ClickHouse |
| Primary DB | PostgreSQL |
| Cache | Redis |
| Streaming | Kafka |
| Frontend | Next.js + Tailwind + Recharts |
| Wallet | Freighter |

---

## Development Roadmap

| Phase | Status | Deliverables |
|---|---|---|
| Phase 1 | 🔨 In Progress | Soroban contracts (50%), Stellar indexer, dashboard MVP |
| Phase 2 | Planned | Liquidity monitoring, whale tracking, alert system |
| Phase 3 | Planned | ML anomaly detection, wallet clustering, community reporting |
| Phase 4 | Planned | DAO governance, reputation staking, browser extension |

---

## Contributing

1. Fork the repo and create a feature branch
2. Make changes inside the relevant workspace (`contracts/`, `backend/`, `frontend/`)
3. Run tests before opening a PR: `cargo test` (contracts), `npm test` (frontend/backend)
4. Open a pull request with a clear description of what changed and why

---

## License

MIT
