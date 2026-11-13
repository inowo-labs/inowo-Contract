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

1. **Create** — An organizer creates an event with a funding goal and a funding deadline, then publishes a **budget**: who gets paid, how much, and for what — before anyone has paid in.
2. **Fund** — Sponsors contribute USDC, seeing the budget first. Every contribution is recorded on-chain against the sponsor's address. Ticket sales flow into the same escrow.
3. **Hold** — Funds sit in the contract, not in the organizer's wallet.
4. **Release** — After the event ends, the organizer releases funds against a budget line — never more than that line's amount, never to anyone the budget didn't name. Each payout sits on the same ledger as every contribution.
5. **Refund** — If the event is cancelled — by the organizer, or by anyone once the funding deadline passes with the goal unmet — every sponsor and ticket holder can claim their money back from the contract in full.

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
| Event creation | Organizer creates an event with a funding goal, a funding deadline, and multiple ticket tiers (price + supply cap) |
| Sponsorship | Sponsors contribute USDC; each contribution is recorded publicly |
| Ticket purchase | Attendees buy tickets in USDC; each ticket is an on-chain ownership record |
| Check-in | Organizer redeems a ticket at the door; a ticket can only be redeemed once |
| Ticket transfer | The owner of an unused ticket can transfer it to another address while the event is active; the new owner inherits check-in and the right to a refund |
| End event | Organizer closes an event, stopping further sales and sponsorships |
| Cancel and refund | Organizer cancels an active event; each ticket holder and sponsor claims a full refund from escrow |
| Escrow | All USDC is held by the contract, tracked per event |
| Contract events | Every state change is published as an on-chain event for indexers |
| Budget lines | Organizer publishes planned spending (recipient, amount, purpose) before anyone funds the event; `release_funds` can only pay against a line, never more than it budgeted — every event needs at least one line before it can release anything |
| Release funds | After an event ends, the organizer releases funds against a budget line; each payout is a public record with the line's purpose as its memo |
| Proof of spend | Organizer attaches a SHA-256 receipt hash (and optional URI) to a payout, once, as evidence of what it paid for |
| Read access | Anyone can query events, tiers, tickets, sponsorships, sponsor totals, balances, and payouts |
| TypeScript bindings | A generated, typed client in [`bindings/`](./bindings) — every argument, return type, and error code matches the contract, regenerated with `scripts/generate-bindings.sh` |
| Organizer track record | On-chain stats per organizer (events created/ended/cancelled, total raised/released/refunded) and a paginated list of every event they've created |
| Funding deadline | Every event has a deadline; if the funding goal isn't met by then, anyone can call `expire_event` to cancel it and open refunds — no organizer action needed |

### Planned — open for contribution

Nothing right now — check the [Issues](https://github.com/inowo-labs/inowo-Contract/issues) tab for what's next.

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
| Organizer | Create events, define tiers, publish a budget, check in tickets, end or cancel events, release funds against a budget line | — |
| Sponsor | Contribute USDC; view every sponsorship for an event; claim a full refund if the event is cancelled or expires unfunded | — |
| Attendee | Buy tickets; prove ownership on-chain; claim a refund if the event is cancelled; transfer an unused ticket to someone else | — |
| Recipient | Receive payouts from an event's escrow, each with a public memo, and have the organizer attach proof of spend | — |
| Anyone | Read all events, tickets, sponsorships, balances, payout history, and every organizer's track record; expire an event that missed its funding deadline | — |

## Data model

- **Event** — organizer, name, description, venue, date, funding goal, funding deadline, escrowed balance, and status (`Active`, `Ended`, `Cancelled`).
- **Ticket tier** — a named price level (e.g. General, VIP) with a price and supply cap.
- **Ticket** — an ownership record linking a buyer's address to an event and tier, with the price paid and redeemed / refunded flags.
- **Sponsorship** — a contribution recorded against the sponsor's address.
- **BudgetLine** — a planned disbursement: recipient, amount, purpose, and how much of it has been released so far.
- **Payout** — a release from escrow: recipient, amount, the budget line it was released against, memo, timestamp, and an optional receipt hash + URI once the organizer attaches proof of spend.
- **OrganizerStats** — per organizer: events created/ended/cancelled, and total USDC raised/released/refunded across every event they've run.

All amounts are in USDC stroops: `1 USDC = 10_000_000`.

For how money moves, what the contract guarantees, and its known limitations, read [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md).

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
  --source <your-key-name> \
  -- \
  --token <usdc-token-contract-id>
```

The token address is passed to the contract's constructor, which runs atomically with deployment.

Or use the scripts, which build, deploy with Circle's testnet USDC, and seed demo data:

```bash
scripts/deploy-testnet.sh [key-alias]                # prints the new contract ID
scripts/seed-testnet.sh <contract-id> [key-alias]    # key must hold ≥ 18.5 testnet USDC
```

### Testnet deployment

| | Address |
|---|---|
| Contract | [`CCWFDV2MIDV7QOEUJNZDFRRJY75O7S65JPVJ2S2WNARJ4INEMDTCNOPV`](https://stellar.expert/explorer/testnet/contract/CCWFDV2MIDV7QOEUJNZDFRRJY75O7S65JPVJ2S2WNARJ4INEMDTCNOPV) |
| USDC token | `CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA` — Circle's testnet USDC |

Get testnet USDC for your own account from [faucet.circle.com](https://faucet.circle.com) (choose Stellar), after adding a USDC trustline to issuer `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`.

The deployment is seeded with three demo events covering the full lifecycle:

| ID | Event | Status | What you'll find |
|----|-------|--------|------------------|
| 0 | Lagos Stellar Builders Meetup | `Active` | Two sponsorships and two tickets — open for you to sponsor or buy |
| 1 | Soroban Smart Contract Workshop | `Ended` | A checked-in ticket and two payouts, to a venue and a crew, each with a memo |
| 2 | Campus Hack Night | `Cancelled` | A sponsorship and a ticket, both refunded in full |

## TypeScript bindings

[`bindings/`](./bindings) is a generated, typed TypeScript client — every function, type, and error code is produced straight from the contract's spec, so a client-side call can't drift out of sync with an argument order or type the contract actually expects.

```bash
./scripts/generate-bindings.sh   # regenerates bindings/src/index.ts from the current contract
```

Never hand-edit `bindings/src/index.ts`; CI regenerates it and fails if the committed output doesn't match. See [`bindings/README.md`](./bindings/README.md) for how to install it and call a read and a write function.

## Function reference

### Write functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `__constructor` | `token: Address` | — | Runs once at deployment; records the USDC token contract address |
| `create_event` | `organizer: Address, name: String, description: String, venue: String, date_unix: u64, funding_goal: i128, funding_deadline: u64, tiers: Vec<TierInput>` | `u32` (event ID) | Creates an event with one or more ticket tiers; `funding_deadline` must be in the future and on or before `date_unix` |
| `sponsor_event` | `sponsor: Address, event_id: u32, amount: i128` | — | Contributes USDC to an event's escrow |
| `buy_ticket` | `buyer: Address, event_id: u32, tier_index: u32` | `u32` (ticket ID) | Buys a ticket, paying the tier price in USDC |
| `transfer_ticket` | `from: Address, event_id: u32, ticket_id: u32, to: Address` | — | Current owner hands an unused ticket to another address (active events only); refunds follow the ticket |
| `redeem_ticket` | `organizer: Address, event_id: u32, ticket_id: u32` | — | Checks in a ticket at the door |
| `end_event` | `organizer: Address, event_id: u32` | — | Closes an event (`Active` → `Ended`); blocks further sales and sponsorships |
| `cancel_event` | `organizer: Address, event_id: u32` | — | Cancels an event (`Active` → `Cancelled`); blocks sales, sponsorships, and check-in, and opens refunds |
| `expire_event` | `event_id: u32` | — | Callable by anyone, no auth required: cancels an `Active` event once its `funding_deadline` has passed with the goal unmet, opening refunds the same as `cancel_event` |
| `refund_ticket` | `owner: Address, event_id: u32, ticket_id: u32` | `i128` (amount) | Ticket owner claims back the price paid for a ticket to a cancelled event |
| `refund_sponsorship` | `sponsor: Address, event_id: u32` | `i128` (amount) | Sponsor claims back their total contribution to a cancelled event |
| `add_budget_line` | `organizer: Address, event_id: u32, recipient: Address, amount: i128, purpose: String` | `u32` (line ID) | Publishes a planned disbursement; only while `Active` and before the event's first sponsorship or ticket sale |
| `release_funds` | `organizer: Address, event_id: u32, line_id: u32, amount: i128` | `u32` (payout ID) | After an event ends, pays the line's recipient from escrow; never more than the line's remaining amount. Memo is the line's purpose |
| `attach_receipt` | `organizer: Address, event_id: u32, payout_id: u32, receipt_hash: BytesN<32>, receipt_uri: Option<String>` | — | Attaches a SHA-256 receipt hash (and optionally where to find the document, e.g. an IPFS CID, up to 200 bytes) to a payout as proof of spend; settable only once per payout |

### Read functions

| Function | Parameters | Returns | Description |
|----------|------------|---------|-------------|
| `get_event` | `event_id: u32` | `Event` | Full event record |
| `get_tiers` | `event_id: u32` | `Vec<TicketTier>` | Ticket tiers with live sales counts |
| `get_ticket` | `event_id: u32, ticket_id: u32` | `Ticket` | A single ticket's ownership record |
| `get_tickets_by_owner` | `event_id: u32, owner: Address, start: u32, limit: u32` | `Vec<u32>` | Ticket IDs `owner` has bought or received by transfer; a history, not a live "currently owned" list (check `get_ticket(...).owner` against `owner` to find out which are still current); capped at `MAX_OWNER_TICKET_PAGE` (50) |
| `owner_ticket_count` | `event_id: u32, owner: Address` | `u32` | How many tickets `owner` has ever bought or received by transfer for the event |
| `get_sponsorships` | `event_id: u32, start: u32, limit: u32` | `Vec<Sponsorship>` | A page of sponsorships, ordered by contribution time; `limit` is capped at `MAX_SPONSORSHIP_PAGE` (50) |
| `get_sponsor_total` | `event_id: u32, sponsor: Address` | `i128` | A sponsor's total contribution not yet refunded |
| `get_balance` | `event_id: u32` | `i128` | USDC held in escrow for an event |
| `get_organizer` | `event_id: u32` | `Address` | The event's organizer |
| `get_payout` | `event_id: u32, payout_id: u32` | `Payout` | A single payout record, including which budget line it was released against and its receipt hash/URI if one has been attached |
| `payout_count` | `event_id: u32` | `u32` | Payouts released for an event |
| `get_budget_line` | `event_id: u32, line_id: u32` | `BudgetLine` | A single budget line, including how much has been released against it |
| `budget_line_count` | `event_id: u32` | `u32` | Budget lines published for an event |
| `total_released` | `event_id: u32` | `i128` | Total USDC released from an event's escrow |
| `get_token` | — | `Address` | The configured USDC token address |
| `event_count` | — | `u32` | Total events created |
| `ticket_count` | `event_id: u32` | `u32` | Tickets sold for an event |
| `sponsor_count` | `event_id: u32` | `u32` | Sponsorship contributions for an event |
| `tier_count` | `event_id: u32` | `u32` | Ticket tiers for an event |
| `get_organizer_stats` | `organizer: Address` | `OrganizerStats` | An organizer's on-chain track record; all zeros for an address that has never organized anything |
| `get_organizer_events` | `organizer: Address, start: u32, limit: u32` | `Vec<u32>` | Event IDs `organizer` has ever created, oldest first; capped at `MAX_ORGANIZER_EVENTS_PAGE` (50) |

### Events

Every state change publishes a contract event, so indexers can follow an event's full money flow without polling storage. The first topic is the event name; fields marked *topic* follow it, and the rest are in the data map.

| Event | Topics | Data | Published by |
|-------|--------|------|--------------|
| `event_created` | `event_id`, `organizer` | `funding_goal`, `date_unix` | `create_event` |
| `sponsored` | `event_id`, `sponsor` | `amount` | `sponsor_event` |
| `ticket_purchased` | `event_id`, `buyer` | `ticket_id`, `tier_index`, `price` | `buy_ticket` |
| `ticket_transferred` | `event_id`, `from` | `to`, `ticket_id` | `transfer_ticket` |
| `ticket_redeemed` | `event_id` | `ticket_id` | `redeem_ticket` |
| `event_ended` | `event_id` | `balance` | `end_event` |
| `event_cancelled` | `event_id` | `balance` | `cancel_event` |
| `event_expired` | `event_id` | `balance` | `expire_event` |
| `ticket_refunded` | `event_id`, `owner` | `ticket_id`, `amount` | `refund_ticket` |
| `sponsorship_refunded` | `event_id`, `sponsor` | `amount` | `refund_sponsorship` |
| `funds_released` | `event_id`, `recipient` | `payout_id`, `line_id`, `amount`, `memo` | `release_funds` |
| `budget_line_added` | `event_id` | `line_id`, `recipient`, `amount`, `purpose` | `add_budget_line` |
| `receipt_attached` | `event_id` | `payout_id`, `receipt_hash` | `attach_receipt` |

### Storage lifetime

Soroban archives storage entries whose TTL runs out. Every write extends the entry it touches, and the contract instance, to 120 days whenever fewer than 30 days remain, so active events stay live.

### Verifying a receipt

A payout's `receipt_hash` is a SHA-256 digest, not the document itself — it proves a document matches without exposing it on-chain. To check a receipt against a payout:

```sh
sha256sum receipt.pdf
```

Compare the hex digest to `get_payout(event_id, payout_id).receipt_hash`. If `receipt_uri` is set, fetch the document from there first (e.g. `ipfs://...`) before hashing it.

### Errors

Every function that can fail returns a typed error. On-chain, clients receive it as `Error(Contract, #<code>)`. Codes are stable and never reused.

| Code | Error | Returned when |
|------|-------|---------------|
| 1 | `AlreadyInitialized` | Reserved — setup happens in the constructor, which cannot run twice |
| 2 | `NotInitialized` | No token is configured (cannot occur on a correctly deployed contract) |
| 3 | `EventNotFound` | No event exists with the given ID |
| 4 | `TicketNotFound` | No ticket exists with the given ID for the event |
| 5 | `NotOrganizer` | The caller is not the event's organizer |
| 6 | `EventNotActive` | The event has ended or been cancelled |
| 7 | `NoTiers` | `create_event` was called with no ticket tiers |
| 8 | `InvalidFundingGoal` | The funding goal is zero or negative |
| 9 | `InvalidTierPrice` | A tier price is zero or negative |
| 10 | `InvalidSupplyCap` | A tier supply cap is zero |
| 11 | `InvalidTier` | The tier index is out of range |
| 12 | `TierSoldOut` | The tier has no tickets left |
| 13 | `AlreadyRedeemed` | The ticket has already been checked in |
| 14 | `InvalidAmount` | A sponsorship amount is zero or negative |
| 15 | `EventNotCancelled` | A refund was requested for an event that is not cancelled |
| 16 | `NotTicketOwner` | The caller does not own the ticket |
| 17 | `AlreadyRefunded` | The ticket has already been refunded |
| 18 | `NothingToRefund` | The caller has no unrefunded sponsorship for the event |
| 19 | `EventNotEnded` | Funds can only be released after the event has ended |
| 20 | `InsufficientFunds` | The release amount exceeds the event's escrowed balance |
| 21 | `InvalidMemo` | The payout memo is empty or longer than 200 bytes |
| 22 | `PayoutNotFound` | No payout exists with the given ID for the event |
| 23 | `TransferToSelf` | A ticket cannot be transferred to its current owner |
| 24 | `ReceiptAlreadyAttached` | The payout already has a receipt hash attached |
| 25 | `InvalidReceiptUri` | The receipt URI is longer than 200 bytes |
| 26 | `InvalidFundingDeadline` | The funding deadline isn't in the future, or is after the event date |
| 27 | `FundingDeadlineNotPassed` | `expire_event` was called before the funding deadline passed |
| 28 | `FundingGoalReached` | `expire_event` was called on an event that already met its funding goal |
| 29 | `BudgetLocked` | `add_budget_line` was called after the event's first sponsorship or ticket sale |
| 30 | `BudgetLineNotFound` | No budget line exists with the given ID for the event |
| 31 | `BudgetLineExceeded` | The release would push the line's total past its budgeted amount |

## Contributing

Inowo is open to developers, designers, and product builders. Read [CONTRIBUTING.md](./CONTRIBUTING.md), then pick an issue from the **Planned** table above or the [Issues](https://github.com/inowo-labs/inowo-Contract/issues) tab. Each issue describes what "done" looks like.

## License

[MIT](./LICENSE)
