#!/usr/bin/env bash
# Regenerates bindings/ from the current contract source — never hand-edit
# bindings/src/index.ts, it's overwritten here every time.
#
# Usage: scripts/generate-bindings.sh
set -euo pipefail

cd "$(dirname "$0")/.."

cargo build --target wasm32v1-none --release >&2

stellar contract bindings typescript \
  --wasm target/wasm32v1-none/release/inowo_contract.wasm \
  --output-dir bindings \
  --overwrite >&2

# The CLI's --overwrite replaces package.json, README.md, and .gitignore
# with its own generic boilerplate too; put back our maintained versions of
# all three so repeated runs are idempotent (and CI's diff check doesn't
# flag a no-op run).
cp scripts/templates/bindings-package.json bindings/package.json
cp scripts/templates/bindings-readme.md bindings/README.md
cp scripts/templates/bindings-gitignore.txt bindings/.gitignore

# The CLI's internal ordering of the ContractSpec array isn't guaranteed
# stable across platforms (seen in practice: Windows vs. CI's Linux build
# producing the same entries in a different order). Canonicalize it so
# CI's diff check only fails on an actual content change.
node scripts/sort-bindings-spec.mjs

echo "Regenerated bindings/ from the current contract." >&2
