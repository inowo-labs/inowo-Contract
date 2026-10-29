import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}




/**
 * Error codes are part of the contract's public interface — clients match on
 * the numeric value, so never renumber or reuse an existing code.
 */
export const Errors = {
  /**
   * Reserved: setup now happens in the constructor, which cannot run twice.
   */
  1: {message:"AlreadyInitialized"},
  2: {message:"NotInitialized"},
  3: {message:"EventNotFound"},
  4: {message:"TicketNotFound"},
  5: {message:"NotOrganizer"},
  6: {message:"EventNotActive"},
  7: {message:"NoTiers"},
  8: {message:"InvalidFundingGoal"},
  9: {message:"InvalidTierPrice"},
  10: {message:"InvalidSupplyCap"},
  11: {message:"InvalidTier"},
  12: {message:"TierSoldOut"},
  13: {message:"AlreadyRedeemed"},
  14: {message:"InvalidAmount"},
  15: {message:"EventNotCancelled"},
  16: {message:"NotTicketOwner"},
  17: {message:"AlreadyRefunded"},
  18: {message:"NothingToRefund"},
  19: {message:"EventNotEnded"},
  20: {message:"InsufficientFunds"},
  21: {message:"InvalidMemo"},
  22: {message:"PayoutNotFound"},
  23: {message:"TransferToSelf"},
  24: {message:"ReceiptAlreadyAttached"},
  25: {message:"InvalidReceiptUri"}
}


export interface Event {
  /**
 * Current USDC balance held by the contract for this event.
 */
balance: i128;
  /**
 * Unix timestamp of the event date.
 */
date_unix: u64;
  description: string;
  /**
 * Funding goal in USDC stroops.
 */
funding_goal: i128;
  name: string;
  organizer: string;
  status: EventStatus;
  venue: string;
}


/**
 * A public record of funds released from an event's escrow.
 */
export interface Payout {
  amount: i128;
  /**
 * What the payout is for, e.g. "Sound crew" — required so every
 * disbursement explains itself next to the contributions it spends.
 */
memo: string;
  /**
 * SHA-256 of a receipt document proving what the payout was spent on.
 * Settable once via `attach_receipt`, after the payout is made.
 */
receipt_hash: Option<Buffer>;
  /**
 * Where to find the receipt document itself, e.g. an IPFS CID. The hash
 * above is what proves it matches; this is just how to find it.
 */
receipt_uri: Option<string>;
  recipient: string;
  /**
 * Ledger timestamp of the release.
 */
timestamp: u64;
}


/**
 * On-chain proof of ticket ownership.
 */
export interface Ticket {
  event_id: u32;
  owner: string;
  /**
 * USDC stroops paid, returned in full if the event is cancelled.
 */
price_paid: i128;
  redeemed: boolean;
  refunded: boolean;
  tier_index: u32;
}

export type DataKey = {tag: "Token", values: void} | {tag: "EventCounter", values: void} | {tag: "Event", values: readonly [u32]} | {tag: "Tiers", values: readonly [u32]} | {tag: "TicketCounter", values: readonly [u32]} | {tag: "Ticket", values: readonly [u32, u32]} | {tag: "SponsorshipCount", values: readonly [u32]} | {tag: "Sponsorship", values: readonly [u32, u32]} | {tag: "SponsorTotal", values: readonly [u32, string]} | {tag: "PayoutCounter", values: readonly [u32]} | {tag: "Payout", values: readonly [u32, u32]} | {tag: "TotalReleased", values: readonly [u32]} | {tag: "OwnerTicketCount", values: readonly [u32, string]} | {tag: "OwnerTicket", values: readonly [u32, string, u32]};


/**
 * Input shape for a ticket tier when creating an event.
 */
export interface TierInput {
  name: string;
  /**
 * Price in USDC stroops (1 USDC = 10_000_000).
 */
price: i128;
  supply_cap: u32;
}



/**
 * Stored state for a ticket tier, extended with live sales count.
 */
export interface TicketTier {
  name: string;
  price: i128;
  supply_cap: u32;
  tickets_sold: u32;
}


export type EventStatus = {tag: "Active", values: void} | {tag: "Ended", values: void} | {tag: "Cancelled", values: void};


/**
 * A public sponsorship contribution record.
 */
export interface Sponsorship {
  amount: i128;
  sponsor: string;
}










export interface Client {
  /**
   * Construct and simulate a end_event transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer closes an event, preventing further ticket sales and sponsorships.
   * Status transitions from Active → Ended.
   */
  end_event: ({organizer, event_id}: {organizer: string, event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_event transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_event: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Event>>>

  /**
   * Construct and simulate a get_tiers transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_tiers: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Array<TicketTier>>>>

  /**
   * Construct and simulate a get_token transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the token contract address configured at deployment.
   */
  get_token: (options?: MethodOptions) => Promise<AssembledTransaction<Result<string>>>

  /**
   * Construct and simulate a buy_ticket transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Buyer purchases a ticket in a given tier.
   * Transfers `tier.price` USDC from buyer to this contract.
   * Returns the new ticket ID.
   */
  buy_ticket: ({buyer, event_id, tier_index}: {buyer: string, event_id: u32, tier_index: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a get_payout transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_payout: ({event_id, payout_id}: {event_id: u32, payout_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Payout>>>

  /**
   * Construct and simulate a get_ticket transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_ticket: ({event_id, ticket_id}: {event_id: u32, ticket_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Ticket>>>

  /**
   * Construct and simulate a tier_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the number of ticket tiers for an event.
   */
  tier_count: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a event_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  event_count: (options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a get_balance transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_balance: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a cancel_event transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer cancels an active event (Active → Cancelled). Sales and
   * sponsorships stop, and every ticket holder and sponsor can claim a
   * full refund. Funds can only be released after an event *ends*, so a
   * cancelled event always still holds everything it collected.
   */
  cancel_event: ({organizer, event_id}: {organizer: string, event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a create_event transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer creates a new event with one or more ticket tiers.
   * Returns the new event ID.
   */
  create_event: ({organizer, name, description, venue, date_unix, funding_goal, tiers}: {organizer: string, name: string, description: string, venue: string, date_unix: u64, funding_goal: i128, tiers: Array<TierInput>}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a payout_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the number of payouts released for an event.
   */
  payout_count: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a ticket_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  ticket_count: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a get_organizer transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_organizer: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<string>>>

  /**
   * Construct and simulate a redeem_ticket transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer checks in (redeems) a ticket at the door.
   */
  redeem_ticket: ({organizer, event_id, ticket_id}: {organizer: string, event_id: u32, ticket_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a refund_ticket transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Ticket owner claims back the price paid for a ticket to a cancelled event.
   * 
   * Refunds are pulled by each holder rather than pushed by the organizer,
   * so cancelling never has to loop over every ticket in one transaction.
   */
  refund_ticket: ({owner, event_id, ticket_id}: {owner: string, event_id: u32, ticket_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a release_funds transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer releases escrowed funds to a recipient (a worker, vendor, or
   * venue) after the event has ended. Every release is stored as a public
   * payout record with a memo explaining what it pays for.
   * Returns the payout ID.
   */
  release_funds: ({organizer, event_id, recipient, amount, memo}: {organizer: string, event_id: u32, recipient: string, amount: i128, memo: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a sponsor_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the total number of sponsorship contributions for an event.
   */
  sponsor_count: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a sponsor_event transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Sponsor contributes USDC to an event.
   * Contribution is recorded publicly against the sponsor's address.
   */
  sponsor_event: ({sponsor, event_id, amount}: {sponsor: string, event_id: u32, amount: i128}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a attach_receipt transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Organizer attaches a receipt hash (and optionally where to find the
   * document itself) to a payout as proof of spend. Settable only once
   * per payout — a receipt is evidence, not something to revise later.
   */
  attach_receipt: ({organizer, event_id, payout_id, receipt_hash, receipt_uri}: {organizer: string, event_id: u32, payout_id: u32, receipt_hash: Buffer, receipt_uri: Option<string>}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a total_released transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns the total USDC released from an event's escrow. Together with
   * `get_balance`, this shows how much was raised and how much was spent.
   */
  total_released: ({event_id}: {event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a transfer_ticket transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * The current owner hands a ticket to another address, e.g. when they can
   * no longer attend. Only possible while the event is `Active` and the
   * ticket has not been checked in.
   * 
   * The ticket keeps its `price_paid`, so if the event is later cancelled the
   * refund goes to whoever owns the ticket at that point.
   */
  transfer_ticket: ({from, event_id, ticket_id, to}: {from: string, event_id: u32, ticket_id: u32, to: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_sponsorships transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns up to `limit` sponsorships (capped at `MAX_SPONSORSHIP_PAGE`)
   * starting at `start`, ordered by the time they were made. `start` past
   * the end returns an empty page rather than an error.
   */
  get_sponsorships: ({event_id, start, limit}: {event_id: u32, start: u32, limit: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Array<Sponsorship>>>>

  /**
   * Construct and simulate a get_sponsor_total transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns how much a sponsor currently has contributed (and not yet been
   * refunded) for an event.
   */
  get_sponsor_total: ({event_id, sponsor}: {event_id: u32, sponsor: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a owner_ticket_count transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns how many tickets `owner` has ever bought or received by
   * transfer for an event (see `get_tickets_by_owner` for what that means).
   */
  owner_ticket_count: ({event_id, owner}: {event_id: u32, owner: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a refund_sponsorship transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Sponsor claims back everything they contributed to a cancelled event.
   */
  refund_sponsorship: ({sponsor, event_id}: {sponsor: string, event_id: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<i128>>>

  /**
   * Construct and simulate a get_tickets_by_owner transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   * Returns up to `limit` (capped at `MAX_OWNER_TICKET_PAGE`) ticket IDs
   * `owner` has bought or received by transfer, starting at `start`.
   * 
   * This is a history, not a live "currently owned" list: a ticket appears
   * here under every address that has ever held it, including one it was
   * later transferred away from. Check `get_ticket(event_id, id).owner`
   * against `owner` to find out which of these are still current.
   */
  get_tickets_by_owner: ({event_id, owner, start, limit}: {event_id: u32, owner: string, start: u32, limit: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Array<u32>>>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
        /** Constructor/Initialization Args for the contract's `__constructor` method */
        {token}: {token: string},
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy({token}, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([
        "AAAAAAAAAAAAAAAJZ2V0X2V2ZW50AAAAAAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAfQAAAABUV2ZW50AAAAAAAAAw==",
        "AAAAAAAAAAAAAAAJZ2V0X3RpZXJzAAAAAAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAPqAAAH0AAAAApUaWNrZXRUaWVyAAAAAAAD",
        "AAAAAAAAAAAAAAAKZ2V0X3BheW91dAAAAAAAAgAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAAAAAAlwYXlvdXRfaWQAAAAAAAAEAAAAAQAAA+kAAAfQAAAABlBheW91dAAAAAAAAw==",
        "AAAAAAAAAAAAAAAKZ2V0X3RpY2tldAAAAAAAAgAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAAAAAAl0aWNrZXRfaWQAAAAAAAAEAAAAAQAAA+kAAAfQAAAABlRpY2tldAAAAAAAAw==",
        "AAAAAAAAAAAAAAALZ2V0X2JhbGFuY2UAAAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAALAAAAAw==",
        "AAAAAAAAAAAAAAALZXZlbnRfY291bnQAAAAAAAAAAAEAAAAE",
        "AAAAAAAAAAAAAAAMdGlja2V0X2NvdW50AAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAAEAAAAAw==",
        "AAAAAAAAAAAAAAANZ2V0X29yZ2FuaXplcgAAAAAAAAEAAAAAAAAACGV2ZW50X2lkAAAABAAAAAEAAAPpAAAAEwAAAAM=",
        "AAAAAAAAADBSZXR1cm5zIHRoZSBudW1iZXIgb2YgdGlja2V0IHRpZXJzIGZvciBhbiBldmVudC4AAAAKdGllcl9jb3VudAAAAAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAAEAAAAAw==",
        "AAAAAAAAADNPcmdhbml6ZXIgY2hlY2tzIGluIChyZWRlZW1zKSBhIHRpY2tldCBhdCB0aGUgZG9vci4AAAAADXJlZGVlbV90aWNrZXQAAAAAAAADAAAAAAAAAAlvcmdhbml6ZXIAAAAAAAATAAAAAAAAAAhldmVudF9pZAAAAAQAAAAAAAAACXRpY2tldF9pZAAAAAAAAAQAAAABAAAD6QAAAAIAAAAD",
        "AAAAAAAAADRSZXR1cm5zIHRoZSBudW1iZXIgb2YgcGF5b3V0cyByZWxlYXNlZCBmb3IgYW4gZXZlbnQuAAAADHBheW91dF9jb3VudAAAAAEAAAAAAAAACGV2ZW50X2lkAAAABAAAAAEAAAPpAAAABAAAAAM=",
        "AAAAAAAAADxSZXR1cm5zIHRoZSB0b2tlbiBjb250cmFjdCBhZGRyZXNzIGNvbmZpZ3VyZWQgYXQgZGVwbG95bWVudC4AAAAJZ2V0X3Rva2VuAAAAAAAAAAAAAAEAAAPpAAAAEwAAAAM=",
        "AAAAAAAAAENSZXR1cm5zIHRoZSB0b3RhbCBudW1iZXIgb2Ygc3BvbnNvcnNoaXAgY29udHJpYnV0aW9ucyBmb3IgYW4gZXZlbnQuAAAAAA1zcG9uc29yX2NvdW50AAAAAAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAAEAAAAAw==",
        "AAAAAAAAAEVTcG9uc29yIGNsYWltcyBiYWNrIGV2ZXJ5dGhpbmcgdGhleSBjb250cmlidXRlZCB0byBhIGNhbmNlbGxlZCBldmVudC4AAAAAAAAScmVmdW5kX3Nwb25zb3JzaGlwAAAAAAACAAAAAAAAAAdzcG9uc29yAAAAABMAAAAAAAAACGV2ZW50X2lkAAAABAAAAAEAAAPpAAAACwAAAAM=",
        "AAAAAAAAAF5SZXR1cm5zIGhvdyBtdWNoIGEgc3BvbnNvciBjdXJyZW50bHkgaGFzIGNvbnRyaWJ1dGVkIChhbmQgbm90IHlldCBiZWVuCnJlZnVuZGVkKSBmb3IgYW4gZXZlbnQuAAAAAAARZ2V0X3Nwb25zb3JfdG90YWwAAAAAAAACAAAAAAAAAAhldmVudF9pZAAAAAQAAAAAAAAAB3Nwb25zb3IAAAAAEwAAAAEAAAPpAAAACwAAAAM=",
        "AAAAAAAAAFZPcmdhbml6ZXIgY3JlYXRlcyBhIG5ldyBldmVudCB3aXRoIG9uZSBvciBtb3JlIHRpY2tldCB0aWVycy4KUmV0dXJucyB0aGUgbmV3IGV2ZW50IElELgAAAAAADGNyZWF0ZV9ldmVudAAAAAcAAAAAAAAACW9yZ2FuaXplcgAAAAAAABMAAAAAAAAABG5hbWUAAAAQAAAAAAAAAAtkZXNjcmlwdGlvbgAAAAAQAAAAAAAAAAV2ZW51ZQAAAAAAABAAAAAAAAAACWRhdGVfdW5peAAAAAAAAAYAAAAAAAAADGZ1bmRpbmdfZ29hbAAAAAsAAAAAAAAABXRpZXJzAAAAAAAD6gAAB9AAAAAJVGllcklucHV0AAAAAAAAAQAAA+kAAAAEAAAAAw==",
        "AAAAAAAAAGZTcG9uc29yIGNvbnRyaWJ1dGVzIFVTREMgdG8gYW4gZXZlbnQuCkNvbnRyaWJ1dGlvbiBpcyByZWNvcmRlZCBwdWJsaWNseSBhZ2FpbnN0IHRoZSBzcG9uc29yJ3MgYWRkcmVzcy4AAAAAAA1zcG9uc29yX2V2ZW50AAAAAAAAAwAAAAAAAAAHc3BvbnNvcgAAAAATAAAAAAAAAAhldmVudF9pZAAAAAQAAAAAAAAABmFtb3VudAAAAAAACwAAAAEAAAPpAAAAAgAAAAM=",
        "AAAAAAAAAH1CdXllciBwdXJjaGFzZXMgYSB0aWNrZXQgaW4gYSBnaXZlbiB0aWVyLgpUcmFuc2ZlcnMgYHRpZXIucHJpY2VgIFVTREMgZnJvbSBidXllciB0byB0aGlzIGNvbnRyYWN0LgpSZXR1cm5zIHRoZSBuZXcgdGlja2V0IElELgAAAAAAAApidXlfdGlja2V0AAAAAAADAAAAAAAAAAVidXllcgAAAAAAABMAAAAAAAAACGV2ZW50X2lkAAAABAAAAAAAAAAKdGllcl9pbmRleAAAAAAABAAAAAEAAAPpAAAABAAAAAM=",
        "AAAAAAAAAHZPcmdhbml6ZXIgY2xvc2VzIGFuIGV2ZW50LCBwcmV2ZW50aW5nIGZ1cnRoZXIgdGlja2V0IHNhbGVzIGFuZCBzcG9uc29yc2hpcHMuClN0YXR1cyB0cmFuc2l0aW9ucyBmcm9tIEFjdGl2ZSDihpIgRW5kZWQuAAAAAAAJZW5kX2V2ZW50AAAAAAAAAgAAAAAAAAAJb3JnYW5pemVyAAAAAAAAEwAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAACAAAAAw==",
        "AAAAAAAAAI9SdW5zIG9uY2UsIGF0b21pY2FsbHkgd2l0aCBkZXBsb3ltZW50OiByZWNvcmRzIHRoZSBVU0RDIHRva2VuIGNvbnRyYWN0CmFkZHJlc3MuIERvaW5nIHRoaXMgaW4gdGhlIGNvbnN0cnVjdG9yIG1lYW5zIG5vIG9uZSBjYW4gZnJvbnQtcnVuIHNldHVwLgAAAAANX19jb25zdHJ1Y3RvcgAAAAAAAAEAAAAAAAAABXRva2VuAAAAAAAAEwAAAAA=",
        "AAAAAAAAAIdSZXR1cm5zIGhvdyBtYW55IHRpY2tldHMgYG93bmVyYCBoYXMgZXZlciBib3VnaHQgb3IgcmVjZWl2ZWQgYnkKdHJhbnNmZXIgZm9yIGFuIGV2ZW50IChzZWUgYGdldF90aWNrZXRzX2J5X293bmVyYCBmb3Igd2hhdCB0aGF0IG1lYW5zKS4AAAAAEm93bmVyX3RpY2tldF9jb3VudAAAAAAAAgAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAAAAAAVvd25lcgAAAAAAABMAAAABAAAD6QAAAAQAAAAD",
        "AAAAAAAAAItSZXR1cm5zIHRoZSB0b3RhbCBVU0RDIHJlbGVhc2VkIGZyb20gYW4gZXZlbnQncyBlc2Nyb3cuIFRvZ2V0aGVyIHdpdGgKYGdldF9iYWxhbmNlYCwgdGhpcyBzaG93cyBob3cgbXVjaCB3YXMgcmFpc2VkIGFuZCBob3cgbXVjaCB3YXMgc3BlbnQuAAAAAA50b3RhbF9yZWxlYXNlZAAAAAAAAQAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAALAAAAAw==",
        "AAAAAAAAAL9SZXR1cm5zIHVwIHRvIGBsaW1pdGAgc3BvbnNvcnNoaXBzIChjYXBwZWQgYXQgYE1BWF9TUE9OU09SU0hJUF9QQUdFYCkKc3RhcnRpbmcgYXQgYHN0YXJ0YCwgb3JkZXJlZCBieSB0aGUgdGltZSB0aGV5IHdlcmUgbWFkZS4gYHN0YXJ0YCBwYXN0CnRoZSBlbmQgcmV0dXJucyBhbiBlbXB0eSBwYWdlIHJhdGhlciB0aGFuIGFuIGVycm9yLgAAAAAQZ2V0X3Nwb25zb3JzaGlwcwAAAAMAAAAAAAAACGV2ZW50X2lkAAAABAAAAAAAAAAFc3RhcnQAAAAAAAAEAAAAAAAAAAVsaW1pdAAAAAAAAAQAAAABAAAD6QAAA+oAAAfQAAAAC1Nwb25zb3JzaGlwAAAAAAM=",
        "AAAAAAAAAMtPcmdhbml6ZXIgYXR0YWNoZXMgYSByZWNlaXB0IGhhc2ggKGFuZCBvcHRpb25hbGx5IHdoZXJlIHRvIGZpbmQgdGhlCmRvY3VtZW50IGl0c2VsZikgdG8gYSBwYXlvdXQgYXMgcHJvb2Ygb2Ygc3BlbmQuIFNldHRhYmxlIG9ubHkgb25jZQpwZXIgcGF5b3V0IOKAlCBhIHJlY2VpcHQgaXMgZXZpZGVuY2UsIG5vdCBzb21ldGhpbmcgdG8gcmV2aXNlIGxhdGVyLgAAAAAOYXR0YWNoX3JlY2VpcHQAAAAAAAUAAAAAAAAACW9yZ2FuaXplcgAAAAAAABMAAAAAAAAACGV2ZW50X2lkAAAABAAAAAAAAAAJcGF5b3V0X2lkAAAAAAAABAAAAAAAAAAMcmVjZWlwdF9oYXNoAAAD7gAAACAAAAAAAAAAC3JlY2VpcHRfdXJpAAAAA+gAAAAQAAAAAQAAA+kAAAACAAAAAw==",
        "AAAAAAAAANhUaWNrZXQgb3duZXIgY2xhaW1zIGJhY2sgdGhlIHByaWNlIHBhaWQgZm9yIGEgdGlja2V0IHRvIGEgY2FuY2VsbGVkIGV2ZW50LgoKUmVmdW5kcyBhcmUgcHVsbGVkIGJ5IGVhY2ggaG9sZGVyIHJhdGhlciB0aGFuIHB1c2hlZCBieSB0aGUgb3JnYW5pemVyLApzbyBjYW5jZWxsaW5nIG5ldmVyIGhhcyB0byBsb29wIG92ZXIgZXZlcnkgdGlja2V0IGluIG9uZSB0cmFuc2FjdGlvbi4AAAANcmVmdW5kX3RpY2tldAAAAAAAAAMAAAAAAAAABW93bmVyAAAAAAAAEwAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAAAAAAl0aWNrZXRfaWQAAAAAAAAEAAAAAQAAA+kAAAALAAAAAw==",
        "AAAAAAAAANpPcmdhbml6ZXIgcmVsZWFzZXMgZXNjcm93ZWQgZnVuZHMgdG8gYSByZWNpcGllbnQgKGEgd29ya2VyLCB2ZW5kb3IsIG9yCnZlbnVlKSBhZnRlciB0aGUgZXZlbnQgaGFzIGVuZGVkLiBFdmVyeSByZWxlYXNlIGlzIHN0b3JlZCBhcyBhIHB1YmxpYwpwYXlvdXQgcmVjb3JkIHdpdGggYSBtZW1vIGV4cGxhaW5pbmcgd2hhdCBpdCBwYXlzIGZvci4KUmV0dXJucyB0aGUgcGF5b3V0IElELgAAAAAADXJlbGVhc2VfZnVuZHMAAAAAAAAFAAAAAAAAAAlvcmdhbml6ZXIAAAAAAAATAAAAAAAAAAhldmVudF9pZAAAAAQAAAAAAAAACXJlY2lwaWVudAAAAAAAABMAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAEbWVtbwAAABAAAAABAAAD6QAAAAQAAAAD",
        "AAAAAAAAAQZPcmdhbml6ZXIgY2FuY2VscyBhbiBhY3RpdmUgZXZlbnQgKEFjdGl2ZSDihpIgQ2FuY2VsbGVkKS4gU2FsZXMgYW5kCnNwb25zb3JzaGlwcyBzdG9wLCBhbmQgZXZlcnkgdGlja2V0IGhvbGRlciBhbmQgc3BvbnNvciBjYW4gY2xhaW0gYQpmdWxsIHJlZnVuZC4gRnVuZHMgY2FuIG9ubHkgYmUgcmVsZWFzZWQgYWZ0ZXIgYW4gZXZlbnQgKmVuZHMqLCBzbyBhCmNhbmNlbGxlZCBldmVudCBhbHdheXMgc3RpbGwgaG9sZHMgZXZlcnl0aGluZyBpdCBjb2xsZWN0ZWQuAAAAAAAMY2FuY2VsX2V2ZW50AAAAAgAAAAAAAAAJb3JnYW5pemVyAAAAAAAAEwAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAA+kAAAACAAAAAw==",
        "AAAAAAAAASxUaGUgY3VycmVudCBvd25lciBoYW5kcyBhIHRpY2tldCB0byBhbm90aGVyIGFkZHJlc3MsIGUuZy4gd2hlbiB0aGV5IGNhbgpubyBsb25nZXIgYXR0ZW5kLiBPbmx5IHBvc3NpYmxlIHdoaWxlIHRoZSBldmVudCBpcyBgQWN0aXZlYCBhbmQgdGhlCnRpY2tldCBoYXMgbm90IGJlZW4gY2hlY2tlZCBpbi4KClRoZSB0aWNrZXQga2VlcHMgaXRzIGBwcmljZV9wYWlkYCwgc28gaWYgdGhlIGV2ZW50IGlzIGxhdGVyIGNhbmNlbGxlZCB0aGUKcmVmdW5kIGdvZXMgdG8gd2hvZXZlciBvd25zIHRoZSB0aWNrZXQgYXQgdGhhdCBwb2ludC4AAAAPdHJhbnNmZXJfdGlja2V0AAAAAAQAAAAAAAAABGZyb20AAAATAAAAAAAAAAhldmVudF9pZAAAAAQAAAAAAAAACXRpY2tldF9pZAAAAAAAAAQAAAAAAAAAAnRvAAAAAAATAAAAAQAAA+kAAAACAAAAAw==",
        "AAAAAAAAAZRSZXR1cm5zIHVwIHRvIGBsaW1pdGAgKGNhcHBlZCBhdCBgTUFYX09XTkVSX1RJQ0tFVF9QQUdFYCkgdGlja2V0IElEcwpgb3duZXJgIGhhcyBib3VnaHQgb3IgcmVjZWl2ZWQgYnkgdHJhbnNmZXIsIHN0YXJ0aW5nIGF0IGBzdGFydGAuCgpUaGlzIGlzIGEgaGlzdG9yeSwgbm90IGEgbGl2ZSAiY3VycmVudGx5IG93bmVkIiBsaXN0OiBhIHRpY2tldCBhcHBlYXJzCmhlcmUgdW5kZXIgZXZlcnkgYWRkcmVzcyB0aGF0IGhhcyBldmVyIGhlbGQgaXQsIGluY2x1ZGluZyBvbmUgaXQgd2FzCmxhdGVyIHRyYW5zZmVycmVkIGF3YXkgZnJvbS4gQ2hlY2sgYGdldF90aWNrZXQoZXZlbnRfaWQsIGlkKS5vd25lcmAKYWdhaW5zdCBgb3duZXJgIHRvIGZpbmQgb3V0IHdoaWNoIG9mIHRoZXNlIGFyZSBzdGlsbCBjdXJyZW50LgAAABRnZXRfdGlja2V0c19ieV9vd25lcgAAAAQAAAAAAAAACGV2ZW50X2lkAAAABAAAAAAAAAAFb3duZXIAAAAAAAATAAAAAAAAAAVzdGFydAAAAAAAAAQAAAAAAAAABWxpbWl0AAAAAAAABAAAAAEAAAPpAAAD6gAAAAQAAAAD",
        "AAAAAQAAAAAAAAAAAAAABUV2ZW50AAAAAAAACAAAADlDdXJyZW50IFVTREMgYmFsYW5jZSBoZWxkIGJ5IHRoZSBjb250cmFjdCBmb3IgdGhpcyBldmVudC4AAAAAAAAHYmFsYW5jZQAAAAALAAAAIVVuaXggdGltZXN0YW1wIG9mIHRoZSBldmVudCBkYXRlLgAAAAAAAAlkYXRlX3VuaXgAAAAAAAAGAAAAAAAAAAtkZXNjcmlwdGlvbgAAAAAQAAAAHUZ1bmRpbmcgZ29hbCBpbiBVU0RDIHN0cm9vcHMuAAAAAAAADGZ1bmRpbmdfZ29hbAAAAAsAAAAAAAAABG5hbWUAAAAQAAAAAAAAAAlvcmdhbml6ZXIAAAAAAAATAAAAAAAAAAZzdGF0dXMAAAAAB9AAAAALRXZlbnRTdGF0dXMAAAAAAAAAAAV2ZW51ZQAAAAAAABA=",
        "AAAAAQAAACNPbi1jaGFpbiBwcm9vZiBvZiB0aWNrZXQgb3duZXJzaGlwLgAAAAAAAAAABlRpY2tldAAAAAAABgAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAAAAAAVvd25lcgAAAAAAABMAAAA+VVNEQyBzdHJvb3BzIHBhaWQsIHJldHVybmVkIGluIGZ1bGwgaWYgdGhlIGV2ZW50IGlzIGNhbmNlbGxlZC4AAAAAAApwcmljZV9wYWlkAAAAAAALAAAAAAAAAAhyZWRlZW1lZAAAAAEAAAAAAAAACHJlZnVuZGVkAAAAAQAAAAAAAAAKdGllcl9pbmRleAAAAAAABA==",
        "AAAAAQAAAClBIHB1YmxpYyBzcG9uc29yc2hpcCBjb250cmlidXRpb24gcmVjb3JkLgAAAAAAAAAAAAALU3BvbnNvcnNoaXAAAAAAAgAAAAAAAAAGYW1vdW50AAAAAAALAAAAAAAAAAdzcG9uc29yAAAAABM=",
        "AAAAAQAAAD9TdG9yZWQgc3RhdGUgZm9yIGEgdGlja2V0IHRpZXIsIGV4dGVuZGVkIHdpdGggbGl2ZSBzYWxlcyBjb3VudC4AAAAAAAAAAApUaWNrZXRUaWVyAAAAAAAEAAAAAAAAAARuYW1lAAAAEAAAAAAAAAAFcHJpY2UAAAAAAAALAAAAAAAAAApzdXBwbHlfY2FwAAAAAAAEAAAAAAAAAAx0aWNrZXRzX3NvbGQAAAAE",
        "AAAAAQAAADVJbnB1dCBzaGFwZSBmb3IgYSB0aWNrZXQgdGllciB3aGVuIGNyZWF0aW5nIGFuIGV2ZW50LgAAAAAAAAAAAAAJVGllcklucHV0AAAAAAAAAwAAAAAAAAAEbmFtZQAAABAAAAAsUHJpY2UgaW4gVVNEQyBzdHJvb3BzICgxIFVTREMgPSAxMF8wMDBfMDAwKS4AAAAFcHJpY2UAAAAAAAALAAAAAAAAAApzdXBwbHlfY2FwAAAAAAAE",
        "AAAAAQAAADlBIHB1YmxpYyByZWNvcmQgb2YgZnVuZHMgcmVsZWFzZWQgZnJvbSBhbiBldmVudCdzIGVzY3Jvdy4AAAAAAAAAAAAABlBheW91dAAAAAAABgAAAAAAAAAGYW1vdW50AAAAAAALAAAAgVdoYXQgdGhlIHBheW91dCBpcyBmb3IsIGUuZy4gIlNvdW5kIGNyZXciIOKAlCByZXF1aXJlZCBzbyBldmVyeQpkaXNidXJzZW1lbnQgZXhwbGFpbnMgaXRzZWxmIG5leHQgdG8gdGhlIGNvbnRyaWJ1dGlvbnMgaXQgc3BlbmRzLgAAAAAAAARtZW1vAAAAEAAAAIFTSEEtMjU2IG9mIGEgcmVjZWlwdCBkb2N1bWVudCBwcm92aW5nIHdoYXQgdGhlIHBheW91dCB3YXMgc3BlbnQgb24uClNldHRhYmxlIG9uY2UgdmlhIGBhdHRhY2hfcmVjZWlwdGAsIGFmdGVyIHRoZSBwYXlvdXQgaXMgbWFkZS4AAAAAAAAMcmVjZWlwdF9oYXNoAAAD6AAAA+4AAAAgAAAAg1doZXJlIHRvIGZpbmQgdGhlIHJlY2VpcHQgZG9jdW1lbnQgaXRzZWxmLCBlLmcuIGFuIElQRlMgQ0lELiBUaGUgaGFzaAphYm92ZSBpcyB3aGF0IHByb3ZlcyBpdCBtYXRjaGVzOyB0aGlzIGlzIGp1c3QgaG93IHRvIGZpbmQgaXQuAAAAAAtyZWNlaXB0X3VyaQAAAAPoAAAAEAAAAAAAAAAJcmVjaXBpZW50AAAAAAAAEwAAACBMZWRnZXIgdGltZXN0YW1wIG9mIHRoZSByZWxlYXNlLgAAAAl0aW1lc3RhbXAAAAAAAAAG",
        "AAAAAgAAAAAAAAAAAAAAB0RhdGFLZXkAAAAADgAAAAAAAAAAAAAABVRva2VuAAAAAAAAAAAAAAAAAAAMRXZlbnRDb3VudGVyAAAAAQAAAAAAAAAFRXZlbnQAAAAAAAABAAAABAAAAAEAAAAAAAAABVRpZXJzAAAAAAAAAQAAAAQAAAABAAAAAAAAAA1UaWNrZXRDb3VudGVyAAAAAAAAAQAAAAQAAAABAAAAAAAAAAZUaWNrZXQAAAAAAAIAAAAEAAAABAAAAAEAAAAAAAAAEFNwb25zb3JzaGlwQ291bnQAAAABAAAABAAAAAEAAACRT25lIGVudHJ5IHBlciBjb250cmlidXRpb24gc28gdGhlIHB1YmxpYyBoaXN0b3J5IGNhbiBncm93IHdpdGhvdXQgYQpzaW5nbGUgc3RvcmFnZSBlbnRyeSBoaXR0aW5nIFNvcm9iYW4ncyBzaXplIGxpbWl0IChzZWUgTUFYX1NQT05TT1JTSElQX1BBR0UpLgAAAAAAAAtTcG9uc29yc2hpcAAAAAACAAAABAAAAAQAAAABAAAAT1J1bm5pbmcgdG90YWwgYSBzcG9uc29yIGhhcyBjb250cmlidXRlZCB0byBhbiBldmVudCwgb3dlZCBiYWNrIG9uIGNhbmNlbGxhdGlvbi4AAAAADFNwb25zb3JUb3RhbAAAAAIAAAAEAAAAEwAAAAEAAAAAAAAADVBheW91dENvdW50ZXIAAAAAAAABAAAABAAAAAEAAABPT25lIGVudHJ5IHBlciBwYXlvdXQgc28gdGhlIGhpc3RvcnkgY2FuIGdyb3cgd2l0aG91dCBoaXR0aW5nIGVudHJ5IHNpemUgbGltaXRzLgAAAAAGUGF5b3V0AAAAAAACAAAABAAAAAQAAAABAAAAAAAAAA1Ub3RhbFJlbGVhc2VkAAAAAAAAAQAAAAQAAAABAAAAAAAAABBPd25lclRpY2tldENvdW50AAAAAgAAAAQAAAATAAAAAQAAAKJPbmUgZW50cnkgcGVyIHRpY2tldCBhbiBhZGRyZXNzIGhhcyBldmVyIGJvdWdodCBvciByZWNlaXZlZCBieQp0cmFuc2ZlciBmb3IgYW4gZXZlbnQg4oCUIGEgaGlzdG9yeSwgbm90IGEgbGl2ZSAiY3VycmVudGx5IG93bmVkIiBsaXN0LgpTZWUgYGdldF90aWNrZXRzX2J5X293bmVyYC4AAAAAAAtPd25lclRpY2tldAAAAAADAAAABAAAABMAAAAE",
        "AAAAAgAAAAAAAAAAAAAAC0V2ZW50U3RhdHVzAAAAAAMAAAAAAAAAAAAAAAZBY3RpdmUAAAAAAAAAAAAAAAAABUVuZGVkAAAAAAAAAAAAAAAAAAAJQ2FuY2VsbGVkAAAA",
        "AAAABAAAAIxFcnJvciBjb2RlcyBhcmUgcGFydCBvZiB0aGUgY29udHJhY3QncyBwdWJsaWMgaW50ZXJmYWNlIOKAlCBjbGllbnRzIG1hdGNoIG9uCnRoZSBudW1lcmljIHZhbHVlLCBzbyBuZXZlciByZW51bWJlciBvciByZXVzZSBhbiBleGlzdGluZyBjb2RlLgAAAAAAAAAFRXJyb3IAAAAAAAAZAAAAR1Jlc2VydmVkOiBzZXR1cCBub3cgaGFwcGVucyBpbiB0aGUgY29uc3RydWN0b3IsIHdoaWNoIGNhbm5vdCBydW4gdHdpY2UuAAAAABJBbHJlYWR5SW5pdGlhbGl6ZWQAAAAAAAEAAAAAAAAADk5vdEluaXRpYWxpemVkAAAAAAACAAAAAAAAAA1FdmVudE5vdEZvdW5kAAAAAAAAAwAAAAAAAAAOVGlja2V0Tm90Rm91bmQAAAAAAAQAAAAAAAAADE5vdE9yZ2FuaXplcgAAAAUAAAAAAAAADkV2ZW50Tm90QWN0aXZlAAAAAAAGAAAAAAAAAAdOb1RpZXJzAAAAAAcAAAAAAAAAEkludmFsaWRGdW5kaW5nR29hbAAAAAAACAAAAAAAAAAQSW52YWxpZFRpZXJQcmljZQAAAAkAAAAAAAAAEEludmFsaWRTdXBwbHlDYXAAAAAKAAAAAAAAAAtJbnZhbGlkVGllcgAAAAALAAAAAAAAAAtUaWVyU29sZE91dAAAAAAMAAAAAAAAAA9BbHJlYWR5UmVkZWVtZWQAAAAADQAAAAAAAAANSW52YWxpZEFtb3VudAAAAAAAAA4AAAAAAAAAEUV2ZW50Tm90Q2FuY2VsbGVkAAAAAAAADwAAAAAAAAAOTm90VGlja2V0T3duZXIAAAAAABAAAAAAAAAAD0FscmVhZHlSZWZ1bmRlZAAAAAARAAAAAAAAAA9Ob3RoaW5nVG9SZWZ1bmQAAAAAEgAAAAAAAAANRXZlbnROb3RFbmRlZAAAAAAAABMAAAAAAAAAEUluc3VmZmljaWVudEZ1bmRzAAAAAAAAFAAAAAAAAAALSW52YWxpZE1lbW8AAAAAFQAAAAAAAAAOUGF5b3V0Tm90Rm91bmQAAAAAABYAAAAAAAAADlRyYW5zZmVyVG9TZWxmAAAAAAAXAAAAAAAAABZSZWNlaXB0QWxyZWFkeUF0dGFjaGVkAAAAAAAYAAAAAAAAABFJbnZhbGlkUmVjZWlwdFVyaQAAAAAAABk=",
        "AAAABQAAAAAAAAAAAAAACVNwb25zb3JlZAAAAAAAAAEAAAAJc3BvbnNvcmVkAAAAAAAAAwAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAAAAAAAAHc3BvbnNvcgAAAAATAAAAAQAAAAAAAAAGYW1vdW50AAAAAAALAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAACkV2ZW50RW5kZWQAAAAAAAEAAAALZXZlbnRfZW5kZWQAAAAAAgAAAAAAAAAIZXZlbnRfaWQAAAAEAAAAAQAAAAAAAAAHYmFsYW5jZQAAAAALAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAAD1JlY2VpcHRBdHRhY2hlZAAAAAABAAAAEHJlY2VpcHRfYXR0YWNoZWQAAAADAAAAAAAAAAhldmVudF9pZAAAAAQAAAABAAAAAAAAAAlwYXlvdXRfaWQAAAAAAAAEAAAAAAAAAAAAAAAMcmVjZWlwdF9oYXNoAAAD7gAAACAAAAAAAAAAAg==",
        "AAAABQAAAAAAAAAAAAAAD1RpY2tldFB1cmNoYXNlZAAAAAABAAAAEHRpY2tldF9wdXJjaGFzZWQAAAAFAAAAAAAAAAhldmVudF9pZAAAAAQAAAABAAAAAAAAAAVidXllcgAAAAAAABMAAAABAAAAAAAAAAl0aWNrZXRfaWQAAAAAAAAEAAAAAAAAAAAAAAAKdGllcl9pbmRleAAAAAAABAAAAAAAAAAAAAAABXByaWNlAAAAAAAACwAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAADEV2ZW50Q3JlYXRlZAAAAAEAAAANZXZlbnRfY3JlYXRlZAAAAAAAAAQAAAAAAAAACGV2ZW50X2lkAAAABAAAAAEAAAAAAAAACW9yZ2FuaXplcgAAAAAAABMAAAABAAAAAAAAAAxmdW5kaW5nX2dvYWwAAAALAAAAAAAAAAAAAAAJZGF0ZV91bml4AAAAAAAABgAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAADUZ1bmRzUmVsZWFzZWQAAAAAAAABAAAADmZ1bmRzX3JlbGVhc2VkAAAAAAAFAAAAAAAAAAhldmVudF9pZAAAAAQAAAABAAAAAAAAAAlyZWNpcGllbnQAAAAAAAATAAAAAQAAAAAAAAAJcGF5b3V0X2lkAAAAAAAABAAAAAAAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAAAAAABG1lbW8AAAAQAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAADkV2ZW50Q2FuY2VsbGVkAAAAAAABAAAAD2V2ZW50X2NhbmNlbGxlZAAAAAACAAAAAAAAAAhldmVudF9pZAAAAAQAAAABAAAAAAAAAAdiYWxhbmNlAAAAAAsAAAAAAAAAAg==",
        "AAAABQAAAAAAAAAAAAAADlRpY2tldFJlZGVlbWVkAAAAAAABAAAAD3RpY2tldF9yZWRlZW1lZAAAAAACAAAAAAAAAAhldmVudF9pZAAAAAQAAAABAAAAAAAAAAl0aWNrZXRfaWQAAAAAAAAEAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAADlRpY2tldFJlZnVuZGVkAAAAAAABAAAAD3RpY2tldF9yZWZ1bmRlZAAAAAAEAAAAAAAAAAhldmVudF9pZAAAAAQAAAABAAAAAAAAAAVvd25lcgAAAAAAABMAAAABAAAAAAAAAAl0aWNrZXRfaWQAAAAAAAAEAAAAAAAAAAAAAAAGYW1vdW50AAAAAAALAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAAE1Nwb25zb3JzaGlwUmVmdW5kZWQAAAAAAQAAABRzcG9uc29yc2hpcF9yZWZ1bmRlZAAAAAMAAAAAAAAACGV2ZW50X2lkAAAABAAAAAEAAAAAAAAAB3Nwb25zb3IAAAAAEwAAAAEAAAAAAAAABmFtb3VudAAAAAAACwAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAAEVRpY2tldFRyYW5zZmVycmVkAAAAAAAAAQAAABJ0aWNrZXRfdHJhbnNmZXJyZWQAAAAAAAQAAAAAAAAACGV2ZW50X2lkAAAABAAAAAEAAAAAAAAABGZyb20AAAATAAAAAQAAAAAAAAACdG8AAAAAABMAAAAAAAAAAAAAAAl0aWNrZXRfaWQAAAAAAAAEAAAAAAAAAAI=" ]),
      options
    )
  }
  public readonly fromJSON = {
    end_event: this.txFromJSON<Result<void>>,
        get_event: this.txFromJSON<Result<Event>>,
        get_tiers: this.txFromJSON<Result<Array<TicketTier>>>,
        get_token: this.txFromJSON<Result<string>>,
        buy_ticket: this.txFromJSON<Result<u32>>,
        get_payout: this.txFromJSON<Result<Payout>>,
        get_ticket: this.txFromJSON<Result<Ticket>>,
        tier_count: this.txFromJSON<Result<u32>>,
        event_count: this.txFromJSON<u32>,
        get_balance: this.txFromJSON<Result<i128>>,
        cancel_event: this.txFromJSON<Result<void>>,
        create_event: this.txFromJSON<Result<u32>>,
        payout_count: this.txFromJSON<Result<u32>>,
        ticket_count: this.txFromJSON<Result<u32>>,
        get_organizer: this.txFromJSON<Result<string>>,
        redeem_ticket: this.txFromJSON<Result<void>>,
        refund_ticket: this.txFromJSON<Result<i128>>,
        release_funds: this.txFromJSON<Result<u32>>,
        sponsor_count: this.txFromJSON<Result<u32>>,
        sponsor_event: this.txFromJSON<Result<void>>,
        attach_receipt: this.txFromJSON<Result<void>>,
        total_released: this.txFromJSON<Result<i128>>,
        transfer_ticket: this.txFromJSON<Result<void>>,
        get_sponsorships: this.txFromJSON<Result<Array<Sponsorship>>>,
        get_sponsor_total: this.txFromJSON<Result<i128>>,
        owner_ticket_count: this.txFromJSON<Result<u32>>,
        refund_sponsorship: this.txFromJSON<Result<i128>>,
        get_tickets_by_owner: this.txFromJSON<Result<Array<u32>>>
  }
}