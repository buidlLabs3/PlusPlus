# Local Development Guide

This guide walks you through setting up and running the ++ (PlusPlus) DEX locally for development and testing.

## Architecture

The local development stack consists of:

```
┌─────────────────────────────────────────────────────────────┐
│  Docker Compose Stack                                       │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  CKB Dev     │  │  Fiber Node 1│  │  Fiber Node 2    │ │
│  │  Chain       │  │  (Maker)     │  │  (Taker)         │ │
│  │  :8114       │  │  :8227 RPC   │  │  :8237 RPC       │ │
│  └──────────────┘  └──────────────┘  └──────────────────┘ │
│         │               │                    │              │
│         └───────────────┴────────────────────┘              │
│                     │                                       │
│         ┌───────────┴───────────┐                           │
│         │   PlusPlus Server     │                           │
│         │   :3000 REST + WS     │                           │
│         └───────────────────────┘                           │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
    ┌─────────┐
    │ Browser  │
    │ :3000    │
    └─────────┘
```

### Components

| Component | Port | Description |
|-----------|------|-------------|
| CKB Dev Chain | 8114 | Local CKB blockchain (no mining needed) |
| Fiber Node 1 | 8227 | Fiber Network Node #1 (maker/sender) |
| Fiber Node 2 | 8237 | Fiber Network Node #2 (taker/receiver) |
| PlusPlus Server | 3000 | DEX REST API + WebSocket |

## Quick Start

### Prerequisites

- **Docker** v20.10+ and **Docker Compose** v2
- **ckb-cli** (optional, for key management): `cargo install ckb-cli --features=cka --locked`

### Setup

```bash
# Clone and enter the repository
git clone <repo-url> && cd PlusPlus

# Run the setup script
./scripts/setup-local.sh

# That's it! The script will:
# 1. Generate CKB keys for both Fiber nodes
# 2. Start the Docker stack
# 3. Fund the nodes with dev chain CKB
# 4. Connect the nodes as peers
# 5. Open a payment channel between them
# 6. Start the PlusPlus DEX server
```

### Access the DEX

- **Web UI**: http://127.0.0.1:3000/index.html
- **REST API**: http://127.0.0.1:3000
- **WebSocket**: ws://127.0.0.1:3000/ws

## Manual Setup

If you prefer to set up components individually:

### 1. Start CKB Dev Chain

```bash
docker compose -f docker-compose.local.yml up -d ckb-dev
```

### 2. Generate Keys

```bash
# Create directories
mkdir -p dev/fiber-node-1/ckb dev/fiber-node-2/ckb

# Generate key for Node 1
ckb-cli --url http://127.0.0.1:8114 account new
# Note the lock-arg from the output

# Export private key
ckb-cli account export --lock-arg <lock-arg> --extended-privkey-path /tmp/key1
head -n 1 /tmp/key1 | sed 's/^0x//' > dev/fiber-node-1/ckb/key

# Repeat for Node 2...
```

### 3. Fund the Nodes

```bash
# Send CKB to each node's address
ckb-cli --url http://127.0.0.1:8114 send \
  --from-key <miner-key> \
  --to-address <node-address> \
  --capacity 1000000000000  # 10,000 CKB
```

### 4. Start Fiber Nodes

```bash
docker compose -f docker-compose.local.yml up -d fiber-node-1 fiber-node-2
```

### 5. Connect Peers

```bash
# Get Node 2's pubkey
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"node_info","params":[]}' \
  http://127.0.0.1:8237 | grep pubkey

# Connect from Node 1
curl -s -X POST -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","id":1,
    "method":"connect_peer",
    "params":[{"pubkey":"<node2-pubkey>","address":"/ip4/127.0.0.1/tcp/8238","save":false}]
  }' http://127.0.0.1:8227
```

### 6. Open Channel

```bash
curl -s -X POST -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","id":1,
    "method":"open_channel",
    "params":[{"pubkey":"<node2-pubkey>","funding_amount":"0xba43b7400","public":true}]
  }' http://127.0.0.1:8227
```

### 7. Start PlusPlus Server

```bash
docker compose -f docker-compose.local.yml up -d plusplus
```

## Testing the Swap Flow

### 1. Create an Offer (via REST API)

```bash
curl -X POST http://127.0.0.1:3000/offers \
  -H "Content-Type: application/json" \
  -d '{
    "sell_type_code_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "sell_type_args": "0x",
    "sell_amount": 10000,
    "buy_type_code_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "buy_type_args": "0x",
    "buy_amount": 500,
    "seller_lock_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "expiry": 100000
  }'
```

### 2. List Offers

```bash
curl http://127.0.0.1:3000/offers
```

### 3. Accept an Offer

```bash
curl -X POST http://127.0.0.1:3000/swaps/<offer-id>/accept \
  -H "Content-Type: application/json" \
  -d '{
    "buyer_lock_hash": "0x0000000000000000000000000000000000000000000000000000000000000000"
  }'
```

### 4. Subscribe to WebSocket Events

```javascript
const ws = new WebSocket('ws://127.0.0.1:3000/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data.event_type, data.data);
};

// Send subscription
ws.send(JSON.stringify({
  type: 'subscribe',
  event_types: ['offer.created', 'swap.executed']
}));
```

## Useful Commands

### View Logs

```bash
# All services
docker compose -f docker-compose.local.yml logs -f

# Specific service
docker compose -f docker-compose.local.yml logs -f plusplus
docker compose -f docker-compose.local.yml logs -f fiber-node-1
```

### Interact with Fiber Nodes

```bash
# Get node info
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"node_info","params":[]}' \
  http://127.0.0.1:8227

# List channels
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"list_channels","params":[{}]}' \
  http://127.0.0.1:8227

# List peers
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"list_peers","params":[{}]}' \
  http://127.0.0.1:8227
```

### Stop Services

```bash
# Stop all
docker compose -f docker-compose.local.yml down

# Stop and remove volumes (full reset)
docker compose -f docker-compose.local.yml down -v
```

### Clean Restart

```bash
./scripts/setup-local.sh --clean
```

## Development Workflow

### Making Code Changes

1. Edit source files in `crates/` or `contracts/`
2. The PlusPlus server will need to be rebuilt:
   ```bash
   docker compose -f docker-compose.local.yml up -d --build plusplus
   ```

### Running Tests

```bash
# Run all workspace tests
cargo test --workspace

# Run specific crate tests
cargo test -p router
cargo test -p fiber-client
```

### Building the Contract

The swap covenant contract is compiled separately for RISC-V:

```bash
cd contracts/swap-covenant
RUSTFLAGS="-C target-feature=+zba,+zbb,+zbc,+zbs -C passes=lower-atomic" \
TARGET_CC="clang" TARGET_AR="llvm-ar-18" \
cargo build --target=riscv64imac-unknown-none-elf --release
```

## Troubleshooting

### "Connection refused" errors

- Ensure Docker is running
- Check if containers are healthy: `docker compose -f docker-compose.local.yml ps`
- Wait a few seconds for services to start

### "Circuit breaker is open" errors

- The Fiber node may be down or unresponsive
- Check logs: `docker compose -f docker-compose.local.yml logs fiber-node-1`
- Restart the node: `docker compose -f docker-compose.local.yml restart fiber-node-1`

### "Channel not ready" errors

- Channel opening takes time (needs on-chain confirmation)
- Wait for the channel state to become `ChannelReady`
- Check channel status: `curl -s -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"list_channels","params":[{}]}' http://127.0.0.1:8227`

### "Insufficient capacity" errors

- The channel may not have enough balance
- Check balances: `list_channels` RPC call
- Consider opening a new channel with higher capacity

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `FIBER_NODE_URL` | `http://localhost:8227` | Fiber node RPC URL |
| `CKB_RPC_URL` | `http://localhost:8114` | CKB RPC URL |
| `SERVER_PORT` | `3000` | PlusPlus server port |
| `SERVER_HOST` | `0.0.0.0` | PlusPlus server bind address |

## Further Reading

- [Fiber Network Documentation](https://www.fiber.world/docs)
- [CKB Developer Guide](https://docs.nervos.org)
- [RGB++ Protocol](https://github.com/nervosnetwork/rgbpp)
- [PlusPlus API Reference](./API.md)
