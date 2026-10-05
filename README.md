# Inowo

> Sponsorship escrow and accountable event budgets on Stellar.

Inowo is a Soroban smart contract that holds event funding in escrow and releases it by rules everyone can see. Sponsors know where their money is going before they pay. Workers and vendors are paid on time. If an event falls through, everyone is refunded.

## The problem

Most community events — meetups, hackathons, campus and church events — run on sponsorship. Today that money moves through private bank transfers:

- **Sponsors pay blind.** They rarely see the budget, what other sponsors gave, or how the money was spent.
- **Workers and vendors chase payment.** Staff, caterers, and sound crews are often paid late, partially, or not at all.
- **Cancelled events strand money.** Getting a refund depends on the organizer's goodwill.
- **Good organizers can't prove it.** An organizer who has run ten events honestly has no record to show the next sponsor.

## How Inowo works

1. **Create** — An organizer creates an event with a funding goal and one or more ticket tiers.
2. **Fund** — Sponsors contribute USDC. Every contribution is recorded on-chain against the sponsor's address. Ticket sales flow into the same escrow.
3. **Hold** — Funds sit in the contract, not in the organizer's wallet.
4. **Release** — After the event, funds are paid out to named recipients through the contract, so every payout sits on the same ledger as every contribution. *(planned)*
5. **Refund** — If the event is cancelled, sponsors and ticket holders are refunded by the contract. *(planned)*

Over time, every event an organizer completes becomes a public track record: funds raised, funds released, refunds paid. That record is the organizer's reason to use Inowo — it's what earns sponsor trust for the next event.

## Why Stellar

- **USDC is native.** Sponsorships, tickets, and payouts settle in a stable currency, not a volatile token.
- **Fees are fractions of a cent.** Paying ten workers small amounts is practical.
- **Anchors connect to local money.** Workers and vendors can cash out to local currency through Stellar anchors, and sponsors can fund from a bank account.
- **Soroban** gives us auditable, deterministic contract logic for escrow and release rules.

## Status

Inowo is in early development on Stellar testnet. Here is exactly what exists today and what is open for contribution.

### Implemented

| Feature | Description |
|---------|-------------|
| Event creation | Organizer creates an event with a funding goal and multiple ticket tiers (price + supply cap) |
| Sponsorship | Sponsors contribute USDC; each contribution is recorded publicly |
| Ticket purchase | Attendees buy tickets in USDC; each ticket is an on-chain ownership record |
| Check-in | Organizer redeems a ticket at the door; a ticket can only be redeemed once |
| End event | Organizer closes an event, stopping further sales and sponsorships |
| Escrow | All USDC is held by the contract, tracked per event |
| Read access | Anyone can query events, tiers, tickets, sponsorships, and balances |

### Planned — open for contribution

| Feature | Issue |
|---------|-------|
| Release funds to recipients after an event ends | [#2](https://github.com/inowo-labs/inowo-Contract/issues/2) |
| Cancel an event with automatic refunds | [#1](https://github.com/inowo-labs/inowo-Contract/issues/1) |
| Ticket transfer between holders | [#3](https://github.com/inowo-labs/inowo-Contract/issues/3) |
| TypeScript bindings for the contract | [#4](https://github.com/inowo-labs/inowo-Contract/issues/4) |
| Funding deadline — refund sponsors if the goal is not met | — |
| Budget lines — organizer publishes planned spending per recipient before funding opens | — |
| Proof of spend — attach a receipt hash to each payout | — |
| Organizer track record — on-chain history of events delivered and funds released | — |
| Contract events for indexers | — |

## Project structure

Inowo is three repositories:

| Repository | Description |
|------------|-------------|
| [inowo-Contract](https://github.com/inowo-labs/inowo-Contract) | Soroban smart contract (Rust) — the source of truth for all funds and records |
| [inowo-api](https://github.com/inowo-labs/inowo-api) | Read API (Node.js, Express) that queries the contract for clients |
| [inowo-app](https://github.com/inowo-labs/inowo-app) | Web frontend (Next.js) for organizers, sponsors, and attendees |

All writes — creating events, sponsoring, buying tickets — are signed by the user's wallet and go directly to the contract. The API is read-only.

## Roles

| Role | Can do today | Planned |
|------|--------------|---------|
| Organizer | Create events, define tiers, check in tickets, end events | Release funds, cancel with refunds, publish budget lines |
| Sponsor | Contribute USDC; view every sponsorship for an event | Automatic refund if the event is cancelled or the goal is missed |
| Attendee | Buy tickets; prove ownership on-chain | Transfer tickets; refund on cancellation |
| Recipient | — | Receive payouts from the event escrow |
| Anyone | Read all events, tickets, sponsorships, and balances | Read payout history and organizer track records |

## Data model

- **Event** — organizer, name, description, venue, date, funding goal, escrowed balance, and status (`Active`, `Ended`, `Cancelled`).
- **Ticket tier** — a named price level (e.g. General, VIP) with a price and supply cap.
- **Ticket** — an ownership record linking a buyer's address to an event and tier, with a redeemed flag.
- **Sponsorship** — a contribution recorded against the sponsor's address.

All amounts are in USDC stroops: `1 USDC = 10_000_000`.

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) with the `wasm32v1-none` target
- [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup) v26+
- A funded Stellar testnet account (`stellar keys generate --network testnet <name>`)

```bash
rustup target add wasm32v1-none
```

### Build

```bash
cargo build --target wasm32v1-none --release
```

The compiled contract is at `target/wasm32v1-none/release/inowo_contract.wasm`.

### Test

```bash
cargo test
```

### Deploy to testnet

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/inowo_contract.wasm \
  --network testnet \
  --source <your-key-name>
```

### Testnet deployment

| | Address |
|---|---|
| Contract | `CABTSQOXHOOAFFWBPDIXAPAL7KKV76WFL3WLGBUH6SLJ7R2BO5YNWKFU` |
| USDC token | `CAUJTFVKA5WCN4ZPUDBRDAS3DT5HVKNQTLFT32KDAFVGJRTB7VPRVNRT` |

## Function reference

### Write functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `initialize` | `token: Address` | — | One-time setup; records the USDC token contract address |
| `create_event` | `organizer: Address, name: String, description: String, venue: String, date_unix: u64, funding_goal: i128, tiers: Vec<TierInput>` | `u32` (event ID) | Creates an event with one or more ticket tiers |
| `sponsor_event` | `sponsor: Address, event_id: u32, amount: i128` | — | Contributes USDC to an event's escrow |
| `buy_ticket` | `buyer: Address, event_id: u32, tier_index: u32` | `u32` (ticket ID) | Buys a ticket, paying the tier price in USDC |
| `redeem_ticket` | `organizer: Address, event_id: u32, ticket_id: u32` | — | Checks in a ticket at the door |
| `end_event` | `organizer: Address, event_id: u32` | — | Closes an event (`Active` → `Ended`); blocks further sales and sponsorships |

### Read functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `get_event` | `event_id: u32` | `Event` | Full event record |
| `get_tiers` | `event_id: u32` | `Vec<TicketTier>` | Ticket tiers with live sales counts |
| `get_ticket` | `event_id: u32, ticket_id: u32` | `Ticket` | A single ticket's ownership record |
| `get_sponsorships` | `event_id: u32` | `Vec<Sponsorship>` | All sponsorships for an event |
| `get_balance` | `event_id: u32` | `i128` | USDC held in escrow for an event |
| `get_organizer` | `event_id: u32` | `Address` | The event's organizer |
| `get_token` | — | `Address` | The configured USDC token address |
| `event_count` | — | `u32` | Total events created |
| `ticket_count` | `event_id: u32` | `u32` | Tickets sold for an event |
| `sponsor_count` | `event_id: u32` | `u32` | Sponsorship contributions for an event |
| `tier_count` | `event_id: u32` | `u32` | Ticket tiers for an event |

## Contributing

Inowo is open to developers, designers, and product builders. Read [CONTRIBUTING.md](./CONTRIBUTING.md), then pick an issue from the **Planned** table above or the [Issues](https://github.com/inowo-labs/inowo-Contract/issues) tab. Each issue describes what "done" looks like.

## License

[MIT](./LICENSE)
