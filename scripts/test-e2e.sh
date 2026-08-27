#!/bin/bash
###############################################################################
# ++ (PlusPlus) DEX — End-to-End Test Script
#
# Tests the full swap flow against running Fiber nodes and PlusPlus server.
#
# Prerequisites:
#   - Local dev stack running: ./scripts/setup-local.sh
#   - curl and jq installed
#
# Usage:
#   ./scripts/test-e2e.sh
###############################################################################
set -euo pipefail

API="http://127.0.0.1:3000"
PASS=0
FAIL=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}✓${NC} $*"; PASS=$((PASS + 1)); }
fail() { echo -e "  ${RED}✗${NC} $*"; FAIL=$((FAIL + 1)); }
step() { echo -e "\n${YELLOW}═══ $* ═══${NC}"; }

# ---------------------------------------------------------------------------
step "1. Server health check"
# ---------------------------------------------------------------------------

INFO=$(curl -s "$API/info")
if echo "$INFO" | grep -q '"success":true'; then
    pass "Server is running"
else
    fail "Server not responding"
    echo "  Response: $INFO"
    exit 1
fi

# ---------------------------------------------------------------------------
step "2. Fiber node health"
# ---------------------------------------------------------------------------

HEALTH=$(curl -s "$API/fiber/health")
if echo "$HEALTH" | grep -q '"reachable":true'; then
    pass "At least one Fiber node reachable"
else
    fail "No Fiber nodes reachable"
    echo "  Response: $HEALTH"
fi

# ---------------------------------------------------------------------------
step "3. Fiber network topology"
# ---------------------------------------------------------------------------

NETWORK=$(curl -s "$API/fiber/network")
if echo "$NETWORK" | grep -q '"success":true'; then
    pass "Network topology fetched"
    NODE1=$(echo "$NETWORK" | jq -r '.data.node1.pubkey // "none"')
    NODE2=$(echo "$NETWORK" | jq -r '.data.node2.pubkey // "none"')
    echo "  Node 1: ${NODE1:0:16}..."
    echo "  Node 2: ${NODE2:0:16}..."
else
    fail "Network topology fetch failed"
fi

# ---------------------------------------------------------------------------
step "4. List Fiber channels"
# ---------------------------------------------------------------------------

CHANNELS=$(curl -s "$API/fiber/channels")
if echo "$CHANNELS" | grep -q '"success":true'; then
    CH_COUNT=$(echo "$CHANNELS" | jq '.data | length')
    pass "Listed $CH_COUNT Fiber channel(s)"
else
    fail "Channel listing failed"
fi

# ---------------------------------------------------------------------------
step "5. Register test asset"
# ---------------------------------------------------------------------------

ASSET=$(curl -s -X POST "$API/assets" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "TestToken",
        "symbol": "TST",
        "issuer_lock": "dfec8003fde3c3ee954bccc2b30189a4ed308ce9",
        "total_supply": 1000000,
        "code_hash": "0x6798ae518401bc21bf9032d93f2f55701ff16cb1a83d58c4f1360922d5575185"
    }')

if echo "$ASSET" | grep -q '"success":true'; then
    ASSET_ID=$(echo "$ASSET" | jq -r '.data.asset_id')
    pass "Asset registered: ${ASSET_ID:0:16}..."
else
    fail "Asset registration failed"
    echo "  Response: $ASSET"
fi

# ---------------------------------------------------------------------------
step "6. Create offer"
# ---------------------------------------------------------------------------

OFFER=$(curl -s -X POST "$API/offers" \
    -H "Content-Type: application/json" \
    -d '{
        "sell_type_code_hash": "0x6798ae518401bc21bf9032d93f2f55701ff16cb1a83d58c4f1360922d5575185",
        "sell_type_args": "0x",
        "sell_amount": 10000,
        "buy_type_code_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "buy_type_args": "0x",
        "buy_amount": 500,
        "seller_lock_hash": "dfec8003fde3c3ee954bccc2b30189a4ed308ce9000000000000000000000000",
        "expiry": 1000000
    }')

if echo "$OFFER" | grep -q '"success":true'; then
    OFFER_ID=$(echo "$OFFER" | jq -r '.data.offer_id')
    pass "Offer created: ${OFFER_ID:0:16}..."
else
    fail "Offer creation failed"
    echo "  Response: $OFFER"
fi

# ---------------------------------------------------------------------------
step "7. List offers"
# ---------------------------------------------------------------------------

OFFERS=$(curl -s "$API/offers?status=Active")
if echo "$OFFERS" | grep -q '"success":true'; then
    OFFER_COUNT=$(echo "$OFFERS" | jq '.data | length')
    pass "Listed $OFFER_COUNT active offer(s)"
else
    fail "Offer listing failed"
fi

# ---------------------------------------------------------------------------
step "8. Execute swap"
# ---------------------------------------------------------------------------

if [ -n "${OFFER_ID:-}" ]; then
    SWAP=$(curl -s -X POST "$API/swaps" \
        -H "Content-Type: application/json" \
        -d "{
            \"offer_id\": \"$OFFER_ID\",
            \"buyer_lock_hash\": \"aabbccdd00000000000000000000000000000000000000000000000000000000\",
            \"amount\": 500
        }")

    if echo "$SWAP" | grep -q '"success":true'; then
        TX_HASH=$(echo "$SWAP" | jq -r '.data.tx_hash')
        pass "Swap executed: ${TX_HASH:0:16}..."
    else
        fail "Swap execution failed"
        echo "  Response: $SWAP"
    fi
else
    fail "Skipping swap (no offer ID)"
fi

# ---------------------------------------------------------------------------
step "9. List swaps"
# ---------------------------------------------------------------------------

SWAPS=$(curl -s "$API/swaps")
if echo "$SWAPS" | grep -q '"success":true'; then
    SWAP_COUNT=$(echo "$SWAPS" | jq '.data | length')
    pass "Listed $SWAP_COUNT swap(s)"
else
    fail "Swap listing failed"
fi

# ---------------------------------------------------------------------------
step "10. Fee estimation"
# ---------------------------------------------------------------------------

FEE=$(curl -s -X POST "$API/fiber/estimate-fee" \
    -H "Content-Type: application/json" \
    -d '{
        "from": "dfec8003fde3c3ee954bccc2b30189a4ed308ce9000000000000000000000000",
        "to": "aabbccdd00000000000000000000000000000000000000000000000000000000",
        "amount": 100000
    }')

if echo "$FEE" | grep -q '"success":true'; then
    pass "Fee estimation returned"
else
    fail "Fee estimation failed"
fi

# ---------------------------------------------------------------------------
step "11. Cancel offer"
# ---------------------------------------------------------------------------

if [ -n "${OFFER_ID:-}" ]; then
    CANCEL=$(curl -s -X DELETE "$API/offers/$OFFER_ID")
    if echo "$CANCEL" | grep -q '"success":true'; then
        pass "Offer cancelled"
    else
        fail "Offer cancellation failed"
    fi
else
    fail "Skipping cancel (no offer ID)"
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

step "Results"
echo ""
TOTAL=$((PASS + FAIL))
echo -e "  ${GREEN}Passed: $PASS${NC} / $TOTAL"
if [ $FAIL -gt 0 ]; then
    echo -e "  ${RED}Failed: $FAIL${NC}"
    exit 1
else
    echo -e "  ${GREEN}All tests passed!${NC}"
    exit 0
fi
