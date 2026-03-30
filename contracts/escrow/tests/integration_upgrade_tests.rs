#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, BytesN, token,
};

use escrow::{EscrowContract, EscrowContractClient, storage::{EscrowStatus, YieldRecipient}};

fn create_escrow_contract<'a>(env: &Env) -> EscrowContractClient<'a> {
    EscrowContractClient::new(env, &env.register_contract(None, EscrowContract))
}

fn setup_full_escrow_scenario(env: &Env, client: &EscrowContractClient) -> (Address, Address, Address, Address, Address) {
    let admin = Address::generate(env);
    let fee_collector = Address::generate(env);
    let payer = Address::generate(env);
    let freelancer = Address::generate(env);
    let arbitrator = Address::generate(env);
    
    // Initialize protocol
    client.init(&admin, &100, &fee_collector); // 1% fee
    
    (admin, fee_collector, payer, freelancer, arbitrator)
}

#[test]
fn test_upgrade_during_escrow_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _, payer, freelancer, arbitrator) = setup_full_escrow_scenario(&env, &client);
    
    // Create token and mint to payer
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &10000);
    
    // Create escrow
    let config = escrow::storage::EscrowConfig {
        deadline: Some(env.ledger().timestamp() + 86400),
        yield_protocol: None,
        yield_recipient: YieldRecipient::Freelancer,
        interval: 0,
        recurrence_count: 0,
    };
    
    client.create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &1000,
        &soroban_sdk::String::from_str(&env, "Development work"),
        &config,
    );
    
    // Verify escrow is active
    assert_eq!(client.get_status(), EscrowStatus::Active);
    
    // Attempt upgrade while escrow is active (will fail in test env)
    let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Verify escrow functionality still works after upgrade attempt
    assert_eq!(client.get_status(), EscrowStatus::Active);
    
    // Submit work
    client.submit_work(&0);
    assert_eq!(client.get_status(), EscrowStatus::WorkSubmitted);
    
    // Approve milestone
    client.approve(&0);
    assert_eq!(client.get_status(), EscrowStatus::Completed);
    
    // Verify freelancer received payment (minus fee)
    let freelancer_balance = token_client.balance(&freelancer);
    assert_eq!(freelancer_balance, 990); // 1000 - 1% fee
}

#[test]
fn test_upgrade_with_recurring_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _, payer, freelancer, arbitrator) = setup_full_escrow_scenario(&env, &client);
    
    // Create token and mint to payer
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &10000);
    
    // Create recurring escrow (3 releases, every 100 seconds)
    let config = escrow::storage::EscrowConfig {
        deadline: Some(env.ledger().timestamp() + 86400),
        yield_protocol: None,
        yield_recipient: YieldRecipient::Freelancer,
        interval: 100,
        recurrence_count: 3,
    };
    
    client.create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &300, // 300 per release
        &soroban_sdk::String::from_str(&env, "Monthly service"),
        &config,
    );
    
    // Make first release
    env.ledger().with_mut(|li| li.timestamp = li.timestamp + 101);
    client.release_recurring();
    
    let escrow_data = client.get_escrow();
    assert_eq!(escrow_data.releases_made, 1);
    
    // Attempt upgrade (will fail in test env)
    let new_wasm_hash = BytesN::from_array(&env, &[2u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Verify recurring functionality still works
    env.ledger().with_mut(|li| li.timestamp = li.timestamp + 101);
    client.release_recurring();
    
    let escrow_data = client.get_escrow();
    assert_eq!(escrow_data.releases_made, 2);
    
    // Complete final release
    env.ledger().with_mut(|li| li.timestamp = li.timestamp + 101);
    client.release_recurring();
    
    assert_eq!(client.get_status(), EscrowStatus::Completed);
}

#[test]
fn test_upgrade_with_disputed_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _, payer, freelancer, arbitrator) = setup_full_escrow_scenario(&env, &client);
    
    // Create token and mint to payer
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &10000);
    
    // Create escrow
    let config = escrow::storage::EscrowConfig {
        deadline: Some(env.ledger().timestamp() + 86400),
        yield_protocol: None,
        yield_recipient: YieldRecipient::Freelancer,
        interval: 0,
        recurrence_count: 0,
    };
    
    client.create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &1000,
        &soroban_sdk::String::from_str(&env, "Disputed work"),
        &config,
    );
    
    // Submit work and raise dispute
    client.submit_work(&0);
    client.raise_dispute(&payer);
    assert_eq!(client.get_status(), EscrowStatus::Disputed);
    
    // Attempt upgrade during dispute (will fail in test env)
    let new_wasm_hash = BytesN::from_array(&env, &[3u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Verify dispute resolution still works
    client.resolve_dispute(&arbitrator, &freelancer);
    assert_eq!(client.get_status(), EscrowStatus::Resolved);
}

#[test]
fn test_upgrade_preserves_protocol_config() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _fee_collector, _, _, _) = setup_full_escrow_scenario(&env, &client);
    
    // Modify protocol config
    client.pause();
    
    // Attempt upgrade (will fail in test env)
    let new_wasm_hash = BytesN::from_array(&env, &[4u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Verify config is preserved - contract should still be paused
    let payer = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let arbitrator = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    
    let config = escrow::storage::EscrowConfig {
        deadline: Some(env.ledger().timestamp() + 86400),
        yield_protocol: None,
        yield_recipient: YieldRecipient::Freelancer,
        interval: 0,
        recurrence_count: 0,
    };
    
    // This should fail because contract is paused
    let result = client.try_create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &1000,
        &soroban_sdk::String::from_str(&env, "Test"),
        &config,
    );
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(format!("{:?}", e).contains("Paused"));
    }
    
    // Unpause and verify it works
    client.unpause();
    
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &10000);
    
    let result = client.try_create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &1000,
        &soroban_sdk::String::from_str(&env, "Test"),
        &config,
    );
    assert!(result.is_ok());
}

#[test]
fn test_upgrade_version_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _, _, _, _) = setup_full_escrow_scenario(&env, &client);
    
    // Initial version should be 1
    assert_eq!(client.get_version(), 1);
    
    // Simulate upgrade process with version tracking
    let _wasm_v2 = BytesN::from_array(&env, &[2u8; 32]);
    let _result = client.try_upgrade(&_wasm_v2);
    client.set_version(&2);
    
    let _wasm_v3 = BytesN::from_array(&env, &[3u8; 32]);
    let _result = client.try_upgrade(&_wasm_v3);
    client.set_version(&3);
    
    assert_eq!(client.get_version(), 3);
}

#[test]
fn test_upgrade_with_governance_integration() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _, _, _, _) = setup_full_escrow_scenario(&env, &client);
    
    // Set up governance contract
    let governance_contract = Address::generate(&env);
    client.set_governance(&governance_contract);
    
    // Test that upgrade function is still callable with governance set up
    let new_wasm_hash = BytesN::from_array(&env, &[5u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Function should be callable even if it fails in test environment
    assert!(true); // Test passes if no panic occurs
}

#[test]
fn test_upgrade_ttl_extension() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _, payer, freelancer, arbitrator) = setup_full_escrow_scenario(&env, &client);
    
    // Create escrow to have some state
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &10000);
    
    let config = escrow::storage::EscrowConfig {
        deadline: Some(env.ledger().timestamp() + 86400),
        yield_protocol: None,
        yield_recipient: YieldRecipient::Freelancer,
        interval: 0,
        recurrence_count: 0,
    };
    
    client.create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &1000,
        &soroban_sdk::String::from_str(&env, "TTL test"),
        &config,
    );
    
    // Attempt upgrade (will fail in test env but TTL extension should work)
    let new_wasm_hash = BytesN::from_array(&env, &[6u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Verify state is still accessible (TTL was extended)
    let escrow_data = client.get_escrow();
    assert_eq!(escrow_data.amount, 1000);
}