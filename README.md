# ++ — RGB++ DEX

Trustless peer-to-peer exchange for RGB++ assets using Fiber channel atomic swaps on Nervos CKB.

## Overview

++ enables BTC holders to swap into RGB++ tokens and vice versa through a fully on-chain, trustless mechanism. Every trade is an atomic swap enforced by a CKB covenant smart contract — no intermediaries, no custody, no counterparty risk.

| | |
|---|---|
| **Network** | [CKB Testnet (Pudge)](https://network.ckbapp.dev/) |
| **Explorer** | [CKB Testnet Explorer](https://testnet.explorer.nervos.org/) |
| **RPC** | `https://testnet.ckbapp.dev` |
| **WebSocket** | `wss://testnet.ckbapp.dev/ws` |

## Deployment

### Swap Covenant Contract

| Field | Value |
|-------|-------|
| **Contract Type** | RISC-V ELF (covenant) |
| **Binary Size** | ~38 KB |
| **Code Hash** | `0x6798ae518401bc21bf9032d93f2f55701ff16cb1a83d58c4f1360922d5575185` |
| **Hash Type** | `type` |
| **Deployment Config** | [`deploy/deploy.toml`](deploy/deploy.toml) |
| **Deployer Lock Args** | `0xdfec8003fde3c3ee954bccc2b30189a4ed308ce9` |
| **Multisig Address** | `ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsqwlajqq8l0rc0hf2j7vc2esrzdya5cge6gts8yrw` |

**Deployment Transactions:**
- [Cell TX](https://testnet.explorer.nervos.org/transaction/0x416746b820fb3ed729c821537d8fa0739aa0bd448004e256a54194bbb72cdb0c)
- [Dep Group TX](https://testnet.explorer.nervos.org/transaction/0x9c3d3fe26b2009877174ae9a0d22a6847448bc2aa528065217cdc962ffc1b54e)
- [Code Hash on Explorer](https://testnet.explorer.nervos.org/search?type=code&id=0x6798ae518401bc21bf9032d93f2f55701ff16cb1a83d58c4f1360922d5575185)
- [Deployer Address](https://testnet.explorer.nervos.org/address/ckt1qzda0cr08m85hc8jlnfp3zer7xulejywt49kt2rr0vthywaa50xwsq06wdgfhg23fdf35pgwf4rvpgv4u7upk9cq7e644)

### Deploy to Testnet

```bash
# 1. Build the covenant
cd contracts/swap-covenant
RUSTFLAGS="-C target-feature=+zba,+zbb,+zbc,+zbs -C passes=lower-atomic" \
TARGET_CC="clang" TARGET_AR="llvm-ar-18" \
cargo build --target=riscv64imac-unknown-none-elf --release

# 2. Calculate code hash
ckb-cli --url https://testnet.ckbapp.dev \
  util blake2b \
  --binary-path target/riscv64imac-unknown-none-elf/release/swap-covenant

# 3. Fund the deployer wallet (min 10,000 CKB)
# Faucet: https://faucet.nervos.org/
# Address: ckt1qyqrdse9437xnz57vv6z73yz839cc6zytzzv...

# 4. Generate deployment transactions
ckb-cli --url https://testnet.ckbapp.dev deploy gen-txs \
  --from-address <YOUR_TESTNET_ADDRESS> \
  --fee-rate 1000 \
  --deployment-config deploy/deploy.toml \
  --info-file deploy/txs/deploy-info.json \
  --migration-dir deploy/migrations \
  --sign-now

# 5. Send deployment transactions
ckb-cli --url https://testnet.ckbapp.dev deploy send \
  --info-file deploy/txs/deploy-info.json \
  --private-key <YOUR_PRIVATE_KEY>
```

### Running Locally

```bash
# Start Fiber nodes (Docker)
docker compose -f docker-compose.local.yml up -d

# Start the DEX server
cargo run -p plusplus-server

# Open the web UI
open http://localhost:3000/index.html
```

## Architecture

```
PlusPlus/
├── contracts/
│   └── swap-covenant/        # On-chain covenant (RISC-V ELF)
├── crates/
│   ├── offer-protocol/        # Offer creation, signing, verification
│   ├── swap-executor/         # Transaction building and execution
│   ├── channel-manager/       # Fiber channel lifecycle
│   ├── router/                # Dijkstra multi-hop routing
│   ├── settlement/            # Settlement proofs, disputes, force-close
│   ├── incentives/            # Fee distribution, reputation system
│   ├── indexer/               # SQLite DB, REST API types
│   ├── fiber-client/          # Fiber node JSON-RPC client
│   ├── plusplus-cli/          # CLI tool
│   └── plusplus-server/       # REST API + WebSocket server
├── sdk/typescript/            # TypeScript SDK
├── web/index.html             # Web interface
├── docs/                      # Documentation
├── scripts/                   # Deployment scripts
└── deploy/                    # Contract deployment config
```

## API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/info` | GET | Server info and stats |
| `/offers` | GET | List offers (query: `?asset=&status=&limit=&offset=`) |
| `/offers` | POST | Create offer |
| `/offers/{id}` | DELETE | Cancel offer |
| `/swaps` | GET | List swaps |
| `/swaps` | POST | Execute swap |
| `/assets` | GET | List assets |
| `/assets` | POST | Register asset |
| `/route` | POST | Find route |
| `/ws` | WebSocket | Real-time events |

## Tech Stack

- **Blockchain:** Nervos CKB (RISC-V smart contracts)
- **Language:** Rust
- **Off-Chain:** Fiber Network (payment channels)
- **Database:** SQLite (indexer)
- **Server:** Axum + Tokio
- **Frontend:** Vanilla HTML/CSS/JS
- **SDK:** TypeScript

## License

MIT



https://tranquil-insight-production-005a.up.railway.app/info 2>&1