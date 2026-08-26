#![no_std]
#![no_main]

ckb_std::entry!(program_entry);
ckb_std::default_alloc!(16384, 1258306, 64);

use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::prelude::*;
use ckb_std::high_level::*;

const ERROR_INVALID_SYNTAX: i8 = -1;
const ERROR_UNAUTHORIZED_SELLER: i8 = -3;
const ERROR_UNAUTHORIZED_BUYER: i8 = -4;
const ERROR_SWAP_NOT_ATOMIC: i8 = -5;
const ERROR_INVALID_AMOUNTS: i8 = -6;
const ERROR_WRONG_OUTPUT_LOCK: i8 = -7;
const ERROR_EXPIRED: i8 = -8;

// Cell data layout: 88 bytes
// [seller_lock_hash: 32] [buyer_lock_hash: 32] [sell_amount: 8] [buy_amount: 8] [expiry: 8]
const SELLER_LOCK_HASH_END: usize = 32;
const BUYER_LOCK_HASH_END: usize = 64;
const SELL_AMOUNT_END: usize = 72;
const BUY_AMOUNT_END: usize = 80;
const EXPIRY_END: usize = 88;

fn read_u64(data: &[u8], start: usize) -> u64 {
    let bytes: [u8; 8] = data[start..start + 8].try_into().unwrap();
    u64::from_le_bytes(bytes)
}

fn verify_input_lock(input_index: usize, expected_hash: &[u8; 32]) -> bool {
    match load_cell_lock_hash(input_index, Source::Input) {
        Ok(actual_hash) => actual_hash == *expected_hash,
        Err(_) => false,
    }
}

pub fn program_entry() -> i8 {
    // Read swap data from the INPUT cell (the swap cell being consumed)
    // Input[0] is the swap cell created by the seller with terms
    let data = match load_cell_data(0, Source::Input) {
        Ok(d) => d,
        Err(_) => return ERROR_INVALID_SYNTAX,
    };

    if data.len() < EXPIRY_END {
        return ERROR_INVALID_SYNTAX;
    }

    let seller_lock_hash: [u8; 32] = data[..SELLER_LOCK_HASH_END].try_into().unwrap();
    let buyer_lock_hash: [u8; 32] = data[SELLER_LOCK_HASH_END..BUYER_LOCK_HASH_END]
        .try_into()
        .unwrap();
    let sell_amount = read_u64(&data, BUYER_LOCK_HASH_END);
    let buy_amount = read_u64(&data, SELL_AMOUNT_END);
    let expiry = read_u64(&data, BUY_AMOUNT_END);

    if sell_amount == 0 || buy_amount == 0 {
        return ERROR_INVALID_AMOUNTS;
    }

    // Load current block number from header dep
    let current_block = match load_header(0, Source::HeaderDep) {
        Ok(header) => header.raw().number().unpack(),
        Err(_) => return ERROR_INVALID_SYNTAX,
    };

    // Expired: seller can reclaim their cell
    if current_block > expiry {
        // Input[0] must be the seller's cell
        if !verify_input_lock(0, &seller_lock_hash) {
            return ERROR_UNAUTHORIZED_SELLER;
        }
        // Output must go back to seller
        match load_cell_lock_hash(0, Source::Output) {
            Ok(h) => {
                if h != seller_lock_hash {
                    return ERROR_WRONG_OUTPUT_LOCK;
                }
            }
            Err(_) => return ERROR_INVALID_SYNTAX,
        }
        return 0;
    }

    // Active swap execution:
    // Input[0] = swap cell (seller signed to create this)
    // Input[1] = buyer's payment cell
    // Output[0] = buyer receives the RGB++ asset
    // Output[1] = seller receives the BTC payment

    // Verify buyer's input lock (input[1] = buyer's payment cell)
    if !verify_input_lock(1, &buyer_lock_hash) {
        return ERROR_UNAUTHORIZED_BUYER;
    }

    // Output[0] → buyer (gets the RGB++ asset being sold)
    match load_cell_lock_hash(0, Source::Output) {
        Ok(h) => {
            if h != buyer_lock_hash {
                return ERROR_SWAP_NOT_ATOMIC;
            }
        }
        Err(_) => return ERROR_SWAP_NOT_ATOMIC,
    }

    // Output[1] → seller (gets the BTC payment)
    match load_cell_lock_hash(1, Source::Output) {
        Ok(h) => {
            if h != seller_lock_hash {
                return ERROR_SWAP_NOT_ATOMIC;
            }
        }
        Err(_) => return ERROR_SWAP_NOT_ATOMIC,
    }

    // Verify amounts in outputs match the terms
    // Output[0] capacity should match sell_amount (the asset being transferred)
    match load_cell_capacity(0, Source::Output) {
        Ok(cap) => {
            if cap < sell_amount {
                return ERROR_INVALID_AMOUNTS;
            }
        }
        Err(_) => return ERROR_INVALID_SYNTAX,
    }

    // Output[1] capacity should match buy_amount (the payment)
    match load_cell_capacity(1, Source::Output) {
        Ok(cap) => {
            if cap < buy_amount {
                return ERROR_INVALID_AMOUNTS;
            }
        }
        Err(_) => return ERROR_INVALID_SYNTAX,
    }

    0
}
