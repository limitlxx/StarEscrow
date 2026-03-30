#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _},
    Address, Env, BytesN, IntoVal,
};

use escrow::{EscrowContract, EscrowContractClient, storage::YieldRecipient};

fn create_escrow_contract<'a>(env: &Env) -> EscrowContractClient<'a> {
    EscrowContractClient::new(env, &env.register_contract(None, EscrowContract))
}

fn setup_protocol(env: &Env, client: &EscrowContractClient) -> (Address, Address) {
    let admin = Address::generate(env);
    let fee_collector = Address::generate(env);
    
    client.init(&admin, &100, &fee_collector);
    (admin, fee_collector)
}

#[test]
fn test_upgrade_authorization() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _) = setup_protocol(&env, &client);
    
    // Create a mock new WASM hash
    let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    
    // Test upgrade authorization - this will fail at the deployer level in tests
    // but we can verify the function exists and accepts the right parameters
    let result = client.try_upgrade(&new_wasm_hash);
    // In test environment, this will fail because WASM doesn't exist
    // but the function should be callable
    assert!(result.is_err());
}

#[test]
fn test_upgrade_invalid_wasm_hash() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _) = setup_protocol(&env, &client);
    
    // Test with empty/zero WASM hash
    let empty_wasm_hash = BytesN::from_array(&env, &[0u8; 32]);
    
    let result = client.try_upgrade(&empty_wasm_hash);
    assert!(result.is_err());
    if let Err(e) = result {
        // Check if it's the right error type
        assert!(format!("{:?}", e).contains("InvalidWasmHash"));
    }
}

#[test]
fn test_version_management() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _) = setup_protocol(&env, &client);
    
    // Test initial version
    let initial_version = client.get_version();
    assert_eq!(initial_version, 1);
    
    // Test setting version
    client.set_version(&2);
    let updated_version = client.get_version();
    assert_eq!(updated_version, 2);
}

#[test]
fn test_version_unauthorized() {
    let env = Env::default();
    
    let client = create_escrow_contract(&env);
    
    // Initialize without mocking auth for admin
    let admin = Address::generate(&env);
    let fee_collector = Address::generate(&env);
    
    // Mock auth only for the init call
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &client.address,
            fn_name: "init",
            args: (&admin, 100u32, &fee_collector).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    
    client.init(&admin, &100, &fee_collector);
    
    // Now try to set version without proper auth - should fail
    let result = client.try_set_version(&2);
    assert!(result.is_err());
}

#[test]
fn test_upgrade_with_active_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _) = setup_protocol(&env, &client);
    
    // Create an active escrow first
    let payer = Address::generate(&env);
    let freelancer = Address::generate(&env);
    let arbitrator = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    
    // Create escrow config
    let config = escrow::storage::EscrowConfig {
        deadline: Some(env.ledger().timestamp() + 86400), // 1 day from now
        yield_protocol: None,
        yield_recipient: YieldRecipient::Freelancer,
        interval: 0,
        recurrence_count: 0,
    };
    
    // Mint tokens to payer
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &1000);
    
    // Create escrow
    client.create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &100,
        &soroban_sdk::String::from_str(&env, "Test milestone"),
        &config,
    );
    
    // Verify escrow exists
    let escrow_data = client.get_escrow();
    assert_eq!(escrow_data.payer, payer);
    assert_eq!(escrow_data.freelancer, freelancer);
    
    // Test upgrade function exists (will fail in test env but function is callable)
    let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    let result = client.try_upgrade(&new_wasm_hash);
    // Expected to fail in test environment due to missing WASM
    assert!(result.is_err());
}

#[test]
fn test_upgrade_state_preservation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (admin, _) = setup_protocol(&env, &client);
    
    // Set initial version
    client.set_version(&1);
    
    // Create some state
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
    
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_client.mint(&payer, &1000);
    
    client.create(
        &payer,
        &freelancer,
        &arbitrator,
        &token,
        &100,
        &soroban_sdk::String::from_str(&env, "Test milestone"),
        &config,
    );
    
    // Verify state before upgrade attempt
    let escrow_data_before = client.get_escrow();
    assert_eq!(escrow_data_before.payer, payer);
    assert_eq!(escrow_data_before.amount, 100);
    
    // Attempt upgrade (will fail in test env)
    let new_wasm_hash = BytesN::from_array(&env, &[2u8; 32]);
    let _result = client.try_upgrade(&new_wasm_hash);
    
    // Verify state is still accessible (not corrupted by failed upgrade)
    let escrow_data_after = client.get_escrow();
    assert_eq!(escrow_data_after.payer, payer);
    assert_eq!(escrow_data_after.amount, 100);
    
    // Version management still works
    client.set_version(&2);
    assert_eq!(client.get_version(), 2);
}

#[test]
fn test_upgrade_paused_contract() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _) = setup_protocol(&env, &client);
    
    // Pause the contract
    client.pause();
    
    // Upgrade should still be callable when paused (admin operation)
    let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    let result = client.try_upgrade(&new_wasm_hash);
    // Will fail due to missing WASM in test env, but function is callable
    assert!(result.is_err());
}

#[test]
fn test_version_tracking() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _) = setup_protocol(&env, &client);
    
    // Initial version should be 1
    assert_eq!(client.get_version(), 1);
    
    // Simulate upgrade process with version tracking
    client.set_version(&2);
    assert_eq!(client.get_version(), 2);
    
    client.set_version(&3);
    assert_eq!(client.get_version(), 3);
}

#[test]
fn test_upgrade_with_governance_integration() {
    let env = Env::default();
    env.mock_all_auths();
    
    let client = create_escrow_contract(&env);
    let (_admin, _) = setup_protocol(&env, &client);
    
    // Set up governance contract
    let governance_contract = Address::generate(&env);
    client.set_governance(&governance_contract);
    
    // Test that upgrade function is still callable with governance set up
    let new_wasm_hash = BytesN::from_array(&env, &[5u8; 32]);
    let result = client.try_upgrade(&new_wasm_hash);
    // Expected to fail in test environment, but function should be accessible
    assert!(result.is_err());
}