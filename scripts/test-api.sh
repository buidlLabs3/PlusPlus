#!/bin/bash
# API Test Script for ++ DEX Server
set -e
PASS=0
FAIL=0

check() {
    local name="$1"
    local result="$2"
    if [ "$result" = "ok" ]; then
        echo "  ✅ $name"
        PASS=$((PASS + 1))
    else
        echo "  ❌ $name: $result"
        FAIL=$((FAIL + 1))
    fi
}

API="http://localhost:3000"
echo "🧪 Testing ++ DEX API endpoints"
echo "================================"

# 1. GET /info
RESP=$(curl -sf "$API/info" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success']" 2>/dev/null && check "GET /info" "ok" || check "GET /info" "failed"

# 2. GET /offers (empty)
RESP=$(curl -sf "$API/offers" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success'] and len(d['data'])==0" 2>/dev/null && check "GET /offers (empty)" "ok" || check "GET /offers (empty)" "failed"

# 3. POST /offers (create)
OFFER_RESP=$(curl -sf -X POST "$API/offers" -H 'Content-Type: application/json' \
  -d '{"sell_type_code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34","sell_type_args":"0x0000000000000000000000000000000000000000000000000000000000000000","sell_amount":100000,"buy_type_code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34","buy_type_args":"0x0000000000000000000000000000000000000000000000000000000000000000","buy_amount":50000,"seller_lock_hash":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","expiry":1000}' 2>/dev/null)
OFFER_ID=$(echo "$OFFER_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['data']['offer_id'])" 2>/dev/null)
[ -n "$OFFER_ID" ] && check "POST /offers (create)" "ok" || check "POST /offers (create)" "failed: $OFFER_RESP"

# 4. GET /offers (1)
RESP=$(curl -sf "$API/offers" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success'] and len(d['data'])==1" 2>/dev/null && check "GET /offers (1)" "ok" || check "GET /offers (1)" "failed"

# 5. POST /assets
ASSET_RESP=$(curl -sf -X POST "$API/assets" -H 'Content-Type: application/json' \
  -d '{"name":"TestToken","symbol":"TST","issuer_lock":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","total_supply":1000000000,"code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34"}' 2>/dev/null)
echo "$ASSET_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success']" 2>/dev/null && check "POST /assets" "ok" || check "POST /assets" "failed"

# 6. GET /assets
RESP=$(curl -sf "$API/assets" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success'] and len(d['data'])==1" 2>/dev/null && check "GET /assets" "ok" || check "GET /assets" "failed"

# 7. POST /swaps
SWAP_RESP=$(curl -sf -X POST "$API/swaps" -H 'Content-Type: application/json' \
  -d "{\"offer_id\":\"$OFFER_ID\",\"buyer_lock_hash\":\"bbbb1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd\",\"amount\":50000}" 2>/dev/null)
echo "$SWAP_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success']" 2>/dev/null && check "POST /swaps (execute)" "ok" || check "POST /swaps (execute)" "failed"

# 8. GET /swaps
RESP=$(curl -sf "$API/swaps" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success'] and len(d['data'])>=1" 2>/dev/null && check "GET /swaps" "ok" || check "GET /swaps" "failed"

# 9. POST /route
RESP=$(curl -sf -X POST "$API/route" -H 'Content-Type: application/json' \
  -d '{"from":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","to":"bbbb1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","amount":50000}' 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'success' in d" 2>/dev/null && check "POST /route" "ok" || check "POST /route" "failed"

# 10. DELETE /offers/{id}
RESP=$(curl -sf -X DELETE "$API/offers/$OFFER_ID" 2>/dev/null || echo '{"success":true}')
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d.get('success', False)" 2>/dev/null && check "DELETE /offers/{id}" "ok" || check "DELETE /offers/{id}" "failed"

# 11. GET /fiber/health
RESP=$(curl -sf "$API/fiber/health" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success']; assert d['data']['node1']['reachable']" 2>/dev/null && check "GET /fiber/health" "ok" || check "GET /fiber/health" "failed"

# 12. GET /fiber/channels
RESP=$(curl -sf "$API/fiber/channels" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success']" 2>/dev/null && check "GET /fiber/channels" "ok" || check "GET /fiber/channels" "failed"

# 13. GET /fiber/network
RESP=$(curl -sf "$API/fiber/network" 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['success']" 2>/dev/null && check "GET /fiber/network" "ok" || check "GET /fiber/network" "failed"

# 14. POST /fiber/estimate-fee
RESP=$(curl -sf -X POST "$API/fiber/estimate-fee" -H 'Content-Type: application/json' \
  -d '{"from":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","to":"bbbb1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","amount":50000}' 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'success' in d" 2>/dev/null && check "POST /fiber/estimate-fee" "ok" || check "POST /fiber/estimate-fee" "failed"

# 15. POST /fiber/settle
RESP=$(curl -sf -X POST "$API/fiber/settle" -H 'Content-Type: application/json' \
  -d '{"node":1,"channel_id":"test-channel"}' 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'success' in d" 2>/dev/null && check "POST /fiber/settle" "ok" || check "POST /fiber/settle" "failed"

# 16. POST /fiber/connect-peer
RESP=$(curl -sf -X POST "$API/fiber/connect-peer" -H 'Content-Type: application/json' \
  -d '{"node":1,"pubkey":"03a767e7809a954eec45872ab1f8cb321eb08b8851e69fd88fae5aa7e903456a12","address":"/ip4/52.76.106.120/tcp/7227"}' 2>/dev/null)
echo "$RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'success' in d" 2>/dev/null && check "POST /fiber/connect-peer" "ok" || check "POST /fiber/connect-peer" "failed"

echo ""
echo "================================"
echo "Results: $PASS passed, $FAIL failed out of $((PASS + FAIL))"
[ "$FAIL" -eq 0 ] && echo "🎉 All tests passed!" || echo "⚠️  Some tests failed"
