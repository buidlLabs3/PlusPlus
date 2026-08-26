# Issuer Onboarding Guide

How to list your RGB++ asset on the ++ DEX.

## Step 1: Issue Your RGB++ Asset

Your asset must be issued via the RGB++ protocol on Bitcoin. The asset is bound to a Bitcoin UTXO and represented on CKB via isomorphic binding.

```bash
# Using rgbpp-sdk
rgbpp-cli issue --name "My Token" --symbol "MTK" --supply 1000000 --network testnet
```

## Step 2: Register on ++ DEX

Register your asset with the ++ DEX indexer:

```bash
curl -X POST http://localhost:3000/assets \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Token",
    "symbol": "MTK",
    "issuer_lock": "<your_ckb_lock_hash>",
    "total_supply": 1000000,
    "code_hash": "<rgbpp_type_script_code_hash>"
  }'
```

## Step 3: Create Initial Offers

Create offers to bootstrap liquidity. You sell your token for BTC:

```bash
curl -X POST http://localhost:3000/offers \
  -H "Content-Type: application/json" \
  -d '{
    "sell_type_code_hash": "<your_asset_code_hash>",
    "sell_type_args": "<your_asset_args>",
    "sell_amount": 10000,
    "buy_type_code_hash": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "buy_type_args": "0x",
    "buy_amount": 500,
    "seller_lock_hash": "<your_lock_hash>",
    "expiry": 100000
  }'
```

## Step 4: Monitor Your Listings

```bash
# View all offers for your asset
curl "http://localhost:3000/offers?asset=<your_asset_code_hash>"

# Check swap activity
curl http://localhost:3000/swaps
```

## Tips

- **Set competitive prices.** Check what similar assets trade at.
- **Keep offers active.** Set expiry far in the future (e.g., 500,000 blocks).
- **Monitor fills.** When an offer is filled, create a new one to maintain liquidity.
- **Use multiple offers.** Different price points attract different buyers.
