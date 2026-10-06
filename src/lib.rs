#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token::Client as TokenClient, Address,
    Env, String, Vec,
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
}

/// A public sponsorship contribution record.
#[contracttype]
#[derive(Clone)]
pub struct Sponsorship {
    pub sponsor: Address,
    pub amount: i128,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Error codes are part of the contract's public interface — clients match on
/// the numeric value, so never renumber or reuse an existing code.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
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
    Sponsorships(u32),
}

// ─── Storage helpers ──────────────────────────────────────────────────────────

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

fn load_sponsorships(env: &Env, event_id: u32) -> Vec<Sponsorship> {
    env.storage()
        .persistent()
        .get(&DataKey::Sponsorships(event_id))
        .unwrap_or_else(|| Vec::new(env))
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct InowoContract;

#[contractimpl]
impl InowoContract {
    /// One-time setup: record the USDC token contract address.
    pub fn initialize(env: Env, token: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Token) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::EventCounter, &0u32);
        Ok(())
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

        let event_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::EventCounter)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::EventCounter, &(event_id + 1));

        env.storage().persistent().set(
            &DataKey::Event(event_id),
            &Event {
                organizer,
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
        env.storage()
            .persistent()
            .set(&DataKey::Tiers(event_id), &tier_list);
        env.storage()
            .persistent()
            .set(&DataKey::TicketCounter(event_id), &0u32);

        let empty_s: Vec<Sponsorship> = Vec::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Sponsorships(event_id), &empty_s);

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
        env.storage()
            .persistent()
            .set(&DataKey::Tiers(event_id), &tiers);

        event.balance += price;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);

        let ticket_id: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::TicketCounter(event_id))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::TicketCounter(event_id), &(ticket_id + 1));

        env.storage().persistent().set(
            &DataKey::Ticket(event_id, ticket_id),
            &Ticket {
                event_id,
                tier_index,
                owner: buyer,
                redeemed: false,
            },
        );

        Ok(ticket_id)
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

        let mut ticket: Ticket = env
            .storage()
            .persistent()
            .get(&DataKey::Ticket(event_id, ticket_id))
            .ok_or(Error::TicketNotFound)?;
        if ticket.redeemed {
            return Err(Error::AlreadyRedeemed);
        }

        ticket.redeemed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Ticket(event_id, ticket_id), &ticket);
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

        let mut sponsorships = load_sponsorships(&env, event_id);
        sponsorships.push_back(Sponsorship { sponsor, amount });
        env.storage()
            .persistent()
            .set(&DataKey::Sponsorships(event_id), &sponsorships);

        event.balance += amount;
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
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
        env.storage()
            .persistent()
            .set(&DataKey::Event(event_id), &event);
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
        env.storage()
            .persistent()
            .get(&DataKey::Ticket(event_id, ticket_id))
            .ok_or(Error::TicketNotFound)
    }

    pub fn get_sponsorships(env: Env, event_id: u32) -> Result<Vec<Sponsorship>, Error> {
        require_event_exists(&env, event_id)?;
        Ok(load_sponsorships(&env, event_id))
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
        Ok(load_sponsorships(&env, event_id).len())
    }

    /// Returns the number of ticket tiers for an event.
    pub fn tier_count(env: Env, event_id: u32) -> Result<u32, Error> {
        Ok(load_tiers(&env, event_id)?.len())
    }

    /// Returns the token contract address configured during initialize.
    pub fn get_token(env: Env) -> Result<Address, Error> {
        load_token(&env)
    }

    pub fn get_balance(env: Env, event_id: u32) -> Result<i128, Error> {
        Ok(load_event(&env, event_id)?.balance)
    }

    pub fn get_organizer(env: Env, event_id: u32) -> Result<Address, Error> {
        Ok(load_event(&env, event_id)?.organizer)
    }
}

#[cfg(test)]
mod test;
