#!/bin/bash
# ++ DEX — Swap Covenant Testnet Deployment
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACT_BIN="$ROOT_DIR/contracts/swap-covenant/target/riscv64imac-unknown-none-elf/release/swap-covenant"
CKB_RPC="https://testnet.ckb.dev/rpc"
WALLET_KEY="$ROOT_DIR/deploy/testnet-key.hex"

export PATH="$HOME/.local/bin:$PATH"

echo "=== ++ DEX Testnet Deployment ==="
echo "RPC: $CKB_RPC"
echo ""

# Step 1: Verify contract binary exists
echo "[1/5] Verifying contract binary..."
if [ ! -f "$CONTRACT_BIN" ]; then
    echo "  ✗ Contract binary not found. Run build first."
    exit 1
fi
SIZE=$(stat -c%s "$CONTRACT_BIN")
echo "  ✓ Contract binary: $SIZE bytes"
echo ""

# Step 2: Calculate code hash
echo "[2/5] Calculating code hash..."
CODE_HASH=$(ckb-cli --url "$CKB_RPC" --output-format json util blake2b --binary-path "$CONTRACT_BIN" 2>&1 | tr -d '"')
echo "  Code hash: $CODE_HASH"
echo ""

# Step 3: Get deploy capacity estimate
echo "[3/5] Estimating deployment cost..."
BYTES=$((SIZE / 1024 + 1))
DEPLOY_CAPACITY=$((BYTES * 100000000 + 20000000000))  # capacity in shannons
echo "  Estimated capacity: $((DEPLOY_CAPACITY / 100000000)) CKB"
echo ""

# Step 4: Connect to testnet
echo "[4/5] Verifying testnet connection..."
BLOCK=$(ckb-cli --url "$CKB_RPC" --output-format json rpc get_tip_block_number 2>&1 | tr -d '"')
echo "  ✓ Connected to testnet at block $BLOCK"
echo ""

# Step 5: Print deployment instructions
echo "[5/5] Deployment summary"
echo ""
echo "  Contract binary: $CONTRACT_BIN"
echo "  Contract size:   $SIZE bytes"
echo "  Code hash:       $CODE_HASH"
echo "  Network:         CKB Testnet (Pudge)"
echo ""
echo "  === DEPLOYMENT STEPS ==="
echo ""
echo "  1. Get testnet CKB from faucet:"
echo "     https://faucet.nervos.org/"
echo "     Wallet address: $(ckb-cli util key-info --privkey-path "$WALLET_KEY" --output-format json 2>&1 | grep -o '"testnet": "[^"]*"' | head -1)"
echo ""
echo "  2. Deploy contract using ckb-cli:"
echo "     (Requires funded wallet with $((DEPLOY_CAPACITY / 100000000))+ CKB)"
echo ""
echo "  3. Start the ++ server:"
echo "     cargo run -p plusplus-server"
echo ""
echo "  4. Open web UI:"
echo "     http://localhost:3000/index.html"
echo ""
echo "=== Ready for deployment ==="
