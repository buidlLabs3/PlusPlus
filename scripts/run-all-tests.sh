#!/bin/bash
# Run all API tests in a single shell session
cd /home/core/Documents/ckb/PlusPlus
kill -9 $(pgrep -f plusplus-server) 2>/dev/null
sleep 1
rm -f plusplus.db
PLUSPLUS_DB=plusplus.db PORT=3000 RUST_LOG=warn ./target/debug/plusplus-server </dev/null >/dev/null 2>&1 &
sleep 5

API="http://localhost:3000"
PASS=0; FAIL=0

t() {
    local name="$1"; local expected="$2"; local actual="$3"
    if echo "$actual" | grep -q "$expected"; then
        echo "✅ $name"; PASS=$((PASS+1))
    else
        echo "❌ $name (expected '$expected')"; echo "   got: $(echo $actual | head -c 120)"; FAIL=$((FAIL+1))
    fi
}

echo "🧪 ++ DEX API Test Suite"
echo "========================"

R=$(curl -s $API/info)
t "GET /info" '"success":true' "$R"

R=$(curl -s $API/offers)
t "GET /offers (empty)" '"total":0' "$R"

R=$(curl -s -X POST $API/offers -H 'Content-Type: application/json' -d '{"sell_type_code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34","sell_type_args":"0x0000000000000000000000000000000000000000000000000000000000000000","sell_amount":100000,"buy_type_code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34","buy_type_args":"0x0000000000000000000000000000000000000000000000000000000000000000","buy_amount":50000,"seller_lock_hash":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","expiry":1000}')
t "POST /offers (create)" '"success":true' "$R"
OID=$(echo $R | python3 -c "import sys,json;print(json.load(sys.stdin)['data']['offer_id'])" 2>/dev/null)

R=$(curl -s $API/offers)
t "GET /offers (1)" '"total":1' "$R"

R=$(curl -s -X POST $API/assets -H 'Content-Type: application/json' -d '{"name":"Test","symbol":"T","issuer_lock":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","total_supply":1000,"code_hash":"0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34"}')
t "POST /assets (register)" '"success":true' "$R"

R=$(curl -s $API/assets)
t "GET /assets" '"total":1' "$R"

R=$(curl -s -X POST $API/swaps -H 'Content-Type: application/json' -d "{\"offer_id\":\"$OID\",\"buyer_lock_hash\":\"bbbb1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd\",\"amount\":50000}")
t "POST /swaps (execute)" '"success":true' "$R"

R=$(curl -s $API/swaps)
t "GET /swaps" '"total":1' "$R"

R=$(curl -s -X POST $API/route -H 'Content-Type: application/json' -d '{"from":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","to":"bbbb1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","amount":50000}')
t "POST /route" '"success":true' "$R"

R=$(curl -s -X DELETE "$API/offers/$OID")
t "DELETE /offers" '"success":true' "$R"

R=$(curl -s $API/fiber/health)
t "GET /fiber/health" '"reachable":true' "$R"

R=$(curl -s $API/fiber/channels)
t "GET /fiber/channels" '"success":true' "$R"

R=$(curl -s $API/fiber/network)
t "GET /fiber/network" '"success":true' "$R"

R=$(curl -s -X POST $API/fiber/estimate-fee -H 'Content-Type: application/json' -d '{"from":"aaaa1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","to":"bbbb1234567890abcdef1234567890abcdef1234567890abcdef12345678abcd","amount":50000}')
t "POST /fiber/estimate-fee" '"success":true' "$R"

R=$(curl -s -X POST $API/fiber/connect-peer -H 'Content-Type: application/json' -d '{"node":1,"pubkey":"03a767e7809a954eec45872ab1f8cb321eb08b8851e69fd88fae5aa7e903456a12","address":"/ip4/52.76.106.120/tcp/7227"}')
t "POST /fiber/connect-peer" '"success":true' "$R"

echo ""
echo "========================"
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] && echo "🎉 ALL TESTS PASSED!" || echo "⚠️  Some tests failed"
