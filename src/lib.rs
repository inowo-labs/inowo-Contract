#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype,
    token::Client as TokenClient, Address, BytesN, Env, IntoVal, String, Val, Vec,
};

// ─── Types ────────────────────────────────────────────────────────────────────

/// Input shape for a ticket tier when creating an event.
#[contracttype]
#[derive(Clone)]
pub struct TierInput {
    pub name: String,
    /// Price in USDC stroops (1 USDC = 10_000_000).
    pub price: i128,
    pub supply_cap: u32,
}

/// Stored state for a ticket tier, extended with live sales count.
#[contracttype]
#[derive(Clone)]
pub struct TicketTier {
    pub name: String,
    pub price: i128,
    pub supply_cap: u32,
    pub tickets_sold: u32,
}

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum EventStatus {
    Active,
    Ended,
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct Event {
    pub organizer: Address,
    pub name: String,
    pub description: String,
    pub venue: String,
    /// Unix timestamp of the event date.
    pub date_unix: u64,
    /// Funding goal in USDC stroops.
    pub funding_goal: i128,
    /// Current USDC balance held by the contract for this event.
    pub balance: i128,
    pub status: EventStatus,
}

/// On-chain proof of ticket ownership.
#[contracttype]
#[derive(Clone)]
pub struct Ticket {
    pub event_id: u32,
    pub tier_index: u32,
    pub owner: Address,
    pub redeemed: bool,
    /// USDC stroops paid, returned in full if the event is cancelled.
    pub price_paid: i128,
    pub refunded: bool,
}

/// A public sponsorship contribution record.
#[contracttype]
#[derive(Clone)]
pub struct Sponsorship {
    pub sponsor: Address,
    pub amount: i128,
}

/// A public record of funds released from an event's escrow.
#[contracttype]
#[derive(Clone)]
pub struct Payout {
    pub recipient: Address,
    pub amount: i128,
    /// What the payout is for, e.g. "Sound crew" — required so every
    /// disbursement explains itself next to the contributions it spends.
    pub memo: String,
    /// Ledger timestamp of the release.
    pub timestamp: u64,
    /// SHA-256 of a receipt document proving what the payout was spent on.
    /// Settable once via `attach_receipt`, after the payout is made.
    pub receipt_hash: Option<BytesN<32>>,
    /// Where to find the receipt document itself, e.g. an IPFS CID. The hash
    /// above is what proves it matches; this is just how to find it.
    pub receipt_uri: Option<String>,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Error codes are part of the contract's public interface — clients match on
/// the numeric value, so never renumber or reuse an existing code.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Reserved: setup now happens in the constructor, which cannot run twice.
    AlreadyInitialized = 1,
    NotInitialized = 2,
    EventNotFound = 3,
    TicketNotFound = 4,
    NotOrganizer = 5,
    EventNotActive = 6,
    NoTiers = 7,
    InvalidFundingGoal = 8,
    InvalidTierPrice = 9,
    InvalidSupplyCap = 10,
    InvalidTier = 11,
    TierSoldOut = 12,
    AlreadyRedeemed = 13,
    InvalidAmount = 14,
    EventNotCancelled = 15,
    NotTicketOwner = 16,
    AlreadyRefunded = 17,
    NothingToRefund = 18,
    EventNotEnded = 19,
    InsufficientFunds = 20,
    InvalidMemo = 21,
    PayoutNotFound = 22,
    TransferToSelf = 23,
    ReceiptAlreadyAttached = 24,
    InvalidReceiptUri = 25,
}

// ─── Contract events ──────────────────────────────────────────────────────────
//
// Every state change emits an event so indexers can follow an event's full
// money flow without polling contract storage.

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCreated {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub organizer: Address,
    pub funding_goal: i128,
    pub date_unix: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sponsored {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub sponsor: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketPurchased {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub buyer: Address,
    pub ticket_id: u32,
    pub tier_index: u32,
    pub price: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketTransferred {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub from: Address,
    pub to: Address,
    pub ticket_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketRedeemed {
    #[topic]
    pub event_id: u32,
    pub ticket_id: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnded {
    #[topic]
    pub event_id: u32,
    pub balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCancelled {
    #[topic]
    pub event_id: u32,
    pub balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TicketRefunded {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub owner: Address,
    pub ticket_id: u32,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SponsorshipRefunded {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub sponsor: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsReleased {
    #[topic]
    pub event_id: u32,
    #[topic]
    pub recipient: Address,
    pub payout_id: u32,
    pub amount: i128,
    pub memo: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptAttached {
    #[topic]
    pub event_id: u32,
    pub payout_id: u32,
    pub receipt_hash: BytesN<32>,
}

// ─── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Token,
    EventCounter,
    Event(u32),
    Tiers(u32),
    TicketCounter(u32),
    Ticket(u32, u32),
    SponsorshipCount(u32),
    /// One entry per contribution so the public history can grow without a
    /// single storage entry hitting Soroban's size limit (see MAX_SPONSORSHIP_PAGE).
    Sponsorship(u32, u32),
    /// Running total a sponsor has contributed to an event, owed back on cancellation.
    SponsorTotal(u32, Address),
    PayoutCounter(u32),
    /// One entry per payout so the history can grow without hitting entry size limits.
    Payout(u32, u32),
    TotalReleased(u32),
    OwnerTicketCount(u32, Address),
    /// One entry per ticket an address has ever bought or received by
    /// transfer for an event — a history, not a live "currently owned" list.
    /// See `get_tickets_by_owner`.
    OwnerTicket(u32, Address, u32),
}

/// Upper bound on a payout memo, in bytes, to keep payout entries small.
pub const MAX_MEMO_LEN: u32 = 200;

/// Upper bound on a receipt URI, in bytes — long enough for an IPFS CID or a
/// short URL, not a full document.
pub const MAX_RECEIPT_URI_LEN: u32 = 200;

/// Most ticket IDs `get_tickets_by_owner` returns in one call, regardless of
/// the requested `limit`, so a page always has a bounded, predictable cost.
pub const MAX_OWNER_TICKET_PAGE: u32 = 50;

/// Most sponsorships `get_sponsorships` returns in one call, regardless of
/// the requested `limit`, so a page always has a bounded, predictable cost.
pub const MAX_SPONSORSHIP_PAGE: u32 = 50;

// ─── Storage helpers ──────────────────────────────────────────────────────────

const DAY_IN_LEDGERS: u32 = 17_280;
// Entries are extended back to TTL_EXTEND_TO whenever they are written and
// have fewer than TTL_THRESHOLD ledgers left, so active events never archive.
const TTL_THRESHOLD: u32 = 30 * DAY_IN_LEDGERS;
const TTL_EXTEND_TO: u32 = 120 * DAY_IN_LEDGERS;

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
}

fn save<V: IntoVal<Env, Val>>(env: &Env, key: &DataKey, value: &V) {
    env.storage().persistent().set(key, value);
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

fn load_token(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Token)
        .ok_or(Error::NotInitialized)
}

fn load_event(env: &Env, event_id: u32) -> Result<Event, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Event(event_id))
        .ok_or(Error::EventNotFound)
}

fn require_event_exists(env: &Env, event_id: u32) -> Result<(), Error> {
    if env.storage().persistent().has(&DataKey::Event(event_id)) {
        Ok(())
    } else {
        Err(Error::EventNotFound)
    }
}

fn load_tiers(env: &Env, event_id: u32) -> Result<Vec<TicketTier>, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Tiers(event_id))
        .ok_or(Error::EventNotFound)
}

fn load_sponsor_total(env: &Env, event_id: u32, sponsor: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::SponsorTotal(event_id, sponsor.clone()))
        .unwrap_or(0)
}

fn load_ticket(env: &Env, event_id: u32, ticket_id: u32) -> Result<Ticket, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Ticket(event_id, ticket_id))
        .ok_or(Error::TicketNotFound)
}

fn load_u32(env: &Env, key: &DataKey) -> u32 {
    env.storage().persistent().get(key).unwrap_or(0)
}

/// Records that `owner` has held `ticket_id` for `event_id`, called on both
/// `buy_ticket` and `transfer_ticket` so the history follows the ticket.
fn append_owner_ticket(env: &Env, event_id: u32, owner: &Address, ticket_id: u32) {
    let index = load_u32(env, &DataKey::OwnerTicketCount(event_id, owner.clone()));
    save(
        env,
        &DataKey::OwnerTicketCount(event_id, owner.clone()),
        &(index + 1),
    );
    save(
        env,
        &DataKey::OwnerTicket(event_id, owner.clone(), index),
        &ticket_id,
    );
}

fn load_total_released(env: &Env, event_id: u32) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TotalReleased(event_id))
        .unwrap_or(0)
}

fn pay_out(env: &Env, to: &Address, amount: i128) -> Result<(), Error> {
    TokenClient::new(env, &load_token(env)?).transfer(&env.current_contract_address(), to, &amount);
    Ok(())
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct InowoContract;

#[contractimpl]
impl InowoContract {
    /// Runs once, atomically with deployment: records the USDC token contract
    /// address. Doing this in the constructor means no one can front-run setup.
    pub fn __constructor(env: Env, token: Address) {
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::EventCounter, &0u32);
        bump_instance(&env);
    }

    /// Organizer creates a new event with one or more ticket tiers.
    /// Returns the new event ID.
    #[allow(clippy::too_many_arguments)]
    pub fn create_event(
        env: Env,
        organizer: Address,
        name: String,
        description: String,
        venue: String,
        date_unix: u64,
        funding_goal: i128,
        tiers: Vec<TierInput>,
    ) -> Result<u32, Error> {
        organizer.require_auth();

        if tiers.is_empty() {
            return Err(Error::NoTiers);
        }
        if funding_goal <= 0 {
            return Err(Error::InvalidFundingGoal);
        }
        for t in tiers.iter() {
            if t.price <= 0 {
                return Err(Error::InvalidTierPrice);
            }
            if t.supply_cap == 0 {
                return Err(Error::InvalidSupplyCap);
            }
        }

        bump_instance(&env);
        let event_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::EventCounter)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::EventCounter, &(event_id + 1));

        save(
            &env,
            &DataKey::Event(event_id),
            &Event {
                organizer: organizer.clone(),
                name,
                description,
                venue,
                date_unix,
                funding_goal,
                balance: 0,
                status: EventStatus::Active,
            },
        );

        let mut tier_list: Vec<TicketTier> = Vec::new(&env);
        for t in tiers.iter() {
            tier_list.push_back(TicketTier {
                name: t.name,
                price: t.price,
                supply_cap: t.supply_cap,
                tickets_sold: 0,
            });
        }
        save(&env, &DataKey::Tiers(event_id), &tier_list);
        save(&env, &DataKey::TicketCounter(event_id), &0u32);

        EventCreated {
            event_id,
            organizer,
            funding_goal,
            date_unix,
        }
        .publish(&env);

        Ok(event_id)
    }

    /// Buyer purchases a ticket in a given tier.
    /// Transfers `tier.price` USDC from buyer to this contract.
    /// Returns the new ticket ID.
    pub fn buy_ticket(
        env: Env,
        buyer: Address,
        event_id: u32,
        tier_index: u32,
    ) -> Result<u32, Error> {
        buyer.require_auth();

        let mut event = load_event(&env, event_id)?;
        if event.status != EventStatus::Active {
            return Err(Error::EventNotActive);
        }

        let mut tiers = load_tiers(&env, event_id)?;
        let mut tier = tiers.get(tier_index).ok_or(Error::InvalidTier)?;
        if tier.tickets_sold >= tier.supply_cap {
            return Err(Error::TierSoldOut);
        }

        let price = tier.price;
        TokenClient::new(&env, &load_token(&env)?).transfer(
            &buyer,
            env.current_contract_address(),
            &price,
        );

        tier.tickets_sold += 1;
        tiers.set(tier_index, tier);
        save(&env, &DataKey::Tiers(event_id), &tiers);

        event.balance += price;
        save(&env, &DataKey::Event(event_id), &event);

        let ticket_id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::TicketCounter(event_id))
            .unwrap_or(0);
        save(&env, &DataKey::TicketCounter(event_id), &(ticket_id + 1));

        save(
            &env,
            &DataKey::Ticket(event_id, ticket_id),
            &Ticket {
                event_id,
                tier_index,
                owner: buyer.clone(),
                redeemed: false,
                price_paid: price,
                refunded: false,
            },
        );
        append_owner_ticket(&env, event_id, &buyer, ticket_id);

        TicketPurchased {
            event_id,
            buyer,
            ticket_id,
            tier_index,
            price,
        }
        .publish(&env);

        Ok(ticket_id)
    }

    /// The current owner hands a ticket to another address, e.g. when they can
    /// no longer attend. Only possible while the event is `Active` and the
    /// ticket has not been checked in.
    ///
    /// The ticket keeps its `price_paid`, so if the event is later cancelled the
    /// refund goes to whoever owns the ticket at that point.
    pub fn transfer_ticket(
        env: Env,
        from: Address,
        event_id: u32,
        ticket_id: u32,
        to: Address,
    ) -> Result<(), Error> {
        from.require_auth();

        let event = load_event(&env, event_id)?;
        if event.status != EventStatus::Active {
            return Err(Error::EventNotActive);
        }

        let mut ticket = load_ticket(&env, event_id, ticket_id)?;
        if ticket.owner != from {
            return Err(Error::NotTicketOwner);
        }
        if ticket.redeemed {
            return Err(Error::AlreadyRedeemed);
        }
        if to == from {
            return Err(Error::TransferToSelf);
        }

        ticket.owner = to.clone();
        save(&env, &DataKey::Ticket(event_id, ticket_id), &ticket);
        append_owner_ticket(&env, event_id, &to, ticket_id);

        TicketTransferred {
            event_id,
            from,
            to,
            ticket_id,
        }
        .publish(&env);
        Ok(())
    }

    /// Organizer checks in (redeems) a ticket at the door.
    pub fn redeem_ticket(
        env: Env,
        organizer: Address,
        event_id: u32,
        ticket_id: u32,
    ) -> Result<(), Error> {
        organizer.require_auth();

        let event = load_event(&env, event_id)?;
        if event.organizer != organizer {
            return Err(Error::NotOrganizer);
        }
        if event.status == EventStatus::Cancelled {
            return Err(Error::EventNotActive);
        }

        let mut ticket = load_ticket(&env, event_id, ticket_id)?;
        if ticket.redeemed {
            return Err(Error::AlreadyRedeemed);
        }

        ticket.redeemed = true;
        save(&env, &DataKey::Ticket(event_id, ticket_id), &ticket);

        TicketRedeemed {
            event_id,
            ticket_id,
        }
        .publish(&env);
        Ok(())
    }

    /// Sponsor contributes USDC to an event.
    /// Contribution is recorded publicly against the sponsor's address.
    pub fn sponsor_event(
        env: Env,
        sponsor: Address,
        event_id: u32,
        amount: i128,
    ) -> Result<(), Error> {
        sponsor.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let mut event = load_event(&env, event_id)?;
        if event.status != EventStatus::Active {
            return Err(Error::EventNotActive);
        }

        TokenClient::new(&env, &load_token(&env)?).transfer(
            &sponsor,
            env.current_contract_address(),
            &amount,
        );

        let index = load_u32(&env, &DataKey::SponsorshipCount(event_id));
        save(&env, &DataKey::SponsorshipCount(event_id), &(index + 1));
        save(
            &env,
            &DataKey::Sponsorship(event_id, index),
            &Sponsorship {
                sponsor: sponsor.clone(),
                amount,
            },
        );

        let total = load_sponsor_total(&env, event_id, &sponsor) + amount;
        save(
            &env,
            &DataKey::SponsorTotal(event_id, sponsor.clone()),
            &total,
        );

        event.balance += amount;
        save(&env, &DataKey::Event(event_id), &event);

        Sponsored {
            event_id,
            sponsor,
            amount,
        }
        .publish(&env);
        Ok(())
    }

    /// Organizer closes an event, preventing further ticket sales and sponsorships.
    /// Status transitions from Active → Ended.
    pub fn end_event(env: Env, organizer: Address, event_id: u32) -> Result<(), Error> {
        organizer.require_auth();

        let mut event = load_event(&env, event_id)?;
        if event.organizer != organizer {
            return Err(Error::NotOrganizer);
        }
        if event.status != EventStatus::Active {
            return Err(Error::EventNotActive);
        }

        event.status = EventStatus::Ended;
        save(&env, &DataKey::Event(event_id), &event);

        EventEnded {
            event_id,
            balance: event.balance,
        }
        .publish(&env);
        Ok(())
    }

    /// Organizer cancels an active event (Active → Cancelled). Sales and
    /// sponsorships stop, and every ticket holder and sponsor can claim a
    /// full refund. Funds can only be released after an event *ends*, so a
    /// cancelled event always still holds everything it collected.
    pub fn cancel_event(env: Env, organizer: Address, event_id: u32) -> Result<(), Error> {
        organizer.require_auth();

        let mut event = load_event(&env, event_id)?;
        if event.organizer != organizer {
            return Err(Error::NotOrganizer);
        }
        if event.status != EventStatus::Active {
            return Err(Error::EventNotActive);
        }

        event.status = EventStatus::Cancelled;
        save(&env, &DataKey::Event(event_id), &event);

        EventCancelled {
            event_id,
            balance: event.balance,
        }
        .publish(&env);
        Ok(())
    }

    /// Ticket owner claims back the price paid for a ticket to a cancelled event.
    ///
    /// Refunds are pulled by each holder rather than pushed by the organizer,
    /// so cancelling never has to loop over every ticket in one transaction.
    pub fn refund_ticket(
        env: Env,
        owner: Address,
        event_id: u32,
        ticket_id: u32,
    ) -> Result<i128, Error> {
        owner.require_auth();

        let mut event = load_event(&env, event_id)?;
        if event.status != EventStatus::Cancelled {
            return Err(Error::EventNotCancelled);
        }

        let mut ticket = load_ticket(&env, event_id, ticket_id)?;
        if ticket.owner != owner {
            return Err(Error::NotTicketOwner);
        }
        if ticket.refunded {
            return Err(Error::AlreadyRefunded);
        }

        let amount = ticket.price_paid;
        ticket.refunded = true;
        save(&env, &DataKey::Ticket(event_id, ticket_id), &ticket);
        event.balance -= amount;
        save(&env, &DataKey::Event(event_id), &event);
        pay_out(&env, &owner, amount)?;

        TicketRefunded {
            event_id,
            owner,
            ticket_id,
            amount,
        }
        .publish(&env);
        Ok(amount)
    }

    /// Sponsor claims back everything they contributed to a cancelled event.
    pub fn refund_sponsorship(env: Env, sponsor: Address, event_id: u32) -> Result<i128, Error> {
        sponsor.require_auth();

        let mut event = load_event(&env, event_id)?;
        if event.status != EventStatus::Cancelled {
            return Err(Error::EventNotCancelled);
        }

        let amount = load_sponsor_total(&env, event_id, &sponsor);
        if amount == 0 {
            return Err(Error::NothingToRefund);
        }

        save(
            &env,
            &DataKey::SponsorTotal(event_id, sponsor.clone()),
            &0i128,
        );
        event.balance -= amount;
        save(&env, &DataKey::Event(event_id), &event);
        pay_out(&env, &sponsor, amount)?;

        SponsorshipRefunded {
            event_id,
            sponsor,
            amount,
        }
        .publish(&env);
        Ok(amount)
    }

    /// Organizer releases escrowed funds to a recipient (a worker, vendor, or
    /// venue) after the event has ended. Every release is stored as a public
    /// payout record with a memo explaining what it pays for.
    /// Returns the payout ID.
    pub fn release_funds(
        env: Env,
        organizer: Address,
        event_id: u32,
        recipient: Address,
        amount: i128,
        memo: String,
    ) -> Result<u32, Error> {
        organizer.require_auth();

        let mut event = load_event(&env, event_id)?;
        if event.organizer != organizer {
            return Err(Error::NotOrganizer);
        }
        if event.status != EventStatus::Ended {
            return Err(Error::EventNotEnded);
        }
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if amount > event.balance {
            return Err(Error::InsufficientFunds);
        }
        if memo.is_empty() || memo.len() > MAX_MEMO_LEN {
            return Err(Error::InvalidMemo);
        }

        let payout_id = load_u32(&env, &DataKey::PayoutCounter(event_id));
        save(&env, &DataKey::PayoutCounter(event_id), &(payout_id + 1));
        save(
            &env,
            &DataKey::Payout(event_id, payout_id),
            &Payout {
                recipient: recipient.clone(),
                amount,
                memo: memo.clone(),
                timestamp: env.ledger().timestamp(),
                receipt_hash: None,
                receipt_uri: None,
            },
        );
        save(
            &env,
            &DataKey::TotalReleased(event_id),
            &(load_total_released(&env, event_id) + amount),
        );
        event.balance -= amount;
        save(&env, &DataKey::Event(event_id), &event);
        pay_out(&env, &recipient, amount)?;

        FundsReleased {
            event_id,
            recipient,
            payout_id,
            amount,
            memo,
        }
        .publish(&env);
        Ok(payout_id)
    }

    /// Organizer attaches a receipt hash (and optionally where to find the
    /// document itself) to a payout as proof of spend. Settable only once
    /// per payout — a receipt is evidence, not something to revise later.
    pub fn attach_receipt(
        env: Env,
        organizer: Address,
        event_id: u32,
        payout_id: u32,
        receipt_hash: BytesN<32>,
        receipt_uri: Option<String>,
    ) -> Result<(), Error> {
        organizer.require_auth();

        let event = load_event(&env, event_id)?;
        if event.organizer != organizer {
            return Err(Error::NotOrganizer);
        }
        if let Some(uri) = &receipt_uri {
            if uri.len() > MAX_RECEIPT_URI_LEN {
                return Err(Error::InvalidReceiptUri);
            }
        }

        let key = DataKey::Payout(event_id, payout_id);
        let mut payout: Payout = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::PayoutNotFound)?;
        if payout.receipt_hash.is_some() {
            return Err(Error::ReceiptAlreadyAttached);
        }

        payout.receipt_hash = Some(receipt_hash.clone());
        payout.receipt_uri = receipt_uri;
        save(&env, &key, &payout);

        ReceiptAttached {
            event_id,
            payout_id,
            receipt_hash,
        }
        .publish(&env);
        Ok(())
    }

    // ─── Queries — readable by anyone ────────────────────────────────────────

    pub fn get_event(env: Env, event_id: u32) -> Result<Event, Error> {
        load_event(&env, event_id)
    }

    pub fn get_tiers(env: Env, event_id: u32) -> Result<Vec<TicketTier>, Error> {
        load_tiers(&env, event_id)
    }

    pub fn get_ticket(env: Env, event_id: u32, ticket_id: u32) -> Result<Ticket, Error> {
        require_event_exists(&env, event_id)?;
        load_ticket(&env, event_id, ticket_id)
    }

    /// Returns up to `limit` (capped at `MAX_OWNER_TICKET_PAGE`) ticket IDs
    /// `owner` has bought or received by transfer, starting at `start`.
    ///
    /// This is a history, not a live "currently owned" list: a ticket appears
    /// here under every address that has ever held it, including one it was
    /// later transferred away from. Check `get_ticket(event_id, id).owner`
    /// against `owner` to find out which of these are still current.
    pub fn get_tickets_by_owner(
        env: Env,
        event_id: u32,
        owner: Address,
        start: u32,
        limit: u32,
    ) -> Result<Vec<u32>, Error> {
        require_event_exists(&env, event_id)?;
        let count = load_u32(&env, &DataKey::OwnerTicketCount(event_id, owner.clone()));
        let end = start
            .saturating_add(limit.min(MAX_OWNER_TICKET_PAGE))
            .min(count);

        let mut out = Vec::new(&env);
        for i in start..end {
            let ticket_id: u32 = env
                .storage()
                .persistent()
                .get(&DataKey::OwnerTicket(event_id, owner.clone(), i))
                .expect("owner ticket index within count must exist");
            out.push_back(ticket_id);
        }
        Ok(out)
    }

    /// Returns how many tickets `owner` has ever bought or received by
    /// transfer for an event (see `get_tickets_by_owner` for what that means).
    pub fn owner_ticket_count(env: Env, event_id: u32, owner: Address) -> Result<u32, Error> {
        require_event_exists(&env, event_id)?;
        Ok(load_u32(&env, &DataKey::OwnerTicketCount(event_id, owner)))
    }

    /// Returns how much a sponsor currently has contributed (and not yet been
    /// refunded) for an event.
    pub fn get_sponsor_total(env: Env, event_id: u32, sponsor: Address) -> Result<i128, Error> {
        require_event_exists(&env, event_id)?;
        Ok(load_sponsor_total(&env, event_id, &sponsor))
    }

    /// Returns up to `limit` sponsorships (capped at `MAX_SPONSORSHIP_PAGE`)
    /// starting at `start`, ordered by the time they were made. `start` past
    /// the end returns an empty page rather than an error.
    pub fn get_sponsorships(
        env: Env,
        event_id: u32,
        start: u32,
        limit: u32,
    ) -> Result<Vec<Sponsorship>, Error> {
        require_event_exists(&env, event_id)?;
        let count = load_u32(&env, &DataKey::SponsorshipCount(event_id));
        let end = start
            .saturating_add(limit.min(MAX_SPONSORSHIP_PAGE))
            .min(count);

        let mut out = Vec::new(&env);
        for i in start..end {
            let sponsorship: Sponsorship = env
                .storage()
                .persistent()
                .get(&DataKey::Sponsorship(event_id, i))
                .expect("sponsorship index within count must exist");
            out.push_back(sponsorship);
        }
        Ok(out)
    }

    pub fn event_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::EventCounter)
            .unwrap_or(0)
    }

    pub fn ticket_count(env: Env, event_id: u32) -> Result<u32, Error> {
        require_event_exists(&env, event_id)?;
        Ok(env
            .storage()
            .persistent()
            .get(&DataKey::TicketCounter(event_id))
            .unwrap_or(0))
    }

    /// Returns the total number of sponsorship contributions for an event.
    pub fn sponsor_count(env: Env, event_id: u32) -> Result<u32, Error> {
        require_event_exists(&env, event_id)?;
        Ok(load_u32(&env, &DataKey::SponsorshipCount(event_id)))
    }

    /// Returns the number of ticket tiers for an event.
    pub fn tier_count(env: Env, event_id: u32) -> Result<u32, Error> {
        Ok(load_tiers(&env, event_id)?.len())
    }

    /// Returns the token contract address configured at deployment.
    pub fn get_token(env: Env) -> Result<Address, Error> {
        load_token(&env)
    }

    pub fn get_balance(env: Env, event_id: u32) -> Result<i128, Error> {
        Ok(load_event(&env, event_id)?.balance)
    }

    pub fn get_organizer(env: Env, event_id: u32) -> Result<Address, Error> {
        Ok(load_event(&env, event_id)?.organizer)
    }

    pub fn get_payout(env: Env, event_id: u32, payout_id: u32) -> Result<Payout, Error> {
        require_event_exists(&env, event_id)?;
        env.storage()
            .persistent()
            .get(&DataKey::Payout(event_id, payout_id))
            .ok_or(Error::PayoutNotFound)
    }

    /// Returns the number of payouts released for an event.
    pub fn payout_count(env: Env, event_id: u32) -> Result<u32, Error> {
        require_event_exists(&env, event_id)?;
        Ok(load_u32(&env, &DataKey::PayoutCounter(event_id)))
    }

    /// Returns the total USDC released from an event's escrow. Together with
    /// `get_balance`, this shows how much was raised and how much was spent.
    pub fn total_released(env: Env, event_id: u32) -> Result<i128, Error> {
        require_event_exists(&env, event_id)?;
        Ok(load_total_released(&env, event_id))
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
mod property_tests;
