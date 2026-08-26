//! ++ CLI — Command-line interface for the RGB++ DEX.

use clap::{Parser, Subcommand};
use offer_protocol::{CellType, create_offer_payload, build_offer, envelope, serialize_offer};
use router::{NetworkGraph, find_route};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "++", about = "RGB++ DEX — Trustless BTC-to-RGB++ swaps")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List an RGB++ asset for sale (create an offer)
    List {
        /// Amount of RGB++ asset to sell
        #[arg(short, long)]
        sell_amount: u64,

        /// Amount of BTC/payment to request
        #[arg(short, long)]
        buy_amount: u64,

        /// Block number when offer expires
        #[arg(short, long)]
        expiry: u64,

        /// Sell asset code hash (hex)
        #[arg(long)]
        sell_code_hash: String,

        /// Sell asset args (hex)
        #[arg(long, default_value = "")]
        sell_args: String,

        /// Buy asset code hash (hex)
        #[arg(long)]
        buy_code_hash: String,

        /// Buy asset args (hex)
        #[arg(long, default_value = "")]
        buy_args: String,

        /// Your lock hash (hex)
        #[arg(long)]
        my_lock: String,
    },

    /// Browse available offers
    Offers {
        /// Filter by asset (optional)
        #[arg(short, long)]
        asset: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Accept an offer (buy)
    Buy {
        /// Offer ID to accept (hex)
        #[arg(short, long)]
        offer_id: String,

        /// Amount to buy
        #[arg(short, long)]
        amount: u64,

        /// Your lock hash (hex)
        #[arg(long)]
        my_lock: String,
    },

    /// Find a route through Fiber network
    Route {
        /// Source node (hex)
        #[arg(short, long)]
        from: String,

        /// Destination node (hex)
        #[arg(short, long)]
        to: String,

        /// Amount to route
        #[arg(short, long)]
        amount: u64,
    },
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::List {
            sell_amount,
            buy_amount,
            expiry,
            sell_code_hash,
            sell_args,
            buy_code_hash,
            buy_args,
            my_lock,
        } => {
            let sell_type = CellType {
                code_hash: hex::decode(&sell_code_hash)
                    .expect("invalid sell_code_hash")
                    .try_into()
                    .expect("sell_code_hash must be 32 bytes"),
                hash_type: 1,
                args: hex::decode(&sell_args).expect("invalid sell_args"),
            };

            let buy_type = CellType {
                code_hash: hex::decode(&buy_code_hash)
                    .expect("invalid buy_code_hash")
                    .try_into()
                    .expect("buy_code_hash must be 32 bytes"),
                hash_type: 1,
                args: hex::decode(&buy_args).expect("invalid buy_args"),
            };

            let lock_hash: [u8; 32] = hex::decode(&my_lock)
                .expect("invalid my_lock")
                .try_into()
                .expect("my_lock must be 32 bytes");

            let (payload, hash) = create_offer_payload(
                sell_type,
                sell_amount,
                buy_type,
                buy_amount,
                lock_hash,
                expiry,
            );

            // In production, this would sign with a wallet
            let offer = build_offer(payload, vec![0u8; 64]); // placeholder signature
            let env = envelope(&offer);

            println!("Offer created:");
            println!("  Offer ID: {}", hex::encode(env.offer_id));
            println!("  Hash:     {}", hex::encode(hash));
            println!("  Sell:     {} units", env.offer.sell_amount);
            println!("  Buy:      {} units", env.offer.buy_amount);
            println!("  Expires:  block {}", env.offer.expiry);
            println!();
            println!("JSON:");
            println!("{}", String::from_utf8(serialize_offer(&env)).unwrap());
        }

        Commands::Offers { asset, limit } => {
            // In production, this would query the indexer
            println!("Active offers (showing up to {}):", limit);
            if let Some(a) = asset {
                println!("  Filtered by asset: {}", a);
            }
            println!("  (No offers indexed yet — indexer service required)");
        }

        Commands::Buy {
            offer_id,
            amount,
            my_lock,
        } => {
            let offer_id_bytes: [u8; 32] = hex::decode(&offer_id)
                .expect("invalid offer_id")
                .try_into()
                .expect("offer_id must be 32 bytes");

            let lock_hash: [u8; 32] = hex::decode(&my_lock)
                .expect("invalid my_lock")
                .try_into()
                .expect("my_lock must be 32 bytes");

            println!("Accepting offer:");
            println!("  Offer ID: {}", hex::encode(offer_id_bytes));
            println!("  Amount:   {}", amount);
            println!("  Your lock: {}", hex::encode(lock_hash));
            println!();
            println!("  (Requires offer data from indexer and wallet signing)");
        }

        Commands::Route { from, to, amount } => {
            let from_bytes: [u8; 32] = hex::decode(&from)
                .expect("invalid from")
                .try_into()
                .expect("from must be 32 bytes");

            let to_bytes: [u8; 32] = hex::decode(&to)
                .expect("invalid to")
                .try_into()
                .expect("to must be 32 bytes");

            let graph = NetworkGraph::new(); // In production, loaded from node

            let result = find_route(&graph, &from_bytes, &to_bytes, amount, 5);

            if result.success {
                let route = result.route.unwrap();
                println!("Route found:");
                println!("  Path:     {}", route.path.iter().map(|n| hex::encode(&n[..4])).collect::<Vec<_>>().join(" → "));
                println!("  Fee:      {}", route.total_fee);
                println!("  Channels: {}", route.channels.len());
            } else {
                println!("No route: {}", result.error.unwrap());
            }
        }
    }
}
