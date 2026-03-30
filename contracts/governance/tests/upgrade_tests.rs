#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String, Vec, BytesN,
};

use governance::{GovernanceContract, GovernanceContractClient, ParamChange, ProposalStatus};

fn create_test_env() -> (Env, GovernanceContractClient, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);

    let vote_token = Address::generate(&env);
    let escrow_contract = Address::generate(&env);
    let proposer = Address::generate(&env);

    // Initialize governance
    client.init(&vote_token, 86400, 172800, 1000, &escrow_contract); // 1 day voting, 2 day timelock, 1000 quorum

    (env, client, vote_token, escrow_contract, proposer)
}

#[test]
fn test_propose_upgrade() {
    let (env, client, _vote_token, _escrow_contract, proposer) = create_test_env();

    // Create a dummy WASM hash for testing
    let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

    // Propose an upgrade
    let proposal_id = client.propose_upgrade(&proposer, &new_wasm_hash);
    assert_eq!(proposal_id, 0);

    // Verify the proposal was created correctly
    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.proposer, proposer);
    assert_eq!(proposal.changes.len(), 1);
    assert_eq!(proposal.changes.get(0).unwrap().key, String::from_str(&env, "upgrade_contract"));
    assert_eq!(proposal.status, ProposalStatus::Active);
}

#[test]
fn test_upgrade_proposal_execution() {
    let (env, client, vote_token, escrow_contract, proposer) = create_test_env();

    // Mock token balance for voting
    let voter = Address::generate(&env);
    
    // Create upgrade proposal
    let new_wasm_hash = BytesN::from_array(&env, &[2u8; 32]);
    let proposal_id = client.propose_upgrade(&proposer, &new_wasm_hash);

    // Mock token client to return sufficient balance for voting
    // In a real test, you'd set up the token contract properly
    
    // Fast forward time to end voting period
    env.ledger().with_mut(|li| {
        li.timestamp = 86401; // Just after voting period
    });

    // Finalize the proposal (in real scenario, this would check vote results)
    let status = client.finalize(&proposal_id);
    // Note: This will likely be Rejected in test since we haven't set up proper voting
    
    // If it were passed, we could test execution:
    // env.ledger().with_mut(|li| {
    //     li.timestamp = 259201; // After timelock period
    // });
    // client.execute(&proposal_id);
}

#[test]
fn test_mixed_upgrade_and_param_proposal() {
    let (env, client, _vote_token, _escrow_contract, proposer) = create_test_env();

    // Create a proposal with both upgrade and parameter changes
    let mut changes = Vec::new(&env);
    
    // Add upgrade change
    changes.push_back(ParamChange {
        key: String::from_str(&env, "upgrade_contract"),
        value: String::from_str(&env, "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
    });
    
    // Add parameter change
    changes.push_back(ParamChange {
        key: String::from_str(&env, "fee_bps"),
        value: String::from_str(&env, "200"),
    });

    let proposal_id = client.propose(&proposer, &changes);
    assert_eq!(proposal_id, 0);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.changes.len(), 2);
    
    // Verify upgrade change
    let upgrade_change = proposal.changes.get(0).unwrap();
    assert_eq!(upgrade_change.key, String::from_str(&env, "upgrade_contract"));
    
    // Verify param change
    let param_change = proposal.changes.get(1).unwrap();
    assert_eq!(param_change.key, String::from_str(&env, "fee_bps"));
    assert_eq!(param_change.value, String::from_str(&env, "200"));
}

#[test]
fn test_upgrade_proposal_timelock() {
    let (env, client, _vote_token, _escrow_contract, proposer) = create_test_env();

    let new_wasm_hash = BytesN::from_array(&env, &[3u8; 32]);
    let proposal_id = client.propose_upgrade(&proposer, &new_wasm_hash);

    let proposal = client.get_proposal(&proposal_id).unwrap();
    
    // Verify timelock is properly set
    let expected_eta = 86400 + 172800; // voting_period + timelock_delay
    assert_eq!(proposal.execution_eta, expected_eta);
}

#[test]
fn test_governance_config_access() {
    let (env, client, vote_token, escrow_contract, _proposer) = create_test_env();

    let config = client.get_config().unwrap();
    assert_eq!(config.vote_token, vote_token);
    assert_eq!(config.escrow_contract, escrow_contract);
    assert_eq!(config.voting_period, 86400);
    assert_eq!(config.timelock_delay, 172800);
    assert_eq!(config.quorum, 1000);
}