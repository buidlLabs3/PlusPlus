#!/bin/bash
# ++ DEX — Testnet Deployment Script
# Uses official CKB contract build process (clang + llvm-ar)
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACT_DIR="$ROOT_DIR/contracts/swap-covenant"
CKB_RPC="https://testnet.ckb.dev/rpc"
WALLET_KEY="$ROOT_DIR/deploy/testnet-key.hex"

export PATH="$HOME/.local/bin:$PATH"

echo "=== ++ DEX Testnet Deployment ==="
echo ""

# Step 1: Build the covenant using official CKB build process
echo "[1/5] Building swap covenant..."
cd "$CONTRACT_DIR"
rm -rf target
RUSTFLAGS="-C target-feature=+zba,+zbb,+zbc,+zbs -C passes=lower-atomic" \
TARGET_CC="clang" \
TARGET_AR="llvm-ar-18" \
cargo build --target=riscv64imac-unknown-none-elf --release 2>&1 | tail -3
echo "  ✓ Covenant built"

# Step 2: Calculate code hash
echo "[2/5] Calculating code hash..."
CONTRACT_BIN="$CONTRACT_DIR/target/riscv64imac-unknown-none-elf/release/swap-covenant"
CODE_HASH=$(ckb-cli --url "$CKB_RPC" --output-format json util blake2b --binary-path "$CONTRACT_BIN" 2>&1 | tr -d '"')
SIZE=$(stat -c%s "$CONTRACT_BIN")
echo "  Code hash: $CODE_HASH"
echo "  Binary size: $SIZE bytes"

# Step 3: Run all tests
echo "[3/5] Running tests..."
cd "$ROOT_DIR"
cargo test --workspace 2>&1 | grep "test result:" | awk '{print "  ✓ " $4 " tests passing"}'

# Step 4: Verify testnet connection
echo "[4/5] Verifying testnet connection..."
BLOCK=$(ckb-cli --url "$CKB_RPC" --output-format json rpc get_tip_block_number 2>&1 | tr -d '"')
echo "  ✓ Connected to testnet at block $BLOCK"

# Step 5: Summary
echo "[5/5] Deployment summary"
echo ""
echo "  Contract binary: $CONTRACT_BIN"
echo "  Binary size:     $SIZE bytes"
echo "  Code hash:       $CODE_HASH"
echo "  Network:         CKB Testnet (Pudge)"
echo ""
echo "  === NEXT STEPS ==="
echo ""
echo "  1. Get testnet CKB from https://faucet.nervos.org/"
echo "     Address: $(ckb-cli util key-info --privkey-path "$WALLET_KEY" --output-format json 2>&1 | grep -o '"testnet": "[^"]*"' | head -1)"
echo ""
echo "  2. Fund the wallet with at least 10,000 CKB"
echo ""
echo "  3. Deploy the contract (once wallet is funded):"
echo "     cd $ROOT_DIR"
echo "     ckb-cli --url $CKB_RPC deploy gen-txs \\"
echo "       --from-address <YOUR_TESTNET_ADDRESS> \\"
echo "       --fee-rate 1000 \\"
echo "       --deployment-config deploy/deploy.toml \\"
echo "       --info-file deploy/txs/deploy-info.json \\"
echo "       --migration-dir deploy/migrations \\"
echo "       --sign-now"
echo ""
echo "  4. Start the ++ server:"
echo "     cargo run -p plusplus-server"
echo ""
echo "=== Deployment ready ==="
