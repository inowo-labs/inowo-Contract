use super::*;
use soroban_sdk::{
    testutils::{
        storage::{Instance as _, Persistent as _},
        Address as _, Events as _,
    },
    token::{Client as TokenClient, StellarAssetClient},
    vec, Address, Env, Event as _, String,
};

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

    let sponsorships = client.get_sponsorships(&event_id);
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
        client.try_get_sponsorships(&missing).err(),
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
            DataKey::Sponsorships(event_id),
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
