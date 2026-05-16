# Stellar Rug-Pull Detector

> A real-time analytics and risk intelligence platform built on the Stellar ecosystem — scanning tokens, issuers, liquidity pools, and DEX activity to detect scams, malicious tokenomics, liquidity drains, fake assets, wash trading, and centralized supply concentration **before** users interact with risky assets.

---

## What Is This?

Think of it as **TokenSniffer + RugCheck + DEXTools — built natively for Stellar**.

The Stellar ecosystem has grown significantly with Soroban smart contracts, AMMs, and cross-chain asset issuance — but security and analytics tooling remain far behind Ethereum and Solana. This project fills that gap by providing a deep on-chain risk intelligence layer for Stellar DeFi.

**Monitors in real-time:**
- Native Stellar DEX orderbooks
- Soroban AMMs (Soroswap, Aquarius, Phoenix)
- Token issuers and trustline behavior
- Liquidity movements and ownership
- Wallet concentration and insider clusters
- Sudden minting/burning events
- Suspicious transaction graphs
- Fake verified assets and honeypot behavior

---

## Core Features

### 🔍 Token Risk Scanner
Analyzes supply distribution, issuer ownership, mint/freeze/burn permissions, whale concentration, and locked liquidity. Outputs a clear safety rating:

| Rating | Description |
|---|---|
| ✅ Safe | No significant risk indicators |
| ⚠️ Moderate Risk | Some flags, proceed with caution |
| 🔴 High Risk | Multiple red flags detected |
| ☠️ Potential Rug | Active rug indicators present |

### 💧 Liquidity Monitoring Engine
Tracks LP creation, withdrawals, lock duration, and sudden drains. Detects soft rugs, hard rugs, and exit liquidity setups before they execute.

### 🐋 Wallet Concentration Analysis
Flags wallets holding >50% of supply, connected insider clusters, Sybil wallets, and developer wallet dumping patterns using graph analysis.

### 📊 Wash Trading Detection
Identifies repeated buy/sell loops, same-wallet routing, volume inflation, and circular trading graphs to surface fake volume.

### 🎭 Fake Asset Detection
Detects copycat asset codes (fake USDC/USDT), missing `stellar.toml`, suspicious issuer domains, and fake anchors — critical given Stellar's trustline-based asset model.

### 📈 Trustline Analytics
Monitors rapid trustline spikes, bot-created trustlines, and mass account creation used to fake adoption metrics.

### 🚨 Real-Time Alert System
Pushes alerts when liquidity drops suddenly, developers mint supply, whales dump, or suspicious contracts appear.

**Channels:** Telegram · Discord · Email · Push Notifications · WebSocket Feeds

### 🖥️ Risk Dashboard
Visual interface showing risk scores, holder graphs, liquidity charts, wallet heatmaps, whale activity, and rug probability.

---

## Architecture

```
┌─────────────────────┐
│   Stellar Network   │
│  Soroban Contracts  │
│    Stellar DEX      │
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│  Blockchain Indexer │
│   Horizon Scanner   │
│   Event Streamer    │
└──────────┬──────────┘
           │
  ┌────────┼────────┐
  ▼        ▼        ▼
Risk     ML/AI    Graph
Engine  Detect   Engine
  └────────┼────────┘
           ▼
    Scoring Service
    Rug Probability
           ▼
   REST + GraphQL API
    WebSocket Streams
           ▼
   Frontend Dashboard
   Alerts + Analytics
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| Blockchain | Stellar + Soroban |
| Smart Contracts | Rust |
| Backend | NestJS / FastAPI |
| ML / AI | Python (Isolation Forest, XGBoost, GNNs) |
| Graph Analysis | Neo4j |
| Analytics DB | ClickHouse |
| Primary DB | PostgreSQL |
| Cache | Redis |
| Streaming | Kafka / Redpanda |
| Frontend | Next.js + Tailwind + Recharts |
| Wallet | Freighter |
| Infrastructure | Kubernetes + Terraform |

---

## Smart Contracts (Soroban)

| Contract | Purpose |
|---|---|
| `verification-registry` | Stores verified assets, approved issuers, reputation scores |
| `community-reporting` | Scam reports, community voting, DAO moderation |
| `staking-reputation` | Users stake tokens to validate assets, earn rewards, penalize false reports |
| `decentralized-alert-feed` | On-chain alert publishing for trustless risk data |

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

---

## Project Structure

```
stellar-rugpull-detector/
├── apps/
│   ├── frontend-dashboard/       # Next.js risk dashboard
│   ├── admin-panel/              # Internal moderation tools
│   ├── mobile-alert-app/         # React Native push alerts
│   └── browser-extension/        # Freighter/StellarX warning overlay
│
├── services/
│   ├── stellar-indexer/          # Horizon + RPC event ingestion
│   ├── soroban-event-listener/   # Soroban contract event streaming
│   ├── risk-engine/              # Core heuristic scoring
│   ├── wallet-clustering-engine/ # Neo4j graph analysis
│   ├── liquidity-monitor/        # LP tracking and drain detection
│   ├── ml-anomaly-engine/        # Python ML models
│   ├── notification-service/     # Telegram, Discord, email alerts
│   ├── scoring-service/          # Aggregated rug probability
│   ├── asset-metadata-parser/    # stellar.toml + issuer validation
│   └── api-gateway/              # REST + GraphQL + WebSocket API
│
├── contracts/                    # Soroban smart contracts (Rust)
├── infrastructure/               # Docker, Kubernetes, Terraform, Nginx
├── databases/                    # Schema and migrations
├── shared/                       # SDK, UI components, types, utils
├── docs/
├── scripts/
└── tests/
```

---

## API Reference

**REST**
```
GET  /tokens
GET  /token/:id
GET  /wallet/:address
GET  /alerts
GET  /risk-score/:asset
GET  /liquidity/:pool
```

**WebSocket Streams**
```
/ws/alerts
/ws/liquidity
/ws/trades
/ws/risk-updates
```

---

## Dashboard Pages

**Public**
```
/                  Landing + live risk feed
/explore           Browse all tracked assets
/tokens            Token list with risk scores
/token/[asset]     Full token risk detail page
/issuers           Issuer reputation profiles
/wallets           Wallet risk lookup
/alerts            Live alert stream
/leaderboard       Safest / riskiest assets
/analytics         Ecosystem-wide analytics
```

**Admin**
```
/admin             Overview
/admin/reports     Community scam reports
/admin/risk-rules  Heuristic rule management
/admin/moderation  DAO voting queue
```

---

## Development Roadmap

| Phase | Deliverables |
|---|---|
| **Phase 1** | Stellar indexer, basic risk engine, dashboard MVP |
| **Phase 2** | Liquidity monitoring, whale tracking, alert system |
| **Phase 3** | ML anomaly detection, wallet clustering, community reporting |
| **Phase 4** | DAO governance, reputation staking, browser extension |

### Suggested MVP (Start Here)
1. Stellar transaction indexer (Horizon + Soroban events)
2. Token risk scoring engine
3. Liquidity monitoring
4. Wallet concentration analysis
5. Web dashboard
6. Alert system

---

## Advanced Features

- **AI Rug Probability** — Predict rugs before they happen using historical pattern models
- **Social Sentiment Analysis** — Telegram, X/Twitter, Discord signal aggregation
- **Cross-DEX Monitoring** — Unified view across Soroswap, Aquarius, and Phoenix
- **Browser Extension** — Warns users before swaps, highlights risky assets inline in Freighter and StellarX
- **Mobile App** — Push alerts for whale moves, portfolio risk, and scam warnings

---

## Revenue Model

| Tier | Description |
|---|---|
| **Free** | Basic risk scans, public dashboard |
| **API Subscription** | Risk feeds, wallet analytics, DEX intelligence |
| **Institutional** | Full analytics suite for exchanges, funds, compliance teams |
| **Token Audits** | Paid on-chain audits for project launches |

---

## Security Considerations

- Rate limiting on all public endpoints
- Multi-node validation to prevent oracle spoofing
- Reputation systems to penalize fake reports
- Signed data feeds for tamper-proof risk scores
- Sybil-resistant voting via staking

---

## Why This Matters

Stellar's DeFi ecosystem is expanding rapidly with Soroban, AMMs, and cross-chain asset issuance — but security tooling is still nascent compared to Ethereum or Solana. This project aims to become:

- The **primary security layer** for Stellar DeFi
- A **risk oracle** for wallets and DEXs
- An **institutional analytics platform** for compliance and due diligence
- A **community-owned** scam detection network via DAO governance

---


