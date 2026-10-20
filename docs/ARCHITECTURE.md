# Architecture and security model

This document explains how the Inowo contract holds and moves money, what it guarantees, and — just as important — what it does **not** guarantee yet. It describes the code in `src/lib.rs`; if the two disagree, the code is right and this document has a bug.

> The contract has not been audited and is deployed on Stellar **testnet** only.

## The idea in one paragraph

All USDC for an event lives in the contract, not in the organizer's wallet. Money enters through two functions (`sponsor_event`, `buy_ticket`) and leaves through three (`release_funds`, `refund_ticket`, `refund_sponsorship`). Each event has its own balance, and every movement is recorded on-chain and published as an event.

## Event lifecycle

```
              end_event (organizer)
   ┌────────┐ ───────────────────────────▶ ┌───────┐
   │ Active │                              │ Ended │   release_funds
   └────────┘ ───────────────────┐         └───────┘   (organizer)
        create_event             │ cancel_event
                                 ▼ (organizer)
                            ┌───────────┐
                            │ Cancelled │   refund_ticket / refund_sponsorship
                            └───────────┘   (each claimant)
```

`Ended` and `Cancelled` are final: there is no way back to `Active`, and an ended event cannot be cancelled (or the reverse).

| Function | Who | `Active` | `Ended` | `Cancelled` |
|---|---|:-:|:-:|:-:|
| `sponsor_event` | anyone | ✅ | ❌ | ❌ |
| `buy_ticket` | anyone | ✅ | ❌ | ❌ |
| `redeem_ticket` | organizer | ✅ | ✅ | ❌ |
| `end_event` | organizer | ✅ | ❌ | ❌ |
| `cancel_event` | organizer | ✅ | ❌ | ❌ |
| `release_funds` | organizer | ❌ | ✅ | ❌ |
| `refund_ticket` | ticket owner | ❌ | ❌ | ✅ |
| `refund_sponsorship` | sponsor | ❌ | ❌ | ✅ |

The two money-out paths are deliberately exclusive: **funds can only be released from an `Ended` event and only refunded from a `Cancelled` one.** An event can't do both, so cancelling always leaves everything that was collected available for refunds.

## Where money can move

| Direction | Function | Amount | Bookkeeping |
|---|---|---|---|
| in | `sponsor_event` | caller's choice, > 0 | `balance += amount`, sponsor's running total `+= amount` |
| in | `buy_ticket` | the tier's price | `balance += price`, ticket records `price_paid` |
| out | `release_funds` | organizer's choice, > 0 and ≤ `balance` | `balance -= amount`, payout recorded with a memo |
| out | `refund_ticket` | the ticket's `price_paid` | `balance -= price_paid`, ticket marked `refunded` |
| out | `refund_sponsorship` | the sponsor's running total | `balance -= total`, running total reset to 0 |

The key invariant, which the tests check:

> For every event, `balance = sponsored + ticket sales − released − refunded`, and `balance ≥ 0`. The contract's token balance equals the sum of all event balances.

Two design choices keep this safe:

- **Refunds are pulled, not pushed.** Each ticket holder or sponsor claims their own refund. Cancelling therefore never loops over every participant, so it can't run out of resources on a large event and strand funds.
- **State is updated before tokens leave.** `release_funds` and the refund functions write the new balance, then call the token. A failed transfer aborts the whole invocation, so state is never half-updated, and Soroban doesn't allow a contract to be re-entered while it is running.

## Authorization

Every state-changing function calls `require_auth()` on the acting address first. Organizer-only functions then compare it to the `organizer` stored on the event (`Error::NotOrganizer`); refund functions compare against the ticket owner or the sponsor's recorded total. The organizer is fixed when the event is created and cannot be transferred.

There is **no admin, owner, pause switch, or upgrade function.** Nobody — including the deployer — can change the contract's code, move funds out of band, or freeze an event.

## Storage

| Key | Storage | Holds |
|---|---|---|
| `Token`, `EventCounter` | instance | the USDC token address (set once, in the constructor) and the number of events |
| `Event(id)` | persistent | organizer, metadata, goal, escrow `balance`, `status` |
| `Tiers(id)` | persistent | tiers with price, cap, and tickets sold |
| `TicketCounter(id)`, `Ticket(id, n)` | persistent | one entry per ticket: owner, tier, `price_paid`, `redeemed`, `refunded` |
| `SponsorshipCount(id)`, `Sponsorship(id, n)` | persistent | one entry per contribution, so a flood of tiny sponsorships can't grow a single entry past Soroban's size limit |
| `SponsorTotal(id, sponsor)` | persistent | what each sponsor is owed if the event is cancelled |
| `PayoutCounter(id)`, `Payout(id, n)`, `TotalReleased(id)` | persistent | one entry per payout: recipient, amount, memo, ledger time |

Soroban archives storage entries whose TTL runs out. Every write goes through the `save` helper, which extends the written entry to 120 days when fewer than 30 remain, and the instance is extended the same way. Read-only calls do not extend anything, so an event nobody has written to for 120 days can be archived (restorable, but needs a restore transaction).

## What the contract publishes

Every state change emits a contract event (see the Events table in the README), so an indexer can reconstruct an event's complete money flow without polling storage. Failed calls emit nothing.

## Trust assumptions

- **The token.** The contract trusts whichever token address it was deployed with. On testnet that's Circle's testnet USDC. A Stellar asset issuer can freeze a trustline or claw funds back; if that happens to an account or to the contract, transfers involving it fail. Inowo inherits those powers of the issuer.
- **The organizer.** See below — today this is the largest assumption.
- **The network.** Standard Stellar and Soroban assumptions.

## Known limitations

These are real, they are tracked as issues, and a few are the project's most important next steps.

1. **The organizer decides who gets paid.** After an event ends, `release_funds` can pay *any* address up to the escrow balance. An organizer can also end an event immediately after collecting sponsorships — even before its date — and release everything to themselves. Nothing today ties payouts to a promised budget. The fix is **published budget lines** that sponsors see before paying and that `release_funds` must follow ([#32](https://github.com/inowo-labs/inowo-Contract/issues/32)), supported by a **funding deadline** ([#31](https://github.com/inowo-labs/inowo-Contract/issues/31)) and **proof of spend** ([#33](https://github.com/inowo-labs/inowo-Contract/issues/33)). Until then, sponsors should treat an event as trust in its organizer, and the public payout list with memos is the accountability mechanism.
2. **The funding goal does nothing.** It's stored and shown, but missing it has no effect: no deadline, no automatic cancellation ([#31](https://github.com/inowo-labs/inowo-Contract/issues/31)).
3. **Money left in an ended event stays there.** After `end_event`, nothing can be refunded, and funds the organizer never releases remain in escrow indefinitely.
4. **Tokens sent straight to the contract are lost.** Only `sponsor_event` and `buy_ticket` credit an event. USDC transferred to the contract address directly is not attributed to any event and cannot be recovered.
5. **Event dates aren't validated.** `create_event` accepts a date in the past ([#37](https://github.com/inowo-labs/inowo-Contract/issues/37)).
6. **Tickets can't be looked up by owner.** `transfer_ticket` lets an owner hand off a ticket, but there's no index to find "all tickets owned by this address" — a caller has to already know a ticket's ID ([#35](https://github.com/inowo-labs/inowo-Contract/issues/35)).

## Testing

`src/test.rs` covers every function's happy path and every error code, asserting the *exact* `Error` variant rather than just failure, plus the emitted events, storage TTL extension, and the conservation of funds across cancellation and release. When you change how money moves, extend those tests in the same PR and keep this document in step.

## Reporting a vulnerability

If you find a way to lock, steal, or miscount funds, please don't post the details in a public issue. Open an issue that says only that you have a security report and ask the maintainers for a private channel, and we'll arrange one.
