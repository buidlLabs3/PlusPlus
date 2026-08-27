# ++ DEX — API Reference

## Server

- **File:** `crates/plusplus-server/src/main.rs`
- **Port:** `3000` (configurable via `PORT` env var)
- **DB:** `plusplus.db` (configurable via `PLUSPLUS_DB` env var)

---

## Endpoints

| Method | Endpoint | Handler | File:Line | Description |
|--------|----------|---------|-----------|-------------|
| GET | `/info` | `server_info` | `main.rs:500` | Server info and stats |
| GET | `/offers` | `list_offers` | `main.rs:189` | List offers with filters |
| POST | `/offers` | `create_offer` | `main.rs:208` | Create a new offer |
| DELETE | `/offers/{offer_id}` | `cancel_offer` | `main.rs:272` | Cancel an offer |
| GET | `/swaps` | `list_swaps` | `main.rs:306` | List swap history |
| POST | `/swaps` | `execute_swap` | `main.rs:324` | Execute a swap |
| GET | `/assets` | `list_assets` | `main.rs:412` | List registered assets |
| POST | `/assets` | `register_asset` | `main.rs:430` | Register a new asset |
| POST | `/route` | `find_route_handler` | `main.rs:488` | Find route through Fiber |
| WS | `/ws` | `ws_handler` | `main.rs:548` | Real-time event stream |

---

## GET /info

Returns server version, network, and counts.

**Handler:** `server_info` — `main.rs:500`

**Response:**
```json
{
  "success": true,
  "data": {
    "name": "++ DEX",
    "version": "0.1.0",
    "network": "testnet",
    "offers_count": 5,
    "swaps_count": 12,
    "assets_count": 3
  }
}
```

---

## GET /offers

List offers with optional filters.

**Handler:** `list_offers` — `main.rs:189`

**Query Params:**
| Param | Type | Description |
|-------|------|-------------|
| `asset` | `string` | Filter by sell or buy asset code hash |
| `status` | `string` | Filter by status: `Active`, `Filled`, `Expired`, `Cancelled` |
| `limit` | `int` | Max results (default: 50) |
| `offset` | `int` | Pagination offset (default: 0) |

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "offer_id": "a1b2c3...",
      "sell_asset": "0x6798ae...",
      "sell_amount": 10000,
      "buy_asset": "0x000000...",
      "buy_amount": 500,
      "seller_lock": "dfec80...",
      "expiry": 100000,
      "status": "Active",
      "created_block": 0,
      "updated_block": 0
    }
  ],
  "total": 1
}
```

---

## POST /offers

Create a new swap offer.

**Handler:** `create_offer` — `main.rs:208`

**Request Body:**
| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `sell_type_code_hash` | `string` | yes | 66 hex chars (0x + 64) |
| `sell_type_args` | `string` | yes | hex string |
| `sell_amount` | `u64` | yes | > 0, < 1e15 |
| `buy_type_code_hash` | `string` | yes | 66 hex chars (0x + 64) |
| `buy_type_args` | `string` | yes | hex string |
| `buy_amount` | `u64` | yes | > 0, < 1e15 |
| `seller_lock_hash` | `string` | yes | 64 hex chars |
| `expiry` | `u64` | yes | > 0, < 1e15 (block height) |
| `signature` | `string` | no | 128 or 130 hex chars |

**Response:**
```json
{
  "success": true,
  "data": {
    "offer_id": "a1b2c3...",
    "sell_asset": "0x6798ae...",
    "sell_amount": 10000,
    "buy_asset": "0x000000...",
    "buy_amount": 500,
    "seller_lock": "dfec80...",
    "expiry": 100000,
    "status": "Active",
    "created_block": 0,
    "updated_block": 0
  }
}
```

**Broadcasts:** `offer.created`

---

## DELETE /offers/{offer_id}

Cancel an active offer.

**Handler:** `cancel_offer` — `main.rs:272`

**Path Params:**
| Param | Type | Validation |
|-------|------|------------|
| `offer_id` | `string` | 64 hex chars |

**Response:**
```json
{ "success": true, "data": null }
```

**Broadcasts:** `offer.cancelled`

---

## GET /swaps

List swap history.

**Handler:** `list_swaps` — `main.rs:306`

**Query Params:**
| Param | Type | Description |
|-------|------|-------------|
| `offer_id` | `string` | Filter by offer ID |
| `status` | `string` | Filter by status: `Pending`, `Confirmed`, `Failed` |
| `limit` | `int` | Max results (default: 50) |
| `offset` | `int` | Pagination offset (default: 0) |

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "tx_hash": "a1b2c3...",
      "offer_id": "d4e5f6...",
      "buyer_lock": "789abc...",
      "amount": 500,
      "status": "Pending",
      "block": 0
    }
  ],
  "total": 1
}
```

---

## POST /swaps

Execute a swap against an active offer.

**Handler:** `execute_swap` — `main.rs:324`

**Request Body:**
| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `offer_id` | `string` | yes | must match existing active offer |
| `buyer_lock_hash` | `string` | yes | 64 hex chars |
| `amount` | `u64` | yes | > 0, ≤ offer's `buy_amount` |
| `signature` | `string` | no | hex signature |

**Response:**
```json
{
  "success": true,
  "data": {
    "tx_hash": "a1b2c3...",
    "offer_id": "d4e5f6...",
    "buyer_lock": "789abc...",
    "amount": 500,
    "status": "Pending",
    "block": 0
  }
}
```

**Behavior:**
- Validates amount ≤ offer's buy_amount
- Marks offer as `Filled` when fully consumed
- **Broadcasts:** `swap.executed`

---

## GET /assets

List registered RGB++ assets.

**Handler:** `list_assets` — `main.rs:412`

**Query Params:**
| Param | Type | Description |
|-------|------|-------------|
| `search` | `string` | Search by name or symbol |
| `limit` | `int` | Max results (default: 50) |
| `offset` | `int` | Pagination offset (default: 0) |

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "asset_id": "a1b2c3...",
      "name": "TokenX",
      "symbol": "TKX",
      "issuer_lock": "dfec80...",
      "total_supply": 1000000,
      "code_hash": "0x6798ae...",
      "registered_block": 0
    }
  ],
  "total": 1
}
```

---

## POST /assets

Register a new RGB++ asset.

**Handler:** `register_asset` — `main.rs:430`

**Request Body:**
| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `name` | `string` | yes | 1-64 chars |
| `symbol` | `string` | yes | 1-16 chars |
| `issuer_lock` | `string` | yes | issuer's lock hash |
| `total_supply` | `u64` | yes | > 0, < 1e15 |
| `code_hash` | `string` | yes | asset code hash |

**Response:**
```json
{
  "success": true,
  "data": {
    "asset_id": "a1b2c3...",
    "name": "TokenX",
    "symbol": "TKX",
    "issuer_lock": "dfec80...",
    "total_supply": 1000000,
    "code_hash": "0x6798ae...",
    "registered_block": 0
  }
}
```

**Broadcasts:** `asset.registered`

---

## POST /route

Find a route through Fiber network channels.

**Handler:** `find_route_handler` — `main.rs:488`

**Request Body:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `from` | `string` | yes | Source node lock hash |
| `to` | `string` | yes | Destination node lock hash |
| `amount` | `u64` | yes | Amount to route |

**Response:**
```json
{
  "success": true,
  "data": {
    "path": ["0xabc...", "0xdef..."],
    "channels": [],
    "total_fee": 100,
    "max_amount": 5000
  }
}
```

---

## WebSocket /ws

Real-time event stream.

**Handler:** `ws_handler` → `handle_ws` — `main.rs:548` → `main.rs:556`

**Connection:** `ws://localhost:3000/ws`

**Welcome message:**
```json
{ "type": "connected", "message": "Connected to ++ DEX event stream" }
```

**Event messages:**
```json
{
  "event": { "type": "offer.created", "offer_id": "...", "seller_lock": "..." },
  "timestamp": "2026-08-27T00:00:00Z"
}
```

**Event types:**
| Event | Fields |
|-------|--------|
| `offer.created` | `offer_id`, `seller_lock` |
| `offer.cancelled` | `offer_id` |
| `swap.executed` | `tx_hash`, `offer_id`, `amount` |
| `swap.confirmed` | `tx_hash`, `offer_id` |
| `swap.failed` | `tx_hash`, `error` |
| `asset.registered` | `asset_id`, `name`, `symbol` |

---

## Response Format

All REST endpoints return:
```json
{
  "success": true|false,
  "data": <object|array|null>,
  "error": <string|null>,
  "total": <int|null>
}
```

## Database

**File:** `crates/indexer/src/lib.rs`

Tables: `offers`, `swaps`, `assets` — see `init_db()` at `indexer/src/lib.rs:83`

## CORS

Fully permissive (`CorsLayer::permissive()`) — acceptable for testnet, restrict for mainnet.
