# Contributing to Inowo

Thanks for considering a contribution. This guide covers everything you need to go from zero to a merged PR.

## What this project is

Inowo is a Soroban smart contract (Rust) for transparent event management on Stellar. Every financial action — sponsor contributions, ticket sales, and worker payouts — settles on-chain through a single contract that serves as the public ledger for an event.

If you haven't read the README, do that first. It explains the roles, the transparency thesis, and the overall architecture.

## Before you start

Browse the open issues. Each issue has clear acceptance criteria that define what "done" looks like. Pick one, leave a comment so others know it's being worked on, and only then start writing code.

If you want to work on something that isn't in the issues, open an issue first and describe what you'd like to build. Don't spend time writing code for a change that hasn't been discussed.

## Setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- The `wasm32v1-none` target:
  ```bash
  rustup target add wasm32v1-none
  ```
- [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) for deploying to testnet

### Format

```bash
cargo fmt
```

### Lint

```bash
cargo clippy -- -D warnings
```

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

All tests must pass before you open a PR. New functions require new tests, and failure tests should assert the exact `Error` variant, not just `is_err()`.

If you're working on a single function, you don't need to run the full suite every time:

```bash
# Run a single test by name
cargo test test_end_event_changes_status_to_ended

# Run all tests whose name contains a keyword
cargo test end_event
```

Add `-- --nocapture` to see `println!` output during tests:

```bash
cargo test -- --nocapture
```

### Build the WASM artifact

```bash
cargo build --target wasm32v1-none --release
```

The compiled contract ends up at `target/wasm32v1-none/release/inowo_contract.wasm`.

### Deploy to testnet (optional, for manual verification)

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/inowo_contract.wasm \
  --network testnet \
  --source <your-account> \
  -- \
  --token <usdc-token-contract-id>
```

To get your own deployment with demo data in one go, fund a key with testnet USDC from [faucet.circle.com](https://faucet.circle.com) and run:

```bash
CONTRACT=$(scripts/deploy-testnet.sh <your-key>)
scripts/seed-testnet.sh "$CONTRACT" <your-key>
```

## Making a contribution

1. Fork the repository and create a branch named after the issue: `issue-42-ticket-transfer`.
2. Write your code. Keep changes focused on the issue — don't refactor unrelated things in the same PR.
3. Add or update tests. Every new function needs a test that covers the happy path and at least one failure case.
4. Run `cargo test` and make sure everything passes.
5. Open a pull request against `main`. Fill in the PR description: what changed, why, and how you tested it.

## Code standards

- This is a `no_std` Soroban contract. Don't introduce `std`-only dependencies.
- Storage: use `persistent` for per-event data, `instance` for contract-wide config. Write persistent entries through the `save` helper so their TTL is extended — a raw `set` can let the entry archive.
- Events: every state-changing function publishes a `#[contractevent]` (see the Events table in the README). Add one for each new state change and assert it in a test.
- Auth: any function that changes state on behalf of a user must call `address.require_auth()` at the top.
- Error handling: return `Err(Error::...)` from the `Error` enum in `src/lib.rs` — never `panic!`. Panic messages are stripped from release builds, so clients only ever see the numeric error code. Add new variants at the end with the next number, and never renumber or reuse an existing code.
- Comments: only add a comment when the *why* is non-obvious. Don't describe what the code does — the code does that.

## AI-assisted contributions

You may use AI tools to help write or understand code. However:

- You are responsible for every line you submit. Review and understand all AI-generated code before including it in a PR.
- Submitting unreviewed AI output — code you can't explain or defend — is grounds for closing the PR.
- If a reviewer asks you to explain a section of your PR, you should be able to do so.

## Amounts and precision

USDC on Stellar uses 7 decimal places. `1 USDC = 10_000_000 stroops`. Use stroops (`i128`) everywhere inside the contract; convert only at the boundary where you display or input values.

## Questions

Open an issue or leave a comment on the relevant issue thread. Don't open a PR without prior discussion for anything beyond a small, clearly scoped bug fix.
