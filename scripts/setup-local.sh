#!/bin/bash
###############################################################################
# ++ (PlusPlus) DEX — Local Development Setup
#
# This script:
#   1. Starts the Docker stack (CKB dev chain + 2 Fiber nodes)
#   2. Generates CKB keys for both Fiber nodes
#   3. Funds the Fiber nodes with dev chain CKB
#   4. Connects the two Fiber nodes as peers
#   5. Opens a payment channel between them
#   6. Starts the PlusPlus DEX server
#
# Prerequisites:
#   - Docker and Docker Compose v2 installed
#
# Usage:
#   ./scripts/setup-local.sh          # Full setup
#   ./scripts/setup-local.sh --clean  # Remove all data and start fresh
###############################################################################
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
DEV_DIR="$ROOT_DIR/dev"
COMPOSE_CMD="docker compose -f $ROOT_DIR/docker-compose.local.yml"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log()   { echo -e "${GREEN}[✓]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[✗]${NC} $*"; }
step()  { echo -e "\n${BLUE}═══ $* ═══${NC}"; }

# ---------------------------------------------------------------------------
# Clean mode
# ---------------------------------------------------------------------------
if [[ "${1:-}" == "--clean" ]]; then
    warn "Cleaning all local dev data..."
    cd "$ROOT_DIR"
    $COMPOSE_CMD down -v --remove-orphans 2>/dev/null || true
    rm -rf "$DEV_DIR"
    log "Clean complete. Run without --clean to set up."
    exit 0
fi

# ---------------------------------------------------------------------------
# Step 1: Check prerequisites
# ---------------------------------------------------------------------------
step "Checking prerequisites"

if ! command -v docker &>/dev/null; then
    error "Docker is not installed. Please install Docker first."
    exit 1
fi
log "Docker found: $(docker --version)"

if ! docker compose version &>/dev/null 2>&1; then
    error "Docker Compose v2 is required. Please update Docker."
    exit 1
fi
log "Docker Compose found"

# ---------------------------------------------------------------------------
# Step 2: Create dev directories
# ---------------------------------------------------------------------------
step "Creating dev directories"

mkdir -p "$DEV_DIR/fiber-node-1/ckb"
mkdir -p "$DEV_DIR/fiber-node-2/ckb"
mkdir -p "$DEV_DIR/configs"
log "Created dev directories"

# ---------------------------------------------------------------------------
# Step 3: Start CKB dev chain
# ---------------------------------------------------------------------------
step "Starting CKB dev chain"

cd "$ROOT_DIR"
$COMPOSE_CMD up -d ckb-dev
log "CKB dev chain starting..."

# Wait for CKB to be healthy
echo -n "  Waiting for CKB to be ready"
for i in $(seq 1 30); do
    if docker exec ckb-dev ckb-cli rpc --url http://127.0.0.1:8114 get_tip_block_number &>/dev/null; then
        echo ""
        log "CKB dev chain is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        echo ""
        error "CKB dev chain failed to start. Check logs: $COMPOSE_CMD logs ckb-dev"
        exit 1
    fi
    echo -n "."
    sleep 2
done

# ---------------------------------------------------------------------------
# Step 4: Generate keys for Fiber nodes
# ---------------------------------------------------------------------------
step "Generating CKB keys for Fiber nodes"

# The CKB dev chain comes with a default miner account that has unlimited funds
# We need to export its private key to fund our Fiber nodes

# Get the miner's private key from the CKB dev chain
MINER_KEY=$(docker exec ckb-dev cat /var/lib/ckb/credentials 2>/dev/null | head -1 || echo "")

if [[ -z "$MINER_KEY" ]]; then
    warn "Could not read miner key. Trying alternative method..."
    # The dev chain genesis account key is well-known
    MINER_KEY="d00c06bfd800d273dd70de114651d273641754608513e82d815c674e18c30e1c"
fi

# Generate a new key pair for Node 1 using ckb-cli inside Docker
KEY1_OUTPUT=$(docker exec ckb-dev ckb-cli account new 2>&1 || echo "")
LOCK_ARG1=$(echo "$KEY1_OUTPUT" | grep -oP 'lock-arg: \K[0-9a-fx]+' || echo "")

if [[ -z "$LOCK_ARG1" ]]; then
    warn "Could not generate key via ckb-cli. Using manual key generation..."
    # Generate a deterministic key from the node index
    LOCK_ARG1="0x$(echo -n "fiber-node-1-$(date +%s)" | sha256sum | cut -c1-40)"
fi

# Export the private key
PRIVKEY1=$(docker exec ckb-dev ckb-cli account export --lock-arg "$LOCK_ARG1" --extended-privkey-path /dev/stdout 2>/dev/null | head -1 || echo "")

if [[ -n "$PRIVKEY1" ]]; then
    echo "$PRIVKEY1" | sed 's/^0x//' | tr -d '\n' | head -c 64 > "$DEV_DIR/fiber-node-1/ckb/key"
else
    # Fallback: use a generated key
    warn "Using fallback key generation for Node 1"
    dd if=/dev/urandom bs=32 count=1 2>/dev/null | xxd -p | tr -d '\n' > "$DEV_DIR/fiber-node-1/ckb/key"
fi
chmod 600 "$DEV_DIR/fiber-node-1/ckb/key"
log "Node 1 key generated"

# Generate key for Node 2
KEY2_OUTPUT=$(docker exec ckb-dev ckb-cli account new 2>&1 || echo "")
LOCK_ARG2=$(echo "$KEY2_OUTPUT" | grep -oP 'lock-arg: \K[0-9a-fx]+' || echo "")

if [[ -z "$LOCK_ARG2" ]]; then
    LOCK_ARG2="0x$(echo -n "fiber-node-2-$(date +%s)" | sha256sum | cut -c1-40)"
fi

PRIVKEY2=$(docker exec ckb-dev ckb-cli account export --lock-arg "$LOCK_ARG2" --extended-privkey-path /dev/stdout 2>/dev/null | head -1 || echo "")

if [[ -n "$PRIVKEY2" ]]; then
    echo "$PRIVKEY2" | sed 's/^0x//' | tr -d '\n' | head -c 64 > "$DEV_DIR/fiber-node-2/ckb/key"
else
    warn "Using fallback key generation for Node 2"
    dd if=/dev/urandom bs=32 count=1 2>/dev/null | xxd -p | tr -d '\n' > "$DEV_DIR/fiber-node-2/ckb/key"
fi
chmod 600 "$DEV_DIR/fiber-node-2/ckb/key"
log "Node 2 key generated"

# Get addresses
ADDR1=$(docker exec ckb-dev ckb-cli util address --lock-arg "$LOCK_ARG1" --format ckb2021 2>/dev/null | tail -1 || echo "unknown")
ADDR2=$(docker exec ckb-dev ckb-cli util address --lock-arg "$LOCK_ARG2" --format ckb2021 2>/dev/null | tail -1 || echo "unknown")
log "Node 1 address: $ADDR1"
log "Node 2 address: $ADDR2"

# ---------------------------------------------------------------------------
# Step 5: Fund the Fiber nodes with dev chain CKB
# ---------------------------------------------------------------------------
step "Funding Fiber nodes with dev chain CKB"

# In CKB dev chain, the default miner account has unlimited funds
# Send 100,000 CKB to each node (10,000,000,000,000 shannons)
FUND_AMOUNT=10000000000000

# Fund Node 1
if docker exec ckb-dev ckb-cli --url http://127.0.0.1:8114 send \
    --from-key "$MINER_KEY" \
    --to-address "$ADDR1" \
    --capacity "$FUND_AMOUNT" \
    --fee-rate 1000 &>/dev/null; then
    log "Funded Node 1 with 100,000 CKB"
else
    warn "Could not fund Node 1 automatically. Manual funding may be required."
fi

# Fund Node 2
if docker exec ckb-dev ckb-cli --url http://127.0.0.1:8114 send \
    --from-key "$MINER_KEY" \
    --to-address "$ADDR2" \
    --capacity "$FUND_AMOUNT" \
    --fee-rate 1000 &>/dev/null; then
    log "Funded Node 2 with 100,000 CKB"
else
    warn "Could not fund Node 2 automatically. Manual funding may be required."
fi

# ---------------------------------------------------------------------------
# Step 6: Start Fiber nodes
# ---------------------------------------------------------------------------
step "Starting Fiber nodes"

$COMPOSE_CMD up -d fiber-node-1 fiber-node-2
log "Fiber nodes starting..."

echo -n "  Waiting for Fiber Node 1"
for i in $(seq 1 30); do
    if curl -s -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"node_info","params":[]}' \
        http://127.0.0.1:8227 &>/dev/null; then
        echo ""
        log "Fiber Node 1 is ready (RPC: http://127.0.0.1:8227)"
        break
    fi
    if [ $i -eq 30 ]; then
        echo ""
        warn "Fiber Node 1 may still be starting. Check logs."
    fi
    echo -n "."
    sleep 2
done

echo -n "  Waiting for Fiber Node 2"
for i in $(seq 1 30); do
    if curl -s -X POST -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"node_info","params":[]}' \
        http://127.0.0.1:8237 &>/dev/null; then
        echo ""
        log "Fiber Node 2 is ready (RPC: http://127.0.0.1:8237)"
        break
    fi
    if [ $i -eq 30 ]; then
        echo ""
        warn "Fiber Node 2 may still be starting. Check logs."
    fi
    echo -n "."
    sleep 2
done

# ---------------------------------------------------------------------------
# Step 7: Connect Fiber nodes as peers
# ---------------------------------------------------------------------------
step "Connecting Fiber nodes as peers"

# Get Node 2's pubkey
NODE2_INFO=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"node_info","params":[]}' \
    http://127.0.0.1:8237)
NODE2_PUBKEY=$(echo "$NODE2_INFO" | grep -oP '"pubkey"\s*:\s*"\K[^"]+' || echo "")

if [[ -n "$NODE2_PUBKEY" ]]; then
    log "Node 2 pubkey: $NODE2_PUBKEY"

    # Connect Node 1 to Node 2
    CONNECT_RESULT=$(curl -s -X POST -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\":\"2.0\",
            \"id\":1,
            \"method\":\"connect_peer\",
            \"params\":[{
                \"pubkey\":\"$NODE2_PUBKEY\",
                \"address\":\"/ip4/127.0.0.1/tcp/8238\",
                \"save\":false
            }]
        }" \
        http://127.0.0.1:8227)
    
    if echo "$CONNECT_RESULT" | grep -q "error"; then
        warn "Peer connection may have failed. Check logs."
        warn "Result: $CONNECT_RESULT"
    else
        log "Connected Node 1 → Node 2"
    fi
else
    warn "Could not get Node 2 pubkey. Manual peer connection may be required."
fi

# ---------------------------------------------------------------------------
# Step 8: Open payment channel
# ---------------------------------------------------------------------------
step "Opening payment channel between nodes"

if [[ -n "${NODE2_PUBKEY:-}" ]]; then
    # Open channel from Node 1 to Node 2 with 500 CKB capacity
    CHANNEL_RESULT=$(curl -s -X POST -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\":\"2.0\",
            \"id\":1,
            \"method\":\"open_channel\",
            \"params\":[{
                \"pubkey\":\"$NODE2_PUBKEY\",
                \"funding_amount\":\"0xba43b7400\",
                \"public\":true
            }]
        }" \
        http://127.0.0.1:8227)
    
    if echo "$CHANNEL_RESULT" | grep -q "error"; then
        warn "Channel open request may have failed. Check logs."
        warn "Result: $CHANNEL_RESULT"
    else
        log "Channel open request sent (500 CKB capacity)"
    fi

    # Wait for channel to be ready
    echo -n "  Waiting for channel to be established"
    for i in $(seq 1 60); do
        CHANNELS=$(curl -s -X POST -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"list_channels","params":[{}]}' \
            http://127.0.0.1:8227)
        if echo "$CHANNELS" | grep -q "ChannelReady"; then
            echo ""
            log "Channel is ready!"
            break
        fi
        if [ $i -eq 60 ]; then
            echo ""
            warn "Channel may still be opening. Check status manually."
        fi
        echo -n "."
        sleep 5
    done
else
    warn "Skipping channel setup (no peer connected)"
fi

# ---------------------------------------------------------------------------
# Step 9: Start PlusPlus DEX server
# ---------------------------------------------------------------------------
step "Starting PlusPlus DEX server"

$COMPOSE_CMD up -d --build plusplus
log "PlusPlus server starting..."

echo -n "  Waiting for PlusPlus server"
for i in $(seq 1 30); do
    if curl -s http://127.0.0.1:3000/info &>/dev/null; then
        echo ""
        log "PlusPlus server is ready"
        break
    fi
    if [ $i -eq 30 ]; then
        echo ""
        warn "PlusPlus server may still be building. Check logs."
    fi
    echo -n "."
    sleep 2
done

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
step "Setup Complete!"

echo ""
echo "  ╔═══════════════════════════════════════════════════════════════╗"
echo "  ║              ++ (PlusPlus) DEX — Local Dev Stack             ║"
echo "  ╠═══════════════════════════════════════════════════════════════╣"
echo "  ║                                                             ║"
echo "  ║  Services:                                                  ║"
echo "  ║    • CKB Dev Chain:      http://127.0.0.1:8114              ║"
echo "  ║    • Fiber Node 1:       http://127.0.0.1:8227 (RPC)        ║"
echo "  ║    • Fiber Node 2:       http://127.0.0.1:8237 (RPC)        ║"
echo "  ║    • PlusPlus DEX API:   http://127.0.0.1:3000              ║"
echo "  ║    • PlusPlus Web UI:    http://127.0.0.1:3000/index.html   ║"
echo "  ║    • WebSocket:          ws://127.0.0.1:3000/ws              ║"
echo "  ║                                                             ║"
echo "  ║  Quick commands:                                            ║"
echo "  ║    • View logs:          $COMPOSE_CMD logs -f               ║"
echo "  ║    • Stop all:           $COMPOSE_CMD down                  ║"
echo "  ║    • Clean restart:      ./scripts/setup-local.sh --clean   ║"
echo "  ║                                                             ║"
echo "  ╚═══════════════════════════════════════════════════════════════╝"
echo ""
echo "  Open http://127.0.0.1:3000/index.html to start trading!"
echo ""
