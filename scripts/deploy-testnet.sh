#!/usr/bin/env bash
# Builds the contract and deploys it to Stellar testnet, with Circle's testnet
# USDC passed to the constructor. Prints the new contract ID.
#
# Usage: scripts/deploy-testnet.sh [source-key-alias]
#   source-key-alias  Stellar CLI key that pays for the deploy (default: inowo-deployer)
#
# Override the token with USDC_SAC=<contract-id> if needed.
set -euo pipefail

SOURCE="${1:-inowo-deployer}"
USDC_SAC="${USDC_SAC:-CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA}"

cd "$(dirname "$0")/.."

cargo build --target wasm32v1-none --release >&2

stellar contract deploy \
  --wasm target/wasm32v1-none/release/inowo_contract.wasm \
  --network testnet \
  --source "$SOURCE" \
  -- \
  --token "$USDC_SAC"
