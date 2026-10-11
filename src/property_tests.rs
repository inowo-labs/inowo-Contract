//! Property-based tests for the escrow's accounting.
//!
//! Each case runs a random sequence of contract calls — valid and invalid —
//! against a fresh contract, while a plain-Rust model tracks what *should*
//! have happened. After every step the contract and token balances must agree
//! with the model exactly, and failed calls must leave state untouched.

extern crate std;

use super::*;
use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig},
    token::{Client as TokenClient, StellarAssetClient},
    vec, Address, Env, String,
};
use std::{collections::BTreeMap, vec::Vec as StdVec};

const USERS: usize = 4;
const RECIPIENTS: usize = 2;
/// Event IDs are drawn from 0..=MAX_EVENT_ID, so some calls target events that
/// don't exist yet.
const MAX_EVENT_ID: u32 = 3;
const START_BALANCE: i128 = 1_000 * 10_000_000;
/// (price, supply cap) of the tiers every generated event is created with.
/// Small caps make "sold out" reachable.
const TIERS: [(i128, u32); 2] = [(10_000_000, 3), (50_000_000, 2)];

/// Who makes a call that only one party may make (an organizer action, a ticket
/// refund, a sponsorship refund). Mostly the rightful party, so sequences actually
/// reach ended and cancelled events and claim refunds; sometimes someone else.
#[derive(Debug, Clone, Copy)]
enum Who {
    /// The k-th rightful party: the organizer, or the k-th existing ticket's owner,
    /// or the k-th sponsor. Small k makes repeat claims by the same party common.
    Rightful(usize),
    User(usize),
}

#[derive(Debug, Clone)]
enum Op {
    Create {
        organizer: usize,
    },
    Sponsor {
        user: usize,
        event: u32,
        amount: i128,
    },
    Buy {
        user: usize,
        event: u32,
        tier: u32,
    },
    Redeem {
        caller: Who,
        event: u32,
        ticket: u32,
    },
    End {
        caller: Who,
        event: u32,
    },
    Cancel {
        caller: Who,
        event: u32,
    },
    Release {
        caller: Who,
        event: u32,
        recipient: usize,
        amount: i128,
    },
    RefundTicket {
        user: Who,
        event: u32,
        ticket: u32,
    },
    RefundSponsorship {
        user: Who,
        event: u32,
    },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    let user = 0..USERS;
    let who =
        prop_oneof![4 => (0..2usize).prop_map(Who::Rightful), 1 => (0..USERS).prop_map(Who::User)];
    let event = 0..=MAX_EVENT_ID;
    let ticket = 0u32..6;
    let amount = prop_oneof![
        1 => Just(0i128),
        1 => Just(-1i128),
        8 => 1i128..=300_000_000,
    ];
    prop_oneof![
        1 => (0..2usize).prop_map(|organizer| Op::Create { organizer }),
        4 => (user.clone(), event.clone(), amount.clone())
            .prop_map(|(user, event, amount)| Op::Sponsor { user, event, amount }),
        4 => (user.clone(), event.clone(), 0u32..3)
            .prop_map(|(user, event, tier)| Op::Buy { user, event, tier }),
        1 => (who.clone(), event.clone(), ticket.clone())
            .prop_map(|(caller, event, ticket)| Op::Redeem { caller, event, ticket }),
        2 => (who.clone(), event.clone()).prop_map(|(caller, event)| Op::End { caller, event }),
        2 => (who.clone(), event.clone()).prop_map(|(caller, event)| Op::Cancel { caller, event }),
        3 => (who.clone(), event.clone(), 0..RECIPIENTS, amount)
            .prop_map(|(caller, event, recipient, amount)| Op::Release { caller, event, recipient, amount }),
        3 => (who.clone(), event.clone(), ticket)
            .prop_map(|(user, event, ticket)| Op::RefundTicket { user, event, ticket }),
        3 => (who.clone(), event).prop_map(|(user, event)| Op::RefundSponsorship { user, event }),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Active,
    Ended,
    Cancelled,
}

#[derive(Debug)]
struct ModelEvent {
    organizer: usize,
    status: Status,
    sold: [u32; 2],
    sponsored: i128,
    sales: i128,
    released: i128,
    refunded: i128,
    /// Unrefunded sponsorship total per user.
    sponsor_totals: [i128; USERS],
    /// Everyone who has ever sponsored, in order, including those since refunded.
    sponsors: StdVec<usize>,
}

#[derive(Debug)]
struct ModelTicket {
    owner: usize,
    price: i128,
    redeemed: bool,
    refunded: bool,
}

/// What the contract *should* hold, built only from calls that succeeded.
#[derive(Default)]
struct Model {
    events: StdVec<ModelEvent>,
    tickets: BTreeMap<(u32, u32), ModelTicket>,
    user_balances: [i128; USERS],
    recipient_balances: [i128; RECIPIENTS],
}

struct Harness<'a> {
    env: Env,
    contract: Address,
    client: InowoContractClient<'a>,
    token: TokenClient<'a>,
    users: StdVec<Address>,
    recipients: StdVec<Address>,
    model: Model,
}

/// Every balance an invariant depends on, compared before and after a failed call.
#[derive(Debug, PartialEq)]
struct Snapshot {
    contract: i128,
    events: StdVec<i128>,
    users: StdVec<i128>,
    recipients: StdVec<i128>,
    event_count: u32,
}

impl Harness<'_> {
    fn new() -> Self {
        let env = Env::new_with_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        env.mock_all_auths();

        let token_admin = Address::generate(&env);
        let token_addr = env
            .register_stellar_asset_contract_v2(token_admin)
            .address();
        let contract = env.register(InowoContract, (token_addr.clone(),));
        let client = InowoContractClient::new(&env, &contract);
        let token = TokenClient::new(&env, &token_addr);

        let minter = StellarAssetClient::new(&env, &token_addr);
        let users: StdVec<Address> = (0..USERS).map(|_| Address::generate(&env)).collect();
        for user in &users {
            minter.mint(user, &START_BALANCE);
        }
        let recipients = (0..RECIPIENTS).map(|_| Address::generate(&env)).collect();

        let model = Model {
            user_balances: [START_BALANCE; USERS],
            ..Model::default()
        };

        Harness {
            env,
            contract,
            client,
            token,
            users,
            recipients,
            model,
        }
    }

    fn snapshot(&self) -> Snapshot {
        let event_count = self.client.event_count();
        Snapshot {
            contract: self.token.balance(&self.contract),
            events: (0..event_count)
                .map(|id| self.client.get_balance(&id))
                .collect(),
            users: self.users.iter().map(|u| self.token.balance(u)).collect(),
            recipients: self
                .recipients
                .iter()
                .map(|r| self.token.balance(r))
                .collect(),
            event_count,
        }
    }

    fn event(&self, id: u32) -> Option<&ModelEvent> {
        self.model.events.get(id as usize)
    }

    fn resolve(&self, who: Who, event: u32) -> usize {
        match who {
            Who::Rightful(_) => self.event(event).map_or(0, |e| e.organizer),
            Who::User(user) => user,
        }
    }

    /// Returns (claimant, ticket id).
    fn resolve_ticket(&self, who: Who, event: u32, ticket: u32) -> (usize, u32) {
        match who {
            Who::Rightful(k) => {
                let ids: StdVec<u32> = self
                    .model
                    .tickets
                    .keys()
                    .filter(|(e, _)| *e == event)
                    .map(|(_, id)| *id)
                    .collect();
                match ids.get(k % ids.len().max(1)) {
                    Some(id) => (self.model.tickets[&(event, *id)].owner, *id),
                    None => (0, ticket),
                }
            }
            Who::User(user) => (user, ticket),
        }
    }

    fn resolve_sponsor(&self, who: Who, event: u32) -> usize {
        match who {
            Who::Rightful(k) => self
                .event(event)
                .and_then(|e| e.sponsors.get(k % e.sponsors.len().max(1)).copied())
                .unwrap_or(0),
            Who::User(user) => user,
        }
    }

    /// Applies one operation. Returns whether the contract accepted it, after
    /// asserting the contract agreed with the model about whether it should.
    fn apply(&mut self, op: &Op) -> bool {
        match *op {
            Op::Create { organizer } => {
                let tiers = vec![
                    &self.env,
                    TierInput {
                        name: String::from_str(&self.env, "General"),
                        price: TIERS[0].0,
                        supply_cap: TIERS[0].1,
                    },
                    TierInput {
                        name: String::from_str(&self.env, "VIP"),
                        price: TIERS[1].0,
                        supply_cap: TIERS[1].1,
                    },
                ];
                let id = self.client.create_event(
                    &self.users[organizer],
                    &String::from_str(&self.env, "Generated event"),
                    &String::from_str(&self.env, "desc"),
                    &String::from_str(&self.env, "venue"),
                    &1_900_000_000,
                    &1_000_000_000,
                    &tiers,
                );
                assert_eq!(id as usize, self.model.events.len());
                self.model.events.push(ModelEvent {
                    organizer,
                    status: Status::Active,
                    sold: [0; 2],
                    sponsored: 0,
                    sales: 0,
                    released: 0,
                    refunded: 0,
                    sponsor_totals: [0; USERS],
                    sponsors: StdVec::new(),
                });
                true
            }

            Op::Sponsor {
                user,
                event,
                amount,
            } => {
                let expect = amount > 0
                    && self
                        .event(event)
                        .is_some_and(|e| e.status == Status::Active);
                let ok = self
                    .client
                    .try_sponsor_event(&self.users[user], &event, &amount)
                    .is_ok();
                assert_eq!(ok, expect, "sponsor {op:?}");
                if ok {
                    let e = &mut self.model.events[event as usize];
                    e.sponsored += amount;
                    e.sponsor_totals[user] += amount;
                    if !e.sponsors.contains(&user) {
                        e.sponsors.push(user);
                    }
                    self.model.user_balances[user] -= amount;
                }
                ok
            }

            Op::Buy { user, event, tier } => {
                let expect = self.event(event).is_some_and(|e| {
                    e.status == Status::Active
                        && (tier as usize) < TIERS.len()
                        && e.sold[tier as usize] < TIERS[tier as usize].1
                });
                let result = self.client.try_buy_ticket(&self.users[user], &event, &tier);
                assert_eq!(result.is_ok(), expect, "buy {op:?}");
                if let Ok(Ok(ticket_id)) = result {
                    let price = TIERS[tier as usize].0;
                    let e = &mut self.model.events[event as usize];
                    e.sold[tier as usize] += 1;
                    e.sales += price;
                    self.model.user_balances[user] -= price;
                    self.model.tickets.insert(
                        (event, ticket_id),
                        ModelTicket {
                            owner: user,
                            price,
                            redeemed: false,
                            refunded: false,
                        },
                    );
                    true
                } else {
                    false
                }
            }

            Op::Redeem {
                caller,
                event,
                ticket,
            } => {
                let caller = self.resolve(caller, event);
                let expect = self.event(event).is_some_and(|e| {
                    e.organizer == caller
                        && e.status != Status::Cancelled
                        && self
                            .model
                            .tickets
                            .get(&(event, ticket))
                            .is_some_and(|t| !t.redeemed)
                });
                let ok = self
                    .client
                    .try_redeem_ticket(&self.users[caller], &event, &ticket)
                    .is_ok();
                assert_eq!(ok, expect, "redeem {op:?}");
                if ok {
                    self.model
                        .tickets
                        .get_mut(&(event, ticket))
                        .unwrap()
                        .redeemed = true;
                }
                ok
            }

            Op::End { caller, event } | Op::Cancel { caller, event } => {
                let caller = self.resolve(caller, event);
                let ending = matches!(op, Op::End { .. });
                let expect = self
                    .event(event)
                    .is_some_and(|e| e.organizer == caller && e.status == Status::Active);
                let ok = if ending {
                    self.client
                        .try_end_event(&self.users[caller], &event)
                        .is_ok()
                } else {
                    self.client
                        .try_cancel_event(&self.users[caller], &event)
                        .is_ok()
                };
                assert_eq!(ok, expect, "end/cancel {op:?}");
                if ok {
                    self.model.events[event as usize].status = if ending {
                        Status::Ended
                    } else {
                        Status::Cancelled
                    };
                }
                ok
            }

            Op::Release {
                caller,
                event,
                recipient,
                amount,
            } => {
                let caller = self.resolve(caller, event);
                let expect = self.event(event).is_some_and(|e| {
                    let balance = e.sponsored + e.sales - e.released - e.refunded;
                    e.organizer == caller
                        && e.status == Status::Ended
                        && amount > 0
                        && amount <= balance
                });
                let ok = self
                    .client
                    .try_release_funds(
                        &self.users[caller],
                        &event,
                        &self.recipients[recipient],
                        &amount,
                        &String::from_str(&self.env, "Generated payout"),
                    )
                    .is_ok();
                assert_eq!(ok, expect, "release {op:?}");
                if ok {
                    self.model.events[event as usize].released += amount;
                    self.model.recipient_balances[recipient] += amount;
                }
                ok
            }

            Op::RefundTicket {
                user,
                event,
                ticket,
            } => {
                let (user, ticket) = self.resolve_ticket(user, event, ticket);
                let expect = self.event(event).is_some_and(|e| {
                    e.status == Status::Cancelled
                        && self
                            .model
                            .tickets
                            .get(&(event, ticket))
                            .is_some_and(|t| t.owner == user && !t.refunded)
                });
                let result = self
                    .client
                    .try_refund_ticket(&self.users[user], &event, &ticket);
                assert_eq!(result.is_ok(), expect, "refund ticket {op:?}");
                if let Ok(Ok(amount)) = result {
                    let t = self.model.tickets.get_mut(&(event, ticket)).unwrap();
                    assert_eq!(amount, t.price, "refund must return the price paid");
                    t.refunded = true;
                    self.model.events[event as usize].refunded += amount;
                    self.model.user_balances[user] += amount;
                    true
                } else {
                    false
                }
            }

            Op::RefundSponsorship { user, event } => {
                let user = self.resolve_sponsor(user, event);
                let owed = self.event(event).map(|e| e.sponsor_totals[user]);
                let expect = self
                    .event(event)
                    .is_some_and(|e| e.status == Status::Cancelled)
                    && owed.unwrap_or(0) > 0;
                let result = self
                    .client
                    .try_refund_sponsorship(&self.users[user], &event);
                assert_eq!(result.is_ok(), expect, "refund sponsorship {op:?}");
                if let Ok(Ok(amount)) = result {
                    assert_eq!(Some(amount), owed, "refund must return the sponsor's total");
                    let e = &mut self.model.events[event as usize];
                    e.sponsor_totals[user] = 0;
                    e.refunded += amount;
                    self.model.user_balances[user] += amount;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn check_invariants(&self) {
        let snap = self.snapshot();
        assert_eq!(snap.event_count as usize, self.model.events.len());

        let mut total_held = 0;
        for (id, e) in self.model.events.iter().enumerate() {
            let expected = e.sponsored + e.sales - e.released - e.refunded;
            assert_eq!(snap.events[id], expected, "event {id} escrow balance");
            assert!(snap.events[id] >= 0, "event {id} balance went negative");
            if e.status == Status::Cancelled {
                assert_eq!(e.released, 0, "cancelled event {id} released funds");
            }
            total_held += expected;
        }
        assert_eq!(
            snap.contract, total_held,
            "contract USDC != sum of event balances"
        );

        for (i, expected) in self.model.user_balances.iter().enumerate() {
            assert_eq!(snap.users[i], *expected, "user {i} balance");
        }
        for (i, expected) in self.model.recipient_balances.iter().enumerate() {
            assert_eq!(snap.recipients[i], *expected, "recipient {i} balance");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // CI runs 64 cases; set PROPTEST_CASES (e.g. 2000) for a deeper local search.
        cases: std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(64),
        max_shrink_iters: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn escrow_accounting_holds_for_any_sequence(
        ops in prop::collection::vec(op_strategy(), 1..60)
    ) {
        let mut h = Harness::new();
        // Start with events from two organizers so sequences spend their steps
        // funding, ending, cancelling, releasing, and refunding.
        for organizer in [0, 1, 0] {
            h.apply(&Op::Create { organizer });
        }
        for op in &ops {
            let before = h.snapshot();
            let accepted = h.apply(op);
            if !accepted {
                prop_assert_eq!(h.snapshot(), before, "rejected {:?} changed state", op);
            }
            h.check_invariants();
        }
    }
}
