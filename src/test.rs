extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{
        storage::{Instance as _, Persistent as _},
        Address as _, Events as _,
    },
    token::{Client as TokenClient, StellarAssetClient},
    vec, Address, Env, Event as _, String,
};
use std::vec::Vec as StdVec;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn setup(
    env: &Env,
) -> (
    Address,
    StellarAssetClient<'_>,
    Address,
    InowoContractClient<'_>,
) {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_addr = token_contract.address();
    let token_admin_client = StellarAssetClient::new(env, &token_addr);

    let contract_id = env.register(InowoContract, (token_addr.clone(),));
    let client = InowoContractClient::new(env, &contract_id);

    (token_addr, token_admin_client, contract_id, client)
}

fn default_tiers(env: &Env) -> Vec<TierInput> {
    vec![
        env,
        TierInput {
            name: String::from_str(env, "General"),
            price: 10_000_000_i128, // 1 USDC
            supply_cap: 100,
        },
        TierInput {
            name: String::from_str(env, "VIP"),
            price: 50_000_000_i128, // 5 USDC
            supply_cap: 20,
        },
    ]
}

fn create_test_event(env: &Env, client: &InowoContractClient, organizer: &Address) -> u32 {
    client.create_event(
        organizer,
        &String::from_str(env, "Stellar Summit"),
        &String::from_str(env, "The biggest Stellar dev conference"),
        &String::from_str(env, "San Francisco"),
        &1_750_000_000_u64,
        &500_000_000_i128,
        &default_tiers(env),
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[test]
fn test_create_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    assert_eq!(client.event_count(), 0);

    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(event_id, 0);
    assert_eq!(client.event_count(), 1);

    let event = client.get_event(&0);
    assert_eq!(event.organizer, organizer);
    assert_eq!(event.balance, 0);
    assert_eq!(event.status, EventStatus::Active);
    assert_eq!(event.funding_goal, 500_000_000_i128);

    let tiers = client.get_tiers(&0);
    assert_eq!(tiers.len(), 2);
    assert_eq!(tiers.get(0).unwrap().tickets_sold, 0);
    assert_eq!(tiers.get(1).unwrap().price, 50_000_000_i128);
}

#[test]
fn test_multiple_events_get_distinct_ids() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let id0 = create_test_event(&env, &client, &organizer);
    let id1 = create_test_event(&env, &client, &organizer);
    let id2 = create_test_event(&env, &client, &organizer);

    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(client.event_count(), 3);
}

#[test]
fn test_buy_ticket() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);

    token_admin.mint(&buyer, &100_000_000_i128); // 10 USDC

    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0); // General tier: 1 USDC

    assert_eq!(ticket_id, 0);
    assert_eq!(client.ticket_count(&event_id), 1);

    let ticket = client.get_ticket(&event_id, &ticket_id);
    assert_eq!(ticket.owner, buyer);
    assert_eq!(ticket.tier_index, 0);
    assert!(!ticket.redeemed);

    let event = client.get_event(&event_id);
    assert_eq!(event.balance, 10_000_000_i128);

    let token = TokenClient::new(&env, &token_addr);
    assert_eq!(token.balance(&buyer), 90_000_000_i128);

    let tiers = client.get_tiers(&event_id);
    assert_eq!(tiers.get(0).unwrap().tickets_sold, 1);
}

#[test]
fn test_buy_multiple_tickets_different_tiers() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer_a = Address::generate(&env);
    let buyer_b = Address::generate(&env);

    token_admin.mint(&buyer_a, &100_000_000_i128);
    token_admin.mint(&buyer_b, &500_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);

    let ticket_a = client.buy_ticket(&buyer_a, &event_id, &0); // General: 1 USDC
    let ticket_b = client.buy_ticket(&buyer_b, &event_id, &1); // VIP: 5 USDC

    assert_eq!(ticket_a, 0);
    assert_eq!(ticket_b, 1);

    let event = client.get_event(&event_id);
    assert_eq!(event.balance, 60_000_000_i128);

    let token = TokenClient::new(&env, &token_addr);
    assert_eq!(token.balance(&buyer_a), 90_000_000_i128);
    assert_eq!(token.balance(&buyer_b), 450_000_000_i128);
}

#[test]
fn test_redeem_ticket() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);

    token_admin.mint(&buyer, &50_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);

    assert!(!client.get_ticket(&event_id, &ticket_id).redeemed);

    client.redeem_ticket(&organizer, &event_id, &ticket_id);

    assert!(client.get_ticket(&event_id, &ticket_id).redeemed);
}

#[test]
fn test_sponsorship_is_publicly_recorded() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor_a = Address::generate(&env);
    let sponsor_b = Address::generate(&env);

    token_admin.mint(&sponsor_a, &1_000_000_000_i128);
    token_admin.mint(&sponsor_b, &1_000_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);

    client.sponsor_event(&sponsor_a, &event_id, &200_000_000_i128);
    client.sponsor_event(&sponsor_b, &event_id, &300_000_000_i128);

    let sponsorships = client.get_sponsorships(&event_id, &0, &50);
    assert_eq!(sponsorships.len(), 2);
    assert_eq!(sponsorships.get(0).unwrap().sponsor, sponsor_a);
    assert_eq!(sponsorships.get(0).unwrap().amount, 200_000_000_i128);
    assert_eq!(sponsorships.get(1).unwrap().sponsor, sponsor_b);
    assert_eq!(sponsorships.get(1).unwrap().amount, 300_000_000_i128);

    let event = client.get_event(&event_id);
    assert_eq!(event.balance, 500_000_000_i128);

    let token = TokenClient::new(&env, &token_addr);
    assert_eq!(token.balance(&sponsor_a), 800_000_000_i128);
    assert_eq!(token.balance(&sponsor_b), 700_000_000_i128);
}

#[test]
fn test_sold_out_tier_blocks_purchase() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let tiers = vec![
        &env,
        TierInput {
            name: String::from_str(&env, "Limited"),
            price: 10_000_000_i128,
            supply_cap: 1,
        },
    ];
    let event_id = client.create_event(
        &organizer,
        &String::from_str(&env, "Exclusive"),
        &String::from_str(&env, "One ticket only"),
        &String::from_str(&env, "Secret venue"),
        &1_750_000_000_u64,
        &10_000_000_i128,
        &tiers,
    );

    let buyer_a = Address::generate(&env);
    let buyer_b = Address::generate(&env);
    token_admin.mint(&buyer_a, &100_000_000_i128);
    token_admin.mint(&buyer_b, &100_000_000_i128);

    client.buy_ticket(&buyer_a, &event_id, &0);

    let result = client.try_buy_ticket(&buyer_b, &event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::TierSoldOut)));
}

#[test]
fn test_double_redeem_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);

    token_admin.mint(&buyer, &50_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);

    client.redeem_ticket(&organizer, &event_id, &ticket_id);

    let result = client.try_redeem_ticket(&organizer, &event_id, &ticket_id);
    assert_eq!(result.err(), Some(Ok(Error::AlreadyRedeemed)));
}

#[test]
fn test_end_event_changes_status_to_ended() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let event_id = create_test_event(&env, &client, &organizer);
    assert_eq!(client.get_event(&event_id).status, EventStatus::Active);

    client.end_event(&organizer, &event_id);
    assert_eq!(client.get_event(&event_id).status, EventStatus::Ended);
}

#[test]
fn test_buy_ticket_blocked_after_end_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);

    token_admin.mint(&buyer, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    client.end_event(&organizer, &event_id);

    let result = client.try_buy_ticket(&buyer, &event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::EventNotActive)));
}

#[test]
fn test_end_event_by_non_organizer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let impostor = Address::generate(&env);

    let event_id = create_test_event(&env, &client, &organizer);

    let result = client.try_end_event(&impostor, &event_id);
    assert_eq!(result.err(), Some(Ok(Error::NotOrganizer)));
}

#[test]
fn test_end_already_ended_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let event_id = create_test_event(&env, &client, &organizer);
    client.end_event(&organizer, &event_id);

    let result = client.try_end_event(&organizer, &event_id);
    assert_eq!(result.err(), Some(Ok(Error::EventNotActive)));
}

#[test]
fn test_sponsor_with_zero_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);

    token_admin.mint(&sponsor, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);

    let result = client.try_sponsor_event(&sponsor, &event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::InvalidAmount)));
}

#[test]
fn test_sponsor_rejected_on_ended_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);

    token_admin.mint(&sponsor, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    client.end_event(&organizer, &event_id);

    let result = client.try_sponsor_event(&sponsor, &event_id, &100_000_000_i128);
    assert_eq!(result.err(), Some(Ok(Error::EventNotActive)));
}

#[test]
fn test_sponsor_nonexistent_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &500_000_000_i128);

    let result = client.try_sponsor_event(&sponsor, &99, &100_000_000_i128);
    assert_eq!(result.err(), Some(Ok(Error::EventNotFound)));
}

#[test]
fn test_non_organizer_cannot_redeem_ticket() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    let impostor = Address::generate(&env);

    token_admin.mint(&buyer, &50_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);

    let result = client.try_redeem_ticket(&impostor, &event_id, &ticket_id);
    assert_eq!(result.err(), Some(Ok(Error::NotOrganizer)));
}

#[test]
fn test_get_token_returns_configured_address() {
    let env = Env::default();
    env.mock_all_auths();
    let (token_addr, _, _, client) = setup(&env);
    assert_eq!(client.get_token(), token_addr);
}

#[test]
fn test_get_balance_reflects_ticket_and_sponsor_payments() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    let sponsor = Address::generate(&env);

    token_admin.mint(&buyer, &100_000_000_i128); // 10 USDC
    token_admin.mint(&sponsor, &500_000_000_i128); // 50 USDC

    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.get_balance(&event_id), 0);

    client.buy_ticket(&buyer, &event_id, &0); // 1 USDC
    assert_eq!(client.get_balance(&event_id), 10_000_000_i128);

    client.sponsor_event(&sponsor, &event_id, &200_000_000_i128); // 20 USDC
    assert_eq!(client.get_balance(&event_id), 210_000_000_i128);
}

#[test]
fn test_invalid_tier_index_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);

    token_admin.mint(&buyer, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);

    let result = client.try_buy_ticket(&buyer, &event_id, &99);
    assert_eq!(result.err(), Some(Ok(Error::InvalidTier)));
}

#[test]
fn test_create_event_with_no_tiers_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let result = client.try_create_event(
        &organizer,
        &String::from_str(&env, "Empty"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "venue"),
        &1_750_000_000_u64,
        &100_000_000_i128,
        &Vec::new(&env),
    );

    assert_eq!(result.err(), Some(Ok(Error::NoTiers)));
}

#[test]
fn test_create_event_with_negative_funding_goal_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let result = client.try_create_event(
        &organizer,
        &String::from_str(&env, "Bad Goal"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "venue"),
        &1_750_000_000_u64,
        &-1_i128,
        &default_tiers(&env),
    );

    assert_eq!(result.err(), Some(Ok(Error::InvalidFundingGoal)));
}

#[test]
fn test_zero_price_tier_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let bad_tiers = vec![
        &env,
        TierInput {
            name: String::from_str(&env, "Free"),
            price: 0,
            supply_cap: 50,
        },
    ];

    let result = client.try_create_event(
        &organizer,
        &String::from_str(&env, "Bad Event"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "venue"),
        &1_750_000_000_u64,
        &100_000_000_i128,
        &bad_tiers,
    );

    assert_eq!(result.err(), Some(Ok(Error::InvalidTierPrice)));
}

#[test]
fn test_zero_supply_cap_tier_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let bad_tiers = vec![
        &env,
        TierInput {
            name: String::from_str(&env, "Ghost"),
            price: 10_000_000_i128,
            supply_cap: 0,
        },
    ];

    let result = client.try_create_event(
        &organizer,
        &String::from_str(&env, "Bad Event"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "venue"),
        &1_750_000_000_u64,
        &100_000_000_i128,
        &bad_tiers,
    );

    assert_eq!(result.err(), Some(Ok(Error::InvalidSupplyCap)));
}

#[test]
fn test_get_organizer_returns_correct_address() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.get_organizer(&event_id), organizer);
}

#[test]
fn test_get_organizer_nonexistent_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);

    let result = client.try_get_organizer(&99);
    assert_eq!(result.err(), Some(Ok(Error::EventNotFound)));
}

#[test]
fn test_sponsor_count_returns_zero_with_no_sponsors() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.sponsor_count(&event_id), 0);
}

#[test]
fn test_sponsor_count_returns_correct_count_after_sponsorships() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor_a = Address::generate(&env);
    let sponsor_b = Address::generate(&env);

    token_admin.mint(&sponsor_a, &1_000_000_000_i128);
    token_admin.mint(&sponsor_b, &1_000_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);

    client.sponsor_event(&sponsor_a, &event_id, &100_000_000_i128);
    assert_eq!(client.sponsor_count(&event_id), 1);

    client.sponsor_event(&sponsor_b, &event_id, &200_000_000_i128);
    assert_eq!(client.sponsor_count(&event_id), 2);
}

#[test]
fn test_ticket_count_increments_with_each_purchase() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer_a = Address::generate(&env);
    let buyer_b = Address::generate(&env);

    token_admin.mint(&buyer_a, &100_000_000_i128);
    token_admin.mint(&buyer_b, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.ticket_count(&event_id), 0);

    client.buy_ticket(&buyer_a, &event_id, &0);
    assert_eq!(client.ticket_count(&event_id), 1);

    client.buy_ticket(&buyer_b, &event_id, &0);
    assert_eq!(client.ticket_count(&event_id), 2);
}

#[test]
fn test_tier_count_returns_correct_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    // default_tiers has 2 tiers (General + VIP)
    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.tier_count(&event_id), 2);
}

#[test]
fn test_buy_ticket_rejected_on_ended_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);

    token_admin.mint(&buyer, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    client.end_event(&organizer, &event_id);

    assert_eq!(client.get_event(&event_id).status, EventStatus::Ended);

    let result = client.try_buy_ticket(&buyer, &event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::EventNotActive)));
}

#[test]
fn test_queries_on_nonexistent_event_return_event_not_found() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let missing = 99_u32;

    assert_eq!(
        client.try_get_event(&missing).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_get_tiers(&missing).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_get_ticket(&missing, &0).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_get_sponsorships(&missing, &0, &50).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_ticket_count(&missing).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_sponsor_count(&missing).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_tier_count(&missing).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_get_balance(&missing).err(),
        Some(Ok(Error::EventNotFound))
    );
}

#[test]
fn test_get_ticket_nonexistent_ticket_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    let result = client.try_get_ticket(&event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::TicketNotFound)));
}

#[test]
fn test_redeem_nonexistent_ticket_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    let result = client.try_redeem_ticket(&organizer, &event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::TicketNotFound)));
}

#[test]
fn test_rejected_purchase_does_not_charge_buyer() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);

    let event_id = create_test_event(&env, &client, &organizer);
    client.end_event(&organizer, &event_id);

    let result = client.try_buy_ticket(&buyer, &event_id, &0);
    assert_eq!(result.err(), Some(Ok(Error::EventNotActive)));
    assert_eq!(
        TokenClient::new(&env, &token_addr).balance(&buyer),
        100_000_000_i128
    );
    assert_eq!(client.get_balance(&event_id), 0);
}

#[test]
fn test_error_codes_are_stable() {
    assert_eq!(Error::AlreadyInitialized as u32, 1);
    assert_eq!(Error::NotInitialized as u32, 2);
    assert_eq!(Error::EventNotFound as u32, 3);
    assert_eq!(Error::TicketNotFound as u32, 4);
    assert_eq!(Error::NotOrganizer as u32, 5);
    assert_eq!(Error::EventNotActive as u32, 6);
    assert_eq!(Error::NoTiers as u32, 7);
    assert_eq!(Error::InvalidFundingGoal as u32, 8);
    assert_eq!(Error::InvalidTierPrice as u32, 9);
    assert_eq!(Error::InvalidSupplyCap as u32, 10);
    assert_eq!(Error::InvalidTier as u32, 11);
    assert_eq!(Error::TierSoldOut as u32, 12);
    assert_eq!(Error::AlreadyRedeemed as u32, 13);
    assert_eq!(Error::InvalidAmount as u32, 14);
    assert_eq!(Error::EventNotCancelled as u32, 15);
    assert_eq!(Error::NotTicketOwner as u32, 16);
    assert_eq!(Error::AlreadyRefunded as u32, 17);
    assert_eq!(Error::NothingToRefund as u32, 18);
    assert_eq!(Error::EventNotEnded as u32, 19);
    assert_eq!(Error::InsufficientFunds as u32, 20);
    assert_eq!(Error::InvalidMemo as u32, 21);
    assert_eq!(Error::PayoutNotFound as u32, 22);
    assert_eq!(Error::TransferToSelf as u32, 23);
}

#[test]
fn test_constructor_sets_token() {
    let env = Env::default();
    let token = Address::generate(&env);
    let contract_id = env.register(InowoContract, (token.clone(),));
    let client = InowoContractClient::new(&env, &contract_id);

    assert_eq!(client.get_token(), token);
    assert_eq!(client.event_count(), 0);
}

// ─── Contract events ─────────────────────────────────────────────────────────

#[test]
fn test_create_event_emits_event_created() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(
        env.events().all().filter_by_contract(&contract_id),
        [EventCreated {
            event_id,
            organizer,
            funding_goal: 500_000_000,
            date_unix: 1_750_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_sponsor_event_emits_sponsored() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);

    client.sponsor_event(&sponsor, &event_id, &40_000_000_i128);

    assert_eq!(
        env.events().all().filter_by_contract(&contract_id),
        [Sponsored {
            event_id,
            sponsor,
            amount: 40_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_buy_ticket_emits_ticket_purchased() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);

    let ticket_id = client.buy_ticket(&buyer, &event_id, &1);

    assert_eq!(
        env.events().all().filter_by_contract(&contract_id),
        [TicketPurchased {
            event_id,
            buyer,
            ticket_id,
            tier_index: 1,
            price: 50_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_redeem_ticket_emits_ticket_redeemed() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);

    client.redeem_ticket(&organizer, &event_id, &ticket_id);

    assert_eq!(
        env.events().all().filter_by_contract(&contract_id),
        [TicketRedeemed {
            event_id,
            ticket_id,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_end_event_emits_event_ended_with_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor, &event_id, &30_000_000_i128);

    client.end_event(&organizer, &event_id);

    assert_eq!(
        env.events().all().filter_by_contract(&contract_id),
        [EventEnded {
            event_id,
            balance: 30_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_failed_call_emits_no_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    let _ = client.try_sponsor_event(&organizer, &event_id, &0);

    assert_eq!(env.events().all().filter_by_contract(&contract_id), []);
}

// ─── Storage TTL ─────────────────────────────────────────────────────────────

#[test]
fn test_writes_extend_storage_ttl() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);

    env.as_contract(&contract_id, || {
        let persistent = env.storage().persistent();
        for key in [
            DataKey::Event(event_id),
            DataKey::Tiers(event_id),
            DataKey::TicketCounter(event_id),
            DataKey::Ticket(event_id, ticket_id),
        ] {
            assert_eq!(persistent.get_ttl(&key), TTL_EXTEND_TO);
        }
        assert_eq!(env.storage().instance().get_ttl(), TTL_EXTEND_TO);
    });
}

// ─── Cancellation and refunds ────────────────────────────────────────────────

#[test]
fn test_cancel_event_sets_status_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor, &event_id, &25_000_000_i128);

    client.cancel_event(&organizer, &event_id);
    // events().all() only covers the latest invocation, so capture before reading state.
    let events = env.events().all().filter_by_contract(&contract_id);

    assert_eq!(client.get_event(&event_id).status, EventStatus::Cancelled);
    assert_eq!(
        events,
        [EventCancelled {
            event_id,
            balance: 25_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_cancel_event_by_non_organizer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let impostor = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    let result = client.try_cancel_event(&impostor, &event_id);
    assert_eq!(result.err(), Some(Ok(Error::NotOrganizer)));
}

#[test]
fn test_cancel_ended_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);
    client.end_event(&organizer, &event_id);

    let result = client.try_cancel_event(&organizer, &event_id);
    assert_eq!(result.err(), Some(Ok(Error::EventNotActive)));
}

#[test]
fn test_cancelled_event_blocks_sales_sponsorship_and_check_in() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);
    client.cancel_event(&organizer, &event_id);

    assert_eq!(
        client.try_buy_ticket(&buyer, &event_id, &0).err(),
        Some(Ok(Error::EventNotActive))
    );
    assert_eq!(
        client
            .try_sponsor_event(&buyer, &event_id, &10_000_000_i128)
            .err(),
        Some(Ok(Error::EventNotActive))
    );
    assert_eq!(
        client
            .try_redeem_ticket(&organizer, &event_id, &ticket_id)
            .err(),
        Some(Ok(Error::EventNotActive))
    );
}

#[test]
fn test_refund_ticket_returns_price_paid() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, contract_id, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &1); // VIP, 5 USDC
    client.cancel_event(&organizer, &event_id);

    let refunded = client.refund_ticket(&buyer, &event_id, &ticket_id);
    let events = env.events().all().filter_by_contract(&contract_id);

    assert_eq!(refunded, 50_000_000);
    assert_eq!(token.balance(&buyer), 100_000_000);
    assert_eq!(client.get_balance(&event_id), 0);
    assert!(client.get_ticket(&event_id, &ticket_id).refunded);
    assert_eq!(
        events,
        [TicketRefunded {
            event_id,
            owner: buyer,
            ticket_id,
            amount: 50_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_refund_redeemed_ticket_is_allowed() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);
    client.redeem_ticket(&organizer, &event_id, &ticket_id);
    client.cancel_event(&organizer, &event_id);

    assert_eq!(
        client.refund_ticket(&buyer, &event_id, &ticket_id),
        10_000_000
    );
}

#[test]
fn test_refund_ticket_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);
    client.cancel_event(&organizer, &event_id);
    client.refund_ticket(&buyer, &event_id, &ticket_id);

    let result = client.try_refund_ticket(&buyer, &event_id, &ticket_id);
    assert_eq!(result.err(), Some(Ok(Error::AlreadyRefunded)));
}

#[test]
fn test_refund_ticket_by_non_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    let thief = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);
    client.cancel_event(&organizer, &event_id);

    let result = client.try_refund_ticket(&thief, &event_id, &ticket_id);
    assert_eq!(result.err(), Some(Ok(Error::NotTicketOwner)));
}

#[test]
fn test_refund_ticket_on_active_or_ended_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let buyer = Address::generate(&env);
    token_admin.mint(&buyer, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&buyer, &event_id, &0);

    assert_eq!(
        client
            .try_refund_ticket(&buyer, &event_id, &ticket_id)
            .err(),
        Some(Ok(Error::EventNotCancelled))
    );

    client.end_event(&organizer, &event_id);
    assert_eq!(
        client
            .try_refund_ticket(&buyer, &event_id, &ticket_id)
            .err(),
        Some(Ok(Error::EventNotCancelled))
    );
}

#[test]
fn test_refund_sponsorship_returns_total_of_all_contributions() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, contract_id, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor, &event_id, &30_000_000_i128);
    client.sponsor_event(&sponsor, &event_id, &20_000_000_i128);
    assert_eq!(client.get_sponsor_total(&event_id, &sponsor), 50_000_000);
    client.cancel_event(&organizer, &event_id);

    let refunded = client.refund_sponsorship(&sponsor, &event_id);
    let events = env.events().all().filter_by_contract(&contract_id);

    assert_eq!(refunded, 50_000_000);
    assert_eq!(token.balance(&sponsor), 100_000_000);
    assert_eq!(client.get_sponsor_total(&event_id, &sponsor), 0);
    assert_eq!(client.get_balance(&event_id), 0);
    assert_eq!(
        events,
        [SponsorshipRefunded {
            event_id,
            sponsor,
            amount: 50_000_000,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_refund_sponsorship_twice_or_by_non_sponsor_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    let stranger = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor, &event_id, &30_000_000_i128);
    client.cancel_event(&organizer, &event_id);
    client.refund_sponsorship(&sponsor, &event_id);

    assert_eq!(
        client.try_refund_sponsorship(&sponsor, &event_id).err(),
        Some(Ok(Error::NothingToRefund))
    );
    assert_eq!(
        client.try_refund_sponsorship(&stranger, &event_id).err(),
        Some(Ok(Error::NothingToRefund))
    );
}

#[test]
fn test_refund_sponsorship_on_active_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor, &event_id, &30_000_000_i128);

    let result = client.try_refund_sponsorship(&sponsor, &event_id);
    assert_eq!(result.err(), Some(Ok(Error::EventNotCancelled)));
}

#[test]
fn test_all_refunds_return_every_stroop_collected() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, contract_id, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let buyers = [Address::generate(&env), Address::generate(&env)];
    let sponsors = [Address::generate(&env), Address::generate(&env)];
    for who in buyers.iter().chain(sponsors.iter()) {
        token_admin.mint(who, &200_000_000_i128);
    }

    let event_id = create_test_event(&env, &client, &organizer);
    let t0 = client.buy_ticket(&buyers[0], &event_id, &0);
    let t1 = client.buy_ticket(&buyers[1], &event_id, &1);
    let t2 = client.buy_ticket(&buyers[1], &event_id, &0);
    client.sponsor_event(&sponsors[0], &event_id, &70_000_000_i128);
    client.sponsor_event(&sponsors[1], &event_id, &15_000_000_i128);
    client.sponsor_event(&sponsors[0], &event_id, &5_000_000_i128);
    assert_eq!(token.balance(&contract_id), 160_000_000);

    client.cancel_event(&organizer, &event_id);
    client.refund_ticket(&buyers[0], &event_id, &t0);
    client.refund_ticket(&buyers[1], &event_id, &t1);
    client.refund_ticket(&buyers[1], &event_id, &t2);
    client.refund_sponsorship(&sponsors[0], &event_id);
    client.refund_sponsorship(&sponsors[1], &event_id);

    assert_eq!(client.get_balance(&event_id), 0);
    assert_eq!(token.balance(&contract_id), 0);
    for who in buyers.iter().chain(sponsors.iter()) {
        assert_eq!(token.balance(who), 200_000_000);
    }
}

// ─── Releasing funds ─────────────────────────────────────────────────────────

/// Creates an event, sponsors it with 60 USDC, sells one 1-USDC ticket, and
/// ends it — leaving 61 USDC in escrow ready for release.
fn ended_event_with_funds(
    env: &Env,
    client: &InowoContractClient,
    token_admin: &StellarAssetClient,
    organizer: &Address,
) -> u32 {
    let sponsor = Address::generate(env);
    let buyer = Address::generate(env);
    token_admin.mint(&sponsor, &600_000_000_i128);
    token_admin.mint(&buyer, &10_000_000_i128);

    let event_id = create_test_event(env, client, organizer);
    client.sponsor_event(&sponsor, &event_id, &600_000_000_i128);
    client.buy_ticket(&buyer, &event_id, &0);
    client.end_event(organizer, &event_id);
    event_id
}

#[test]
fn test_release_funds_pays_recipient_and_records_payout() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, contract_id, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let crew = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let memo = String::from_str(&env, "Sound crew");

    let payout_id = client.release_funds(&organizer, &event_id, &crew, &250_000_000_i128, &memo);
    let events = env.events().all().filter_by_contract(&contract_id);

    assert_eq!(payout_id, 0);
    assert_eq!(token.balance(&crew), 250_000_000);
    assert_eq!(client.get_balance(&event_id), 360_000_000);
    assert_eq!(client.total_released(&event_id), 250_000_000);
    assert_eq!(client.payout_count(&event_id), 1);

    let payout = client.get_payout(&event_id, &payout_id);
    assert_eq!(payout.recipient, crew);
    assert_eq!(payout.amount, 250_000_000);
    assert_eq!(payout.memo, memo);

    assert_eq!(
        events,
        [FundsReleased {
            event_id,
            recipient: crew,
            payout_id,
            amount: 250_000_000,
            memo,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_release_funds_can_pay_several_recipients_until_escrow_is_empty() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, contract_id, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let venue = Address::generate(&env);
    let caterer = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);

    client.release_funds(
        &organizer,
        &event_id,
        &venue,
        &400_000_000_i128,
        &String::from_str(&env, "Venue hire"),
    );
    let second = client.release_funds(
        &organizer,
        &event_id,
        &caterer,
        &210_000_000_i128,
        &String::from_str(&env, "Catering"),
    );

    assert_eq!(second, 1);
    assert_eq!(client.payout_count(&event_id), 2);
    assert_eq!(client.get_balance(&event_id), 0);
    assert_eq!(client.total_released(&event_id), 610_000_000);
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
fn test_release_more_than_balance_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);

    let result = client.try_release_funds(
        &organizer,
        &event_id,
        &recipient,
        &610_000_001_i128,
        &String::from_str(&env, "Too much"),
    );
    assert_eq!(result.err(), Some(Ok(Error::InsufficientFunds)));
}

#[test]
fn test_release_before_event_ends_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    let recipient = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor, &event_id, &100_000_000_i128);
    let memo = String::from_str(&env, "Early withdrawal");

    assert_eq!(
        client
            .try_release_funds(&organizer, &event_id, &recipient, &1_i128, &memo)
            .err(),
        Some(Ok(Error::EventNotEnded))
    );

    client.cancel_event(&organizer, &event_id);
    assert_eq!(
        client
            .try_release_funds(&organizer, &event_id, &recipient, &1_i128, &memo)
            .err(),
        Some(Ok(Error::EventNotEnded))
    );
}

#[test]
fn test_release_by_non_organizer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let impostor = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);

    let result = client.try_release_funds(
        &impostor,
        &event_id,
        &impostor,
        &10_000_000_i128,
        &String::from_str(&env, "Mine now"),
    );
    assert_eq!(result.err(), Some(Ok(Error::NotOrganizer)));
}

#[test]
fn test_release_non_positive_amount_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let memo = String::from_str(&env, "Nothing");

    for amount in [0_i128, -5_i128] {
        assert_eq!(
            client
                .try_release_funds(&organizer, &event_id, &recipient, &amount, &memo)
                .err(),
            Some(Ok(Error::InvalidAmount))
        );
    }
}

#[test]
fn test_release_requires_a_memo_within_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let recipient = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);

    let empty = String::from_str(&env, "");
    let too_long = String::from_bytes(&env, &[b'x'; (MAX_MEMO_LEN + 1) as usize]);
    let at_limit = String::from_bytes(&env, &[b'x'; MAX_MEMO_LEN as usize]);

    for memo in [empty, too_long] {
        assert_eq!(
            client
                .try_release_funds(&organizer, &event_id, &recipient, &1_i128, &memo)
                .err(),
            Some(Ok(Error::InvalidMemo))
        );
    }
    assert_eq!(
        client.release_funds(&organizer, &event_id, &recipient, &1_i128, &at_limit),
        0
    );
}

#[test]
fn test_payout_queries_on_missing_data() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.payout_count(&event_id), 0);
    assert_eq!(client.total_released(&event_id), 0);
    assert_eq!(
        client.try_get_payout(&event_id, &0).err(),
        Some(Ok(Error::PayoutNotFound))
    );
    assert_eq!(
        client.try_get_payout(&99, &0).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_payout_count(&99).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_total_released(&99).err(),
        Some(Ok(Error::EventNotFound))
    );
}

// ─── Ticket transfer ─────────────────────────────────────────────────────────

/// An active event where `buyer` holds ticket 0 (General, 1 USDC).
fn event_with_ticket(
    env: &Env,
    client: &InowoContractClient,
    token_admin: &StellarAssetClient,
    organizer: &Address,
    buyer: &Address,
) -> (u32, u32) {
    token_admin.mint(buyer, &100_000_000_i128);
    let event_id = create_test_event(env, client, organizer);
    let ticket_id = client.buy_ticket(buyer, &event_id, &0);
    (event_id, ticket_id)
}

#[test]
fn test_transfer_ticket_changes_owner_and_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);
    let events = env.events().all().filter_by_contract(&contract_id);

    let ticket = client.get_ticket(&event_id, &ticket_id);
    assert_eq!(ticket.owner, bob);
    assert_eq!(ticket.price_paid, 10_000_000);
    assert!(!ticket.redeemed && !ticket.refunded);
    assert_eq!(
        events,
        [TicketTransferred {
            event_id,
            from: alice,
            to: bob,
            ticket_id,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_transfer_ticket_moves_no_money() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, contract_id, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);

    assert_eq!(token.balance(&alice), 90_000_000);
    assert_eq!(token.balance(&bob), 0);
    assert_eq!(token.balance(&contract_id), 10_000_000);
    assert_eq!(client.get_balance(&event_id), 10_000_000);
    assert_eq!(client.ticket_count(&event_id), 1);
}

#[test]
fn test_ticket_can_be_transferred_along_a_chain_of_owners() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);
    client.transfer_ticket(&bob, &event_id, &ticket_id, &carol);

    assert_eq!(client.get_ticket(&event_id, &ticket_id).owner, carol);
    // The previous owners can no longer transfer it.
    assert_eq!(
        client
            .try_transfer_ticket(&alice, &event_id, &ticket_id, &bob)
            .err(),
        Some(Ok(Error::NotTicketOwner))
    );
    assert_eq!(
        client
            .try_transfer_ticket(&bob, &event_id, &ticket_id, &alice)
            .err(),
        Some(Ok(Error::NotTicketOwner))
    );
}

#[test]
fn test_transfer_ticket_by_non_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let thief = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    let result = client.try_transfer_ticket(&thief, &event_id, &ticket_id, &thief);
    assert_eq!(result.err(), Some(Ok(Error::NotTicketOwner)));
    assert_eq!(client.get_ticket(&event_id, &ticket_id).owner, alice);
}

#[test]
fn test_organizer_cannot_transfer_someone_elses_ticket() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    let result = client.try_transfer_ticket(&organizer, &event_id, &ticket_id, &organizer);
    assert_eq!(result.err(), Some(Ok(Error::NotTicketOwner)));
}

#[test]
fn test_transfer_redeemed_ticket_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);
    client.redeem_ticket(&organizer, &event_id, &ticket_id);

    let result = client.try_transfer_ticket(&alice, &event_id, &ticket_id, &bob);
    assert_eq!(result.err(), Some(Ok(Error::AlreadyRedeemed)));
    assert_eq!(client.get_ticket(&event_id, &ticket_id).owner, alice);
}

#[test]
fn test_transfer_on_ended_or_cancelled_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (ended, ended_ticket) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);
    let (cancelled, cancelled_ticket) =
        event_with_ticket(&env, &client, &token_admin, &organizer, &alice);
    client.end_event(&organizer, &ended);
    client.cancel_event(&organizer, &cancelled);

    assert_eq!(
        client
            .try_transfer_ticket(&alice, &ended, &ended_ticket, &bob)
            .err(),
        Some(Ok(Error::EventNotActive))
    );
    assert_eq!(
        client
            .try_transfer_ticket(&alice, &cancelled, &cancelled_ticket, &bob)
            .err(),
        Some(Ok(Error::EventNotActive))
    );
}

#[test]
fn test_transfer_ticket_to_self_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    let result = client.try_transfer_ticket(&alice, &event_id, &ticket_id, &alice);
    assert_eq!(result.err(), Some(Ok(Error::TransferToSelf)));
}

#[test]
fn test_transfer_missing_event_or_ticket_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (event_id, _) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    assert_eq!(
        client.try_transfer_ticket(&alice, &99, &0, &bob).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client
            .try_transfer_ticket(&alice, &event_id, &99, &bob)
            .err(),
        Some(Ok(Error::TicketNotFound))
    );
}

#[test]
fn test_failed_transfer_emits_no_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);

    let _ = client.try_transfer_ticket(&alice, &event_id, &ticket_id, &alice);

    assert_eq!(env.events().all().filter_by_contract(&contract_id), []);
}

#[test]
fn test_refund_after_transfer_goes_to_the_new_owner() {
    let env = Env::default();
    env.mock_all_auths();

    let (token_addr, token_admin, _, client) = setup(&env);
    let token = TokenClient::new(&env, &token_addr);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);
    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);
    client.cancel_event(&organizer, &event_id);

    // The original buyer no longer has any claim on the ticket...
    assert_eq!(
        client
            .try_refund_ticket(&alice, &event_id, &ticket_id)
            .err(),
        Some(Ok(Error::NotTicketOwner))
    );
    // ...and the current owner gets the price the ticket was bought for.
    assert_eq!(
        client.refund_ticket(&bob, &event_id, &ticket_id),
        10_000_000
    );
    assert_eq!(token.balance(&bob), 10_000_000);
    assert_eq!(token.balance(&alice), 90_000_000);
    assert_eq!(client.get_balance(&event_id), 0);
}

#[test]
fn test_new_owner_can_be_checked_in() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let (event_id, ticket_id) = event_with_ticket(&env, &client, &token_admin, &organizer, &alice);
    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);

    client.redeem_ticket(&organizer, &event_id, &ticket_id);

    let ticket = client.get_ticket(&event_id, &ticket_id);
    assert!(ticket.redeemed);
    assert_eq!(ticket.owner, bob);
}

// ─── Sponsorship storage (one entry per contribution) ────────────────────────

#[test]
fn test_get_sponsorships_pagination_bounds() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    let sponsors: StdVec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
    for (i, sponsor) in sponsors.iter().enumerate() {
        token_admin.mint(sponsor, &100_000_000_i128);
        client.sponsor_event(sponsor, &event_id, &(10_000_000_i128 * (i as i128 + 1)));
    }

    // A full page in order.
    let all = client.get_sponsorships(&event_id, &0, &50);
    assert_eq!(all.len(), 5);
    for (i, sponsor) in sponsors.iter().enumerate() {
        assert_eq!(all.get(i as u32).unwrap().sponsor, *sponsor);
        assert_eq!(
            all.get(i as u32).unwrap().amount,
            10_000_000 * (i as i128 + 1)
        );
    }

    // A page in the middle.
    let middle = client.get_sponsorships(&event_id, &1, &2);
    assert_eq!(middle.len(), 2);
    assert_eq!(middle.get(0).unwrap().sponsor, sponsors[1]);
    assert_eq!(middle.get(1).unwrap().sponsor, sponsors[2]);

    // limit longer than what's left is clamped to what exists.
    let tail = client.get_sponsorships(&event_id, &3, &50);
    assert_eq!(tail.len(), 2);
    assert_eq!(tail.get(0).unwrap().sponsor, sponsors[3]);
    assert_eq!(tail.get(1).unwrap().sponsor, sponsors[4]);

    // start at or past the end returns an empty page, not an error.
    assert_eq!(client.get_sponsorships(&event_id, &5, &50).len(), 0);
    assert_eq!(client.get_sponsorships(&event_id, &1000, &50).len(), 0);

    // limit of 0 returns an empty page.
    assert_eq!(client.get_sponsorships(&event_id, &0, &0).len(), 0);
}

#[test]
fn test_get_sponsorships_limit_is_capped_at_max_page() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    for _ in 0..(MAX_SPONSORSHIP_PAGE + 10) {
        let sponsor = Address::generate(&env);
        token_admin.mint(&sponsor, &10_000_000_i128);
        client.sponsor_event(&sponsor, &event_id, &1_000_000_i128);
    }

    // Asking for more than MAX_SPONSORSHIP_PAGE still returns at most that many.
    let page = client.get_sponsorships(&event_id, &0, &(MAX_SPONSORSHIP_PAGE + 10));
    assert_eq!(page.len(), MAX_SPONSORSHIP_PAGE);
}

#[test]
fn test_many_sponsorships_all_succeed_and_are_all_readable() {
    let env = Env::default();
    env.mock_all_auths();

    const N: u32 = 220;
    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    let sponsors: StdVec<Address> = (0..N)
        .map(|_| {
            let s = Address::generate(&env);
            token_admin.mint(&s, &10_000_000_i128);
            client.sponsor_event(&s, &event_id, &1_000_000_i128);
            s
        })
        .collect();

    assert_eq!(client.sponsor_count(&event_id), N);
    assert_eq!(client.get_balance(&event_id), 1_000_000 * N as i128);

    // Every contribution is readable by paging through in MAX_SPONSORSHIP_PAGE
    // chunks, and comes back in the order it was made.
    let mut seen: StdVec<Address> = StdVec::new();
    let mut start = 0u32;
    loop {
        let page = client.get_sponsorships(&event_id, &start, &MAX_SPONSORSHIP_PAGE);
        if page.is_empty() {
            break;
        }
        for s in page.iter() {
            seen.push(s.sponsor.clone());
        }
        start += MAX_SPONSORSHIP_PAGE;
    }
    assert_eq!(seen.len(), N as usize);
    assert_eq!(seen, sponsors);
}

/// Proves the fix: before this change, every sponsorship rewrote one growing
/// `Vec`, so each call wrote more bytes than the last, and the single entry
/// could eventually hit Soroban's storage-entry size limit — at which point
/// every further sponsorship for that event would fail outright, not just
/// cost more. Each contribution is now its own entry, so a single
/// `sponsor_event` call always touches the same number of entries and writes
/// the same number of bytes, however many sponsorships came before it.
#[test]
fn test_sponsor_event_write_footprint_does_not_grow_with_sponsorship_count() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = create_test_event(&env, &client, &organizer);

    // Soroban's cost_estimate() only reports the *last* top-level invocation,
    // so call once, record its footprint, call again, and compare.
    let sponsor_once = |amount: i128| -> (u32, u32) {
        let s = Address::generate(&env);
        token_admin.mint(&s, &amount);
        client.sponsor_event(&s, &event_id, &amount);
        let res = env.cost_estimate().resources();
        (res.write_entries, res.write_bytes)
    };

    let (entries_1st, bytes_1st) = sponsor_once(1_000_000);
    for _ in 0..198 {
        sponsor_once(1_000_000);
    }
    let (entries_200th, bytes_200th) = sponsor_once(1_000_000);

    assert_eq!(client.sponsor_count(&event_id), 200);
    assert_eq!(
        (entries_200th, bytes_200th),
        (entries_1st, bytes_1st),
        "the 200th sponsor_event call touched a different number of entries \
         or wrote a different number of bytes than the 1st — the old design \
         would have grown both with every call"
    );
}

#[test]
fn test_sponsorship_storage_extends_ttl() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let sponsor = Address::generate(&env);
    token_admin.mint(&sponsor, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);

    client.sponsor_event(&sponsor, &event_id, &10_000_000_i128);

    env.as_contract(&contract_id, || {
        let persistent = env.storage().persistent();
        for key in [
            DataKey::SponsorshipCount(event_id),
            DataKey::Sponsorship(event_id, 0),
        ] {
            assert_eq!(persistent.get_ttl(&key), TTL_EXTEND_TO);
        }
    });
}

// ─── Tickets by owner ─────────────────────────────────────────────────────────

#[test]
fn test_buy_ticket_indexes_it_under_the_buyer() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    token_admin.mint(&alice, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);

    assert_eq!(client.owner_ticket_count(&event_id, &alice), 0);

    let t0 = client.buy_ticket(&alice, &event_id, &0);
    let t1 = client.buy_ticket(&alice, &event_id, &1);

    assert_eq!(client.owner_ticket_count(&event_id, &alice), 2);
    assert_eq!(
        client.get_tickets_by_owner(&event_id, &alice, &0, &50),
        vec![&env, t0, t1]
    );
}

#[test]
fn test_tickets_by_owner_is_per_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    token_admin.mint(&alice, &200_000_000_i128);
    let event_a = create_test_event(&env, &client, &organizer);
    let event_b = create_test_event(&env, &client, &organizer);

    client.buy_ticket(&alice, &event_a, &0);

    assert_eq!(client.owner_ticket_count(&event_a, &alice), 1);
    assert_eq!(client.owner_ticket_count(&event_b, &alice), 0);
    assert_eq!(
        client.get_tickets_by_owner(&event_b, &alice, &0, &50).len(),
        0
    );
}

#[test]
fn test_transferred_ticket_is_indexed_under_the_new_owner_without_removing_history() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    token_admin.mint(&alice, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&alice, &event_id, &0);

    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);

    // The ticket now shows up under bob's history...
    assert_eq!(client.owner_ticket_count(&event_id, &bob), 1);
    assert_eq!(
        client.get_tickets_by_owner(&event_id, &bob, &0, &50),
        vec![&env, ticket_id]
    );
    // ...and the current owner, via get_ticket, confirms it's really bob's.
    assert_eq!(client.get_ticket(&event_id, &ticket_id).owner, bob);

    // alice's history still lists it (she once held it) - callers are
    // expected to check get_ticket().owner to find out it's no longer hers.
    assert_eq!(client.owner_ticket_count(&event_id, &alice), 1);
    let alice_ticket = client
        .get_tickets_by_owner(&event_id, &alice, &0, &50)
        .get(0)
        .unwrap();
    assert_eq!(alice_ticket, ticket_id);
    assert_ne!(client.get_ticket(&event_id, &alice_ticket).owner, alice);
}

#[test]
fn test_tickets_by_owner_follows_a_chain_of_transfers() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    token_admin.mint(&alice, &100_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);
    let ticket_id = client.buy_ticket(&alice, &event_id, &0);

    client.transfer_ticket(&alice, &event_id, &ticket_id, &bob);
    client.transfer_ticket(&bob, &event_id, &ticket_id, &carol);

    assert_eq!(client.owner_ticket_count(&event_id, &carol), 1);
    assert_eq!(client.get_ticket(&event_id, &ticket_id).owner, carol);
    // bob held it only briefly, but it's still in his history too.
    assert_eq!(client.owner_ticket_count(&event_id, &bob), 1);
}

#[test]
fn test_get_tickets_by_owner_pagination_bounds() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    token_admin.mint(&alice, &500_000_000_i128);
    let event_id = create_test_event(&env, &client, &organizer);

    let bought: StdVec<u32> = (0..5)
        .map(|_| client.buy_ticket(&alice, &event_id, &0))
        .collect();

    let all = client.get_tickets_by_owner(&event_id, &alice, &0, &50);
    assert_eq!(all.len(), 5);
    for (i, id) in bought.iter().enumerate() {
        assert_eq!(all.get(i as u32).unwrap(), *id);
    }

    let middle = client.get_tickets_by_owner(&event_id, &alice, &1, &2);
    assert_eq!(middle.len(), 2);
    assert_eq!(middle.get(0).unwrap(), bought[1]);
    assert_eq!(middle.get(1).unwrap(), bought[2]);

    assert_eq!(
        client
            .get_tickets_by_owner(&event_id, &alice, &5, &50)
            .len(),
        0
    );
    assert_eq!(
        client
            .get_tickets_by_owner(&event_id, &alice, &1000, &50)
            .len(),
        0
    );
    assert_eq!(
        client.get_tickets_by_owner(&event_id, &alice, &0, &0).len(),
        0
    );
}

#[test]
fn test_get_tickets_by_owner_limit_is_capped_at_max_page() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let alice = Address::generate(&env);
    let tiers = vec![
        &env,
        TierInput {
            name: String::from_str(&env, "General"),
            price: 1_000_000_i128,
            supply_cap: MAX_OWNER_TICKET_PAGE + 10,
        },
    ];
    let event_id = client.create_event(
        &organizer,
        &String::from_str(&env, "Big event"),
        &String::from_str(&env, "desc"),
        &String::from_str(&env, "venue"),
        &1_750_000_000_u64,
        &100_000_000_i128,
        &tiers,
    );
    token_admin.mint(&alice, &1_000_000_000_i128);
    for _ in 0..(MAX_OWNER_TICKET_PAGE + 10) {
        client.buy_ticket(&alice, &event_id, &0);
    }

    let page = client.get_tickets_by_owner(&event_id, &alice, &0, &(MAX_OWNER_TICKET_PAGE + 10));
    assert_eq!(page.len(), MAX_OWNER_TICKET_PAGE);
}

#[test]
fn test_tickets_by_owner_on_nonexistent_event_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let alice = Address::generate(&env);

    assert_eq!(
        client.try_get_tickets_by_owner(&99, &alice, &0, &50).err(),
        Some(Ok(Error::EventNotFound))
    );
    assert_eq!(
        client.try_owner_ticket_count(&99, &alice).err(),
        Some(Ok(Error::EventNotFound))
    );
}

#[test]
fn test_attach_receipt_records_hash_and_uri_and_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, contract_id, client) = setup(&env);
    let organizer = Address::generate(&env);
    let crew = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let payout_id = client.release_funds(
        &organizer,
        &event_id,
        &crew,
        &250_000_000_i128,
        &String::from_str(&env, "Sound crew"),
    );

    let hash = BytesN::from_array(&env, &[7u8; 32]);
    let uri = String::from_str(&env, "ipfs://bafy-receipt");
    client.attach_receipt(&organizer, &event_id, &payout_id, &hash, &Some(uri.clone()));
    let events = env.events().all().filter_by_contract(&contract_id);

    let payout = client.get_payout(&event_id, &payout_id);
    assert_eq!(payout.receipt_hash, Some(hash.clone()));
    assert_eq!(payout.receipt_uri, Some(uri));
    assert_eq!(
        events,
        [ReceiptAttached {
            event_id,
            payout_id,
            receipt_hash: hash,
        }
        .to_xdr(&env, &contract_id)]
    );
}

#[test]
fn test_attach_receipt_without_uri_leaves_it_none() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let crew = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let payout_id = client.release_funds(
        &organizer,
        &event_id,
        &crew,
        &250_000_000_i128,
        &String::from_str(&env, "Sound crew"),
    );

    client.attach_receipt(
        &organizer,
        &event_id,
        &payout_id,
        &BytesN::from_array(&env, &[1u8; 32]),
        &None,
    );

    assert_eq!(client.get_payout(&event_id, &payout_id).receipt_uri, None);
}

#[test]
fn test_attach_receipt_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let crew = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let payout_id = client.release_funds(
        &organizer,
        &event_id,
        &crew,
        &250_000_000_i128,
        &String::from_str(&env, "Sound crew"),
    );

    client.attach_receipt(
        &organizer,
        &event_id,
        &payout_id,
        &BytesN::from_array(&env, &[1u8; 32]),
        &None,
    );

    let result = client.try_attach_receipt(
        &organizer,
        &event_id,
        &payout_id,
        &BytesN::from_array(&env, &[2u8; 32]),
        &None,
    );
    assert_eq!(result.err(), Some(Ok(Error::ReceiptAlreadyAttached)));
    // The first hash must survive a rejected second attempt.
    assert_eq!(
        client.get_payout(&event_id, &payout_id).receipt_hash,
        Some(BytesN::from_array(&env, &[1u8; 32]))
    );
}

#[test]
fn test_attach_receipt_by_non_organizer_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let impostor = Address::generate(&env);
    let crew = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let payout_id = client.release_funds(
        &organizer,
        &event_id,
        &crew,
        &250_000_000_i128,
        &String::from_str(&env, "Sound crew"),
    );

    let result = client.try_attach_receipt(
        &impostor,
        &event_id,
        &payout_id,
        &BytesN::from_array(&env, &[1u8; 32]),
        &None,
    );
    assert_eq!(result.err(), Some(Ok(Error::NotOrganizer)));
}

#[test]
fn test_attach_receipt_on_missing_payout_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);

    let result = client.try_attach_receipt(
        &organizer,
        &event_id,
        &0,
        &BytesN::from_array(&env, &[1u8; 32]),
        &None,
    );
    assert_eq!(result.err(), Some(Ok(Error::PayoutNotFound)));
}

#[test]
fn test_attach_receipt_with_uri_too_long_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    let crew = Address::generate(&env);
    let event_id = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    let payout_id = client.release_funds(
        &organizer,
        &event_id,
        &crew,
        &250_000_000_i128,
        &String::from_str(&env, "Sound crew"),
    );

    let too_long = String::from_bytes(&env, &[b'x'; (MAX_RECEIPT_URI_LEN + 1) as usize]);
    let result = client.try_attach_receipt(
        &organizer,
        &event_id,
        &payout_id,
        &BytesN::from_array(&env, &[1u8; 32]),
        &Some(too_long),
    );
    assert_eq!(result.err(), Some(Ok(Error::InvalidReceiptUri)));
}

#[test]
fn test_organizer_stats_are_all_zero_for_an_unknown_organizer() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let stranger = Address::generate(&env);

    let stats = client.get_organizer_stats(&stranger);
    assert_eq!(stats.events_created, 0);
    assert_eq!(stats.events_ended, 0);
    assert_eq!(stats.events_cancelled, 0);
    assert_eq!(stats.total_raised, 0);
    assert_eq!(stats.total_released, 0);
    assert_eq!(stats.total_refunded, 0);

    assert_eq!(client.get_organizer_events(&stranger, &0, &50).len(), 0);
}

#[test]
fn test_organizer_stats_match_a_mixed_lifecycle_of_ended_and_cancelled_events() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, token_admin, _, client) = setup(&env);
    let organizer = Address::generate(&env);

    // Event A: ended, with a partial payout released from escrow.
    let event_a = ended_event_with_funds(&env, &client, &token_admin, &organizer);
    client.release_funds(
        &organizer,
        &event_a,
        &Address::generate(&env),
        &400_000_000_i128,
        &String::from_str(&env, "Venue hire"),
    );

    // Event B: cancelled, with both the sponsor and the ticket holder refunded.
    let sponsor_b = Address::generate(&env);
    let buyer_b = Address::generate(&env);
    token_admin.mint(&sponsor_b, &300_000_000_i128);
    token_admin.mint(&buyer_b, &10_000_000_i128);
    let event_b = create_test_event(&env, &client, &organizer);
    client.sponsor_event(&sponsor_b, &event_b, &300_000_000_i128);
    let ticket_b = client.buy_ticket(&buyer_b, &event_b, &0);
    client.cancel_event(&organizer, &event_b);
    client.refund_ticket(&buyer_b, &event_b, &ticket_b);
    client.refund_sponsorship(&sponsor_b, &event_b);

    let stats = client.get_organizer_stats(&organizer);
    assert_eq!(stats.events_created, 2);
    assert_eq!(stats.events_ended, 1);
    assert_eq!(stats.events_cancelled, 1);
    // Event A raised 610_000_000 (600m sponsor + 10m ticket); event B raised
    // 310_000_000 (300m sponsor + 10m ticket).
    assert_eq!(stats.total_raised, 610_000_000 + 310_000_000);
    assert_eq!(stats.total_released, 400_000_000);
    assert_eq!(stats.total_refunded, 300_000_000 + 10_000_000);

    let events = client.get_organizer_events(&organizer, &0, &50);
    assert_eq!(events, vec![&env, event_a, event_b]);
}

#[test]
fn test_get_organizer_events_pagination_bounds() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    for _ in 0..3 {
        create_test_event(&env, &client, &organizer);
    }

    assert_eq!(client.get_organizer_events(&organizer, &0, &2).len(), 2);
    assert_eq!(client.get_organizer_events(&organizer, &2, &2).len(), 1);
    assert_eq!(client.get_organizer_events(&organizer, &3, &2).len(), 0);
    assert_eq!(client.get_organizer_events(&organizer, &10, &2).len(), 0);
}

#[test]
fn test_get_organizer_events_limit_is_capped_at_max_page() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let organizer = Address::generate(&env);
    for _ in 0..(MAX_ORGANIZER_EVENTS_PAGE + 10) {
        create_test_event(&env, &client, &organizer);
    }

    let page = client.get_organizer_events(&organizer, &0, &(MAX_ORGANIZER_EVENTS_PAGE + 10));
    assert_eq!(page.len(), MAX_ORGANIZER_EVENTS_PAGE);
}

#[test]
fn test_organizer_stats_are_kept_separately_per_organizer() {
    let env = Env::default();
    env.mock_all_auths();

    let (_, _, _, client) = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    create_test_event(&env, &client, &alice);
    create_test_event(&env, &client, &bob);
    create_test_event(&env, &client, &bob);

    assert_eq!(client.get_organizer_stats(&alice).events_created, 1);
    assert_eq!(client.get_organizer_stats(&bob).events_created, 2);
}
