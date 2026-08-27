#!/usr/bin/env python3
"""Full API test suite for ++ DEX server."""
import subprocess, json, time, os, sys

API = "http://localhost:3000"
passed = 0
failed = 0

def api(method, path, body=None):
    cmd = ["curl", "-s", "-w", "\nHTTP:%{http_code}", "-X", method, f"{API}{path}"]
    if body:
        cmd.extend(["-H", "Content-Type: application/json", "-d", json.dumps(body)])
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        lines = r.stdout.strip().split("\n")
        http_code = "?"
        body_lines = []
        for line in lines:
            if line.startswith("HTTP:"):
                http_code = line.split(":", 1)[1].strip()
            else:
                body_lines.append(line)
        body_text = "\n".join(body_lines).strip()
        data = json.loads(body_text) if body_text else {}
        return data, http_code
    except Exception as e:
        return {"error": str(e)}, "err"

def test(name, condition, extra=""):
    global passed, failed
    if condition:
        print(f"  ✅ {name} {extra}")
        passed += 1
    else:
        print(f"  ❌ {name} {extra}")
        failed += 1

# Start server
env = os.environ.copy()
env.update({"PLUSPLUS_DB": "plusplus.db", "PORT": "3000", "RUST_LOG": "warn"})
os.system("kill -9 $(pgrep -f plusplus-server) 2>/dev/null; sleep 1; rm -f plusplus.db plusplus.db-wal plusplus.db-shm")
srv = subprocess.Popen(["./target/debug/plusplus-server"], env=env,
    stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(5)

print("🧪 ++ DEX Full API Test Suite")
print("=" * 40)

LOCK_A = "a" * 16 + "1234567890abcdef" * 3
LOCK_B = "b" * 16 + "1234567890abcdef" * 3
CODE_HASH = "0x9bd7e06f3ecf4be0f2fcd2188b23f1df992b8113e9922bc63a1a340c1fb1df34"
ZERO_ARGS = "0x" + "00" * 32

# 1: Server info
r, code = api("GET", "/info")
test("GET /info", r.get("success") and code == "200", f"v={r.get('data',{}).get('version','?')}")

# 2: List offers (should be empty after fresh DB)
r, code = api("GET", "/offers")
test("GET /offers", r.get("success") and code == "200", f"count={len(r.get('data',[]))}")

# 3: Create offer
r, code = api("POST", "/offers", {
    "sell_type_code_hash": CODE_HASH, "sell_type_args": ZERO_ARGS, "sell_amount": 100000,
    "buy_type_code_hash": CODE_HASH, "buy_type_args": ZERO_ARGS, "buy_amount": 50000,
    "seller_lock_hash": LOCK_A, "expiry": 1000
})
oid = r.get("data", {}).get("offer_id", "")
test("POST /offers", r.get("success") and code == "200" and len(oid) == 64, f"id={oid[:12]}...")

# 4: List offers (1)
r, code = api("GET", "/offers")
test("GET /offers (1)", r.get("success") and len(r.get("data", [])) == 1)

# 5: Register asset
r, code = api("POST", "/assets", {
    "name": "TestToken", "symbol": "TST", "issuer_lock": LOCK_A,
    "total_supply": 1000000000, "code_hash": CODE_HASH
})
test("POST /assets", r.get("success") and code == "200")

# 6: List assets
r, code = api("GET", "/assets")
test("GET /assets", r.get("success") and len(r.get("data", [])) >= 1)

# 7: Execute swap
r, code = api("POST", "/swaps", {"offer_id": oid, "buyer_lock_hash": LOCK_B, "amount": 50000})
test("POST /swaps", r.get("success") and code == "200", f"status={r.get('data',{}).get('status','?')}")

# 8: List swaps
r, code = api("GET", "/swaps")
test("GET /swaps", r.get("success") and len(r.get("data", [])) >= 1)

# 9: Route
r, code = api("POST", "/route", {"from": LOCK_A, "to": LOCK_B, "amount": 50000})
test("POST /route", r.get("success", False) and code == "200")

# 10: Cancel offer
r, code = api("POST", f"/offers/{oid}/cancel")
test("POST /offers/{id}/cancel", r.get("success", False) and code == "200", f"http={code}")

# 11: Fiber health
r, code = api("GET", "/fiber/health")
n1 = r.get("data", {}).get("node1", {})
test("GET /fiber/health", r.get("success") and code == "200" and n1.get("reachable"),
     f"pubkey={n1.get('pubkey', '?')[:16]}...")

# 12: Fiber channels
r, code = api("GET", "/fiber/channels")
test("GET /fiber/channels", r.get("success") and code == "200", f"count={len(r.get('data', []))}")

# 13: Fiber network
r, code = api("GET", "/fiber/network")
test("GET /fiber/network", r.get("success") and code == "200" and "node1" in r.get("data", {}),
     f"peers={len(r.get('data', {}).get('peers', []))}")

# 14: Fee estimation
r, code = api("POST", "/fiber/estimate-fee", {"from": LOCK_A, "to": LOCK_B, "amount": 50000})
test("POST /fiber/estimate-fee", r.get("success", False) and code == "200")

# 15: Connect peer
r, code = api("POST", "/fiber/connect-peer", {
    "node": 1,
    "pubkey": "03a767e7809a954eec45872ab1f8cb321eb08b8851e69fd88fae5aa7e903456a12",
    "address": "/ip4/52.76.106.120/tcp/7227"
})
test("POST /fiber/connect-peer", r.get("success", False) and code == "200")

# 16: Settle (no real channels, so "Channel not found" is expected)
r, code = api("POST", "/fiber/settle", {"node": 1, "channel_id": "test"})
test("POST /fiber/settle", code == "200", f"success={r.get('success')} error={r.get('error','none')}")

# Cleanup
srv.kill()
srv.wait()

print(f"\n{'=' * 40}")
print(f"Results: {passed} passed, {failed} failed out of {passed + failed}")
if failed == 0:
    print("🎉 ALL TESTS PASSED!")
else:
    print(f"⚠️  {failed} test(s) failed")
    sys.exit(1)
