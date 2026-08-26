#!/bin/bash
# Build script for Vercel: injects PLUSPLUS_API_URL env var into the HTML
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

export OUT="$PROJECT_ROOT/dist/index.html"
SRC="$PROJECT_ROOT/web/index.html"

mkdir -p "$PROJECT_ROOT/dist"
cp "$SRC" "$OUT"

python3 << 'PYEOF'
import os
with open(os.environ['OUT'], 'r') as f:
    content = f.read()

api_url = os.environ.get('PLUSPLUS_API_URL', '')
ws_url = os.environ.get('PLUSPLUS_WS_URL', '')

if api_url:
    content = content.replace("'__PLUSPLUS_API_URL__'", repr(api_url))
    print(f'Injected API URL: {api_url[:40]}...')
else:
    content = content.replace("'__PLUSPLUS_API_URL__'", 'window.location.origin')
    print('Using same-origin API')

if ws_url:
    content = content.replace("'__PLUSPLUS_WS_URL__'", repr(ws_url))
else:
    content = content.replace("'__PLUSPLUS_WS_URL__'", 'null')

with open(os.environ['OUT'], 'w') as f:
    f.write(content)
print('Build complete')
PYEOF

echo "Output: $OUT"
