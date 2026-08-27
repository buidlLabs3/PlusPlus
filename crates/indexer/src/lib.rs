//! Indexer — CKB chain indexer for ++ DEX offers, swaps, and assets.
//!
//! Tails the CKB chain, parses swap-related transactions, and serves
//! a REST API for querying offers, swaps, and registered assets.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An indexed offer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedOffer {
    pub offer_id: String,
    pub sell_asset: String,
    pub sell_amount: u64,
    pub buy_asset: String,
    pub buy_amount: u64,
    pub seller_lock: String,
    pub expiry: u64,
    pub status: OfferStatus,
    pub created_block: u64,
    pub updated_block: u64,
}

/// Status of an offer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OfferStatus {
    Active,
    Filled,
    Expired,
    Cancelled,
}

/// An indexed swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSwap {
    pub tx_hash: String,
    pub offer_id: String,
    pub buyer_lock: String,
    pub amount: u64,
    pub status: SwapStatus,
    pub block: u64,
}

/// Status of a swap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SwapStatus {
    Pending,
    Confirmed,
    Failed,
}

/// A registered RGB++ asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedAsset {
    pub asset_id: String,
    pub name: String,
    pub symbol: String,
    pub issuer_lock: String,
    pub total_supply: u64,
    pub code_hash: String,
    pub registered_block: u64,
}

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

/// Query params for listing offers.
#[derive(Debug, Deserialize)]
pub struct OfferQuery {
    pub asset: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query params for listing swaps.
#[derive(Debug, Deserialize)]
pub struct SwapQuery {
    pub offer_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query params for listing assets.
#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ---------------------------------------------------------------------------
// API response wrappers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub total: Option<usize>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { success: true, data: Some(data), error: None, total: None }
    }

    pub fn ok_with_total(data: T, total: usize) -> Self {
        Self { success: true, data: Some(data), error: None, total: Some(total) }
    }

    pub fn err(msg: &str) -> Self {
        Self { success: false, data: None, error: Some(msg.to_string()), total: None }
    }
}

// ---------------------------------------------------------------------------
// Database operations (SQLite)
// ---------------------------------------------------------------------------

/// Initialize the database schema.
pub async fn init_db(db: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS offers (
            offer_id TEXT PRIMARY KEY,
            sell_asset TEXT NOT NULL,
            sell_amount INTEGER NOT NULL,
            buy_asset TEXT NOT NULL,
            buy_amount INTEGER NOT NULL,
            seller_lock TEXT NOT NULL,
            expiry INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'Active',
            created_block INTEGER NOT NULL,
            updated_block INTEGER NOT NULL,
            invoice_hash TEXT,
            invoice_cancelled BOOLEAN DEFAULT FALSE
        )"
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS swaps (
            tx_hash TEXT PRIMARY KEY,
            offer_id TEXT NOT NULL,
            buyer_lock TEXT NOT NULL,
            amount INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'Pending',
            block INTEGER NOT NULL,
            payment_hash TEXT,
            FOREIGN KEY (offer_id) REFERENCES offers(offer_id)
        )"
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS assets (
            asset_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            symbol TEXT NOT NULL,
            issuer_lock TEXT NOT NULL,
            total_supply INTEGER NOT NULL,
            code_hash TEXT NOT NULL,
            registered_block INTEGER NOT NULL
        )"
    )
    .execute(db)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS channel_states (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_id TEXT NOT NULL,
            balance_a INTEGER NOT NULL,
            balance_b INTEGER NOT NULL,
            sequence INTEGER NOT NULL,
            block_number INTEGER NOT NULL,
            settled BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .execute(db)
    .await?;

    // Migration: add invoice_hash column to offers if missing
    let _ = sqlx::query("ALTER TABLE offers ADD COLUMN invoice_hash TEXT").execute(db).await;
    let _ = sqlx::query("ALTER TABLE offers ADD COLUMN invoice_cancelled BOOLEAN DEFAULT FALSE").execute(db).await;
    let _ = sqlx::query("ALTER TABLE swaps ADD COLUMN payment_hash TEXT").execute(db).await;

    Ok(())
}

/// Insert or update an offer.
pub async fn upsert_offer(db: &sqlx::SqlitePool, offer: &IndexedOffer) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO offers (offer_id, sell_asset, sell_amount, buy_asset, buy_amount, seller_lock, expiry, status, created_block, updated_block)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&offer.offer_id)
    .bind(&offer.sell_asset)
    .bind(offer.sell_amount as i64)
    .bind(&offer.buy_asset)
    .bind(offer.buy_amount as i64)
    .bind(&offer.seller_lock)
    .bind(offer.expiry as i64)
    .bind(&offer.status.to_string())
    .bind(offer.created_block as i64)
    .bind(offer.updated_block as i64)
    .execute(db)
    .await?;
    Ok(())
}

/// Query offers with optional filters.
pub async fn query_offers(
    db: &sqlx::SqlitePool,
    query: &OfferQuery,
) -> Result<Vec<IndexedOffer>, sqlx::Error> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows: Vec<(String, String, i64, String, i64, String, i64, String, i64, i64)> =
        if let Some(ref asset) = query.asset {
            sqlx::query_as(
                "SELECT offer_id, sell_asset, sell_amount, buy_asset, buy_amount, seller_lock, expiry, status, created_block, updated_block
                 FROM offers WHERE sell_asset = ? OR buy_asset = ? ORDER BY created_block DESC LIMIT ? OFFSET ?"
            )
            .bind(asset)
            .bind(asset)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(db)
            .await?
        } else {
            sqlx::query_as(
                "SELECT offer_id, sell_asset, sell_amount, buy_asset, buy_amount, seller_lock, expiry, status, created_block, updated_block
                 FROM offers ORDER BY created_block DESC LIMIT ? OFFSET ?"
            )
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(db)
            .await?
        };

    Ok(rows.into_iter().map(|r| IndexedOffer {
        offer_id: r.0,
        sell_asset: r.1,
        sell_amount: r.2 as u64,
        buy_asset: r.3,
        buy_amount: r.4 as u64,
        seller_lock: r.5,
        expiry: r.6 as u64,
        status: OfferStatus::from_str(&r.7),
        created_block: r.8 as u64,
        updated_block: r.9 as u64,
    }).collect())
}

/// Insert a swap record.
pub async fn insert_swap(db: &sqlx::SqlitePool, swap: &IndexedSwap) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO swaps (tx_hash, offer_id, buyer_lock, amount, status, block)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&swap.tx_hash)
    .bind(&swap.offer_id)
    .bind(&swap.buyer_lock)
    .bind(swap.amount as i64)
    .bind(&swap.status.to_string())
    .bind(swap.block as i64)
    .execute(db)
    .await?;
    Ok(())
}

/// Query swaps.
pub async fn query_swaps(
    db: &sqlx::SqlitePool,
    query: &SwapQuery,
) -> Result<Vec<IndexedSwap>, sqlx::Error> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows: Vec<(String, String, String, i64, String, i64)> = sqlx::query_as(
        "SELECT tx_hash, offer_id, buyer_lock, amount, status, block
         FROM swaps ORDER BY block DESC LIMIT ? OFFSET ?"
    )
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|r| IndexedSwap {
        tx_hash: r.0,
        offer_id: r.1,
        buyer_lock: r.2,
        amount: r.3 as u64,
        status: SwapStatus::from_str(&r.4),
        block: r.5 as u64,
    }).collect())
}

/// Insert an asset record.
pub async fn insert_asset(db: &sqlx::SqlitePool, asset: &IndexedAsset) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO assets (asset_id, name, symbol, issuer_lock, total_supply, code_hash, registered_block)
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&asset.asset_id)
    .bind(&asset.name)
    .bind(&asset.symbol)
    .bind(&asset.issuer_lock)
    .bind(asset.total_supply as i64)
    .bind(&asset.code_hash)
    .bind(asset.registered_block as i64)
    .execute(db)
    .await?;
    Ok(())
}

/// Query assets.
pub async fn query_assets(
    db: &sqlx::SqlitePool,
    query: &AssetQuery,
) -> Result<Vec<IndexedAsset>, sqlx::Error> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows: Vec<(String, String, String, String, i64, String, i64)> = sqlx::query_as(
        "SELECT asset_id, name, symbol, issuer_lock, total_supply, code_hash, registered_block
         FROM assets ORDER BY registered_block DESC LIMIT ? OFFSET ?"
    )
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|r| IndexedAsset {
        asset_id: r.0,
        name: r.1,
        symbol: r.2,
        issuer_lock: r.3,
        total_supply: r.4 as u64,
        code_hash: r.5,
        registered_block: r.6 as u64,
    }).collect())
}

// ---------------------------------------------------------------------------
// Status helpers
// ---------------------------------------------------------------------------

impl OfferStatus {
    pub fn to_string(&self) -> String {
        match self {
            OfferStatus::Active => "Active".to_string(),
            OfferStatus::Filled => "Filled".to_string(),
            OfferStatus::Expired => "Expired".to_string(),
            OfferStatus::Cancelled => "Cancelled".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Active" => OfferStatus::Active,
            "Filled" => OfferStatus::Filled,
            "Expired" => OfferStatus::Expired,
            "Cancelled" => OfferStatus::Cancelled,
            _ => OfferStatus::Active,
        }
    }
}

impl SwapStatus {
    pub fn to_string(&self) -> String {
        match self {
            SwapStatus::Pending => "Pending".to_string(),
            SwapStatus::Confirmed => "Confirmed".to_string(),
            SwapStatus::Failed => "Failed".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Pending" => SwapStatus::Pending,
            "Confirmed" => SwapStatus::Confirmed,
            "Failed" => SwapStatus::Failed,
            _ => SwapStatus::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offer_status_roundtrip() {
        let statuses = vec![
            OfferStatus::Active,
            OfferStatus::Filled,
            OfferStatus::Expired,
            OfferStatus::Cancelled,
        ];
        for s in statuses {
            let str_val = s.to_string();
            let parsed = OfferStatus::from_str(&str_val);
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_swap_status_roundtrip() {
        let statuses = vec![
            SwapStatus::Pending,
            SwapStatus::Confirmed,
            SwapStatus::Failed,
        ];
        for s in statuses {
            let str_val = s.to_string();
            let parsed = SwapStatus::from_str(&str_val);
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_api_response_ok() {
        let resp: ApiResponse<String> = ApiResponse::ok("test".to_string());
        assert!(resp.success);
        assert_eq!(resp.data.unwrap(), "test");
    }

    #[test]
    fn test_api_response_err() {
        let resp: ApiResponse<String> = ApiResponse::err("error");
        assert!(!resp.success);
        assert_eq!(resp.error.unwrap(), "error");
    }
}
