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
