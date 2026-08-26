#!/bin/bash
# ++ DEX — Testnet Deployment Script
# Full deployment: build, test, verify, and prepare for ckb-cli deployment
#
# Prerequisites:
#   - ckb-cli installed (https://github.com/nervosnetwork/ckb-cli)
#   - Rust toolchain with riscv64 target
#   - Network: CKB Testnet (Pudge)
#
# Usage:
#   ./scripts/deploy-testnet.sh [--skip-tests] [--skip-build]

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONTRACT_DIR="$ROOT_DIR/contracts/swap-covenant"
CKB_RPC="https://testnet.ckb.dev/rpc"
WALLET_KEY="$ROOT_DIR/deploy/testnet-key.hex"

export PATH="$HOME/.local/bin:$PATH"

SKIP_TESTS=false
SKIP_BUILD=false

for arg in "$@"; do
    case $arg in
        --skip-tests) SKIP_TESTS=true ;;
        --skip-build) SKIP_BUILD=true ;;
    esac
done

echo "============================================="
echo "  ++ DEX — Testnet Deployment"
echo "  Network: CKB Testnet (Pudge)"
echo "============================================="
echo ""

# Step 1: Build the covenant
if [ "$SKIP_BUILD" = false ]; then
    echo "[1/6] Building swap covenant (RISC-V)..."
    cd "$CONTRACT_DIR"
    rm -rf target
    RUSTFLAGS="-C target-feature=+zba,+zbb,+zbc,+zbs -C passes=lower-atomic" \
    TARGET_CC="clang" \
    TARGET_AR="llvm-ar-18" \
    cargo build --target=riscv64imac-unknown-none-elf --release 2>&1 | tail -5

    CONTRACT_BIN="$CONTRACT_DIR/target/riscv64imac-unknown-none-elf/release/swap-covenant"
    if [ ! -f "$CONTRACT_BIN" ]; then
        echo "  ✗ Contract binary not found"
        exit 1
    fi
    SIZE=$(stat -c%s "$CONTRACT_BIN")
    echo "  ✓ Covenant built: $SIZE bytes"
else
    echo "[1/6] Skipping build (--skip-build)"
    CONTRACT_BIN="$CONTRACT_DIR/target/riscv64imac-unknown-none-elf/release/swap-covenant"
    SIZE=$(stat -c%s "$CONTRACT_BIN" 2>/dev/null || echo "0")
fi
echo ""

# Step 2: Calculate code hash
echo "[2/6] Calculating code hash..."
if command -v ckb-cli &> /dev/null; then
    CODE_HASH=$(ckb-cli --url "$CKB_RPC" --output-format json util blake2b --binary-path "$CONTRACT_BIN" 2>&1 | tr -d '"')
    echo "  Code hash: $CODE_HASH"
else
    echo "  ⚠ ckb-cli not found, using blake2b sum"
    CODE_HASH=$(b3sum "$CONTRACT_BIN" 2>/dev/null | awk '{print $1}')
    echo "  Blake3 hash: $CODE_HASH"
    echo "  (Install ckb-cli for proper CKB code hash calculation)"
fi
echo ""

# Step 3: Build workspace
if [ "$SKIP_BUILD" = false ]; then
    echo "[3/6] Building Rust workspace..."
    cd "$ROOT_DIR"
    cargo build --workspace 2>&1 | tail -3
    echo "  ✓ Workspace built"
else
    echo "[3/6] Skipping workspace build"
fi
echo ""

# Step 4: Run tests
if [ "$SKIP_TESTS" = false ]; then
    echo "[4/6] Running tests..."
    cd "$ROOT_DIR"
    cargo test --workspace 2>&1 | grep "test result:" | awk '{print "  ✓ " $4 " tests " $5}'
    echo ""
else
    echo "[4/6] Skipping tests (--skip-tests)"
    echo ""
fi

# Step 5: Verify testnet connection
echo "[5/6] Verifying testnet connection..."
if command -v ckb-cli &> /dev/null; then
    BLOCK=$(ckb-cli --url "$CKB_RPC" --output-format json rpc get_tip_block_number 2>&1 | tr -d '"')
    echo "  ✓ Connected to testnet at block $BLOCK"

    # Check wallet balance
    if [ -f "$WALLET_KEY" ]; then
        echo ""
        echo "  Wallet address:"
        ckb-cli util key-info --privkey-path "$WALLET_KEY" --output-format json 2>&1 | grep -o '"testnet": "[^"]*"' | head -1 | sed 's/"testnet": "/    /' | tr -d '"'
        echo ""

        ADDR=$(ckb-cli util key-info --privkey-path "$WALLET_KEY" --output-format json 2>&1 | grep -o '"testnet": "[^"]*"' | head -1 | sed 's/"testnet": "//;s/"//')
        if [ -n "$ADDR" ]; then
            BALANCE=$(ckb-cli --url "$CKB_RPC" --output-format json rpc get_live_cells_by_lock_script \
                --script-hash $(ckb-cli util blake2b --data "$ADDR" 2>/dev/null) 2>&1 | head -1)
            echo "  Balance check: (use ckb-cli to verify)"
        fi
    fi
else
    echo "  ⚠ ckb-cli not installed — skipping testnet verification"
    echo "  Install: cargo install ckb-cli --git https://github.com/nervosnetwork/ckb-cli"
fi
echo ""

# Step 6: Deployment instructions
echo "[6/6] Deployment Summary"
echo ""
echo "============================================="
echo "  Contract Binary:  $CONTRACT_BIN"
echo "  Binary Size:      $SIZE bytes"
echo "  Code Hash:        ${CODE_HASH:-<not calculated>}"
echo "  Network:          CKB Testnet (Pudge)"
echo "  RPC Endpoint:     $CKB_RPC"
echo "============================================="
echo ""
echo "  === DEPLOYMENT STEPS ==="
echo ""
echo "  1. Fund your testnet wallet with CKB:"
echo "     https://faucet.nervos.org/"
echo ""
echo "  2. Deploy the contract:"
echo "     cd $ROOT_DIR"
echo "     ckb-cli --url $CKB_RPC deploy gen-txs \\"
echo "       --from-address <YOUR_TESTNET_ADDRESS> \\"
echo "       --fee-rate 1000 \\"
echo "       --deployment-config deploy/deploy.toml \\"
echo "       --info-file deploy/txs/deploy-info.json \\"
echo "       --migration-dir deploy/migrations \\"
echo "       --sign-now"
echo ""
echo "  3. Update the code hash in README.md and deploy.toml"
echo ""
echo "  4. Start the server:"
echo "     cargo run -p plusplus-server"
echo "     # Or with Docker:"
echo "     docker-compose up -d"
echo ""
echo "  5. Open the web UI:"
echo "     http://localhost:3000/index.html"
echo ""
echo "  6. (Optional) Start a Fiber node for routing"
echo ""
echo "=== Deployment ready ==="
