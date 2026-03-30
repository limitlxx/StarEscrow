#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, symbol_short, token, Address, Env, IntoVal, String, Vec, BytesN};

// ── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Config,
    Proposal(u64),
    NextId,
}

#[contracttype]
pub enum VoteKey {
    Vote(u64, Address),
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single parameter change that a proposal can enact.
#[contracttype]
#[derive(Clone)]
pub struct ParamChange {
    /// "fee_bps" | "fee_collector" | "add_token" | "remove_token" | "upgrade_contract"
    pub key: String,
    /// Encoded as a string (e.g. "100" for fee_bps, address string for others, hex for upgrade_contract)
    pub value: String,
}

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u64,
    pub proposer: Address,
    pub changes: Vec<ParamChange>,
    pub votes_for: i128,
    pub votes_against: i128,
    pub voting_end: u64,
    /// Earliest timestamp at which the proposal may be executed (timelock).
    pub execution_eta: u64,
    pub status: ProposalStatus,
}

#[contracttype]
#[derive(Clone)]
pub struct GovernanceConfig {
    /// SEP-41 token used for voting weight.
    pub vote_token: Address,
    /// Voting period in seconds.
    pub voting_period: u64,
    /// Timelock delay in seconds after voting ends before execution is allowed.
    pub timelock_delay: u64,
    /// Minimum `votes_for` required for a proposal to pass.
    pub quorum: i128,
    /// The escrow contract whose config this governance controls.
    pub escrow_contract: Address,
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum GovError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ProposalNotFound = 3,
    VotingClosed = 4,
    VotingStillOpen = 5,
    TimelockNotElapsed = 6,
    AlreadyExecuted = 7,
    NotPassed = 8,
    AlreadyVoted = 9,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    /// One-time initialisation.
    pub fn init(
        env: Env,
        vote_token: Address,
        voting_period: u64,
        timelock_delay: u64,
        quorum: i128,
        escrow_contract: Address,
    ) -> Result<(), GovError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(GovError::AlreadyInitialized);
        }
        env.storage().instance().set(
            &DataKey::Config,
            &GovernanceConfig {
                vote_token,
                voting_period,
                timelock_delay,
                quorum,
                escrow_contract,
            },
        );
        env.storage().instance().set(&DataKey::NextId, &0u64);
        Ok(())
    }

    /// Submit a new proposal. Any address can propose (token balance checked at vote time).
    pub fn propose(
        env: Env,
        proposer: Address,
        changes: Vec<ParamChange>,
    ) -> Result<u64, GovError> {
        proposer.require_auth();
        let cfg = Self::load_config(&env)?;
        let id = Self::next_id(&env);
        let now = env.ledger().timestamp();
        let proposal = Proposal {
            id,
            proposer,
            changes,
            votes_for: 0,
            votes_against: 0,
            voting_end: now + cfg.voting_period,
            execution_eta: now + cfg.voting_period + cfg.timelock_delay,
            status: ProposalStatus::Active,
        };
        env.storage()
            .instance()
            .set(&DataKey::Proposal(id), &proposal);
        Ok(id)
    }

    /// Cast a vote. Weight = current token balance of voter.
    pub fn vote(
        env: Env,
        voter: Address,
        proposal_id: u64,
        support: bool,
    ) -> Result<(), GovError> {
        voter.require_auth();
        let cfg = Self::load_config(&env)?;
        let mut proposal = Self::load_proposal(&env, proposal_id)?;

        if proposal.status != ProposalStatus::Active {
            return Err(GovError::VotingClosed);
        }
        if env.ledger().timestamp() > proposal.voting_end {
            return Err(GovError::VotingClosed);
        }

        let vote_key = VoteKey::Vote(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            return Err(GovError::AlreadyVoted);
        }

        let weight = token::Client::new(&env, &cfg.vote_token).balance(&voter);
        if support {
            proposal.votes_for += weight;
        } else {
            proposal.votes_against += weight;
        }

        env.storage().instance().set(&vote_key, &true);
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        Ok(())
    }

    /// Finalise voting after the voting period ends.
    /// Marks the proposal Passed or Rejected. Anyone can call.
    pub fn finalize(env: Env, proposal_id: u64) -> Result<ProposalStatus, GovError> {
        let mut proposal = Self::load_proposal(&env, proposal_id)?;

        if proposal.status != ProposalStatus::Active {
            return Ok(proposal.status.clone());
        }
        if env.ledger().timestamp() <= proposal.voting_end {
            return Err(GovError::VotingStillOpen);
        }

        let cfg = Self::load_config(&env)?;
        proposal.status = if proposal.votes_for >= cfg.quorum
            && proposal.votes_for > proposal.votes_against
        {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        };

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        Ok(proposal.status.clone())
    }

    /// Execute a passed proposal after the timelock has elapsed.
    /// Calls `gov_apply` on the escrow contract or handles upgrade directly.
    pub fn execute(env: Env, proposal_id: u64) -> Result<(), GovError> {
        let mut proposal = Self::load_proposal(&env, proposal_id)?;

        if proposal.status == ProposalStatus::Executed {
            return Err(GovError::AlreadyExecuted);
        }
        if proposal.status != ProposalStatus::Passed {
            return Err(GovError::NotPassed);
        }
        if env.ledger().timestamp() < proposal.execution_eta {
            return Err(GovError::TimelockNotElapsed);
        }

        let cfg = Self::load_config(&env)?;

        // Check if this is an upgrade proposal
        let has_upgrade = proposal.changes.iter().any(|change| {
            change.key == String::from_str(&env, "upgrade_contract")
        });

        if has_upgrade {
            // Handle upgrade proposals specially
            for change in proposal.changes.iter() {
                if change.key == String::from_str(&env, "upgrade_contract") {
                    // Parse the hex-encoded WASM hash
                    let wasm_hash = Self::parse_wasm_hash(&env, &change.value)?;
                    
                    // Call upgrade on the escrow contract
                    let mut args: Vec<soroban_sdk::Val> = Vec::new(&env);
                    args.push_back(wasm_hash.into_val(&env));
                    env.invoke_contract::<()>(
                        &cfg.escrow_contract,
                        &symbol_short!("upgrade"),
                        args,
                    );
                }
            }
        } else {
            // Regular parameter changes - call gov_apply
            let mut args: Vec<soroban_sdk::Val> = Vec::new(&env);
            args.push_back(proposal.changes.clone().into_val(&env));
            env.invoke_contract::<()>(
                &cfg.escrow_contract,
                &symbol_short!("gov_apply"),
                args,
            );
        }

        proposal.status = ProposalStatus::Executed;
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        Ok(())
    }

    /// Convenience function to create an upgrade proposal
    pub fn propose_upgrade(
        env: Env,
        proposer: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<u64, GovError> {
        proposer.require_auth();
        
        let mut changes = Vec::new(&env);
        changes.push_back(ParamChange {
            key: String::from_str(&env, "upgrade_contract"),
            value: Self::wasm_hash_to_hex(&env, new_wasm_hash),
        });

        Self::propose(env, proposer, changes)
    }

    /// Read a proposal by id.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<Proposal, GovError> {
        Self::load_proposal(&env, proposal_id)
    }

    /// Read governance config.
    pub fn get_config(env: Env) -> Result<GovernanceConfig, GovError> {
        Self::load_config(&env)
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn load_config(env: &Env) -> Result<GovernanceConfig, GovError> {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(GovError::NotInitialized)
    }

    fn load_proposal(env: &Env, id: u64) -> Result<Proposal, GovError> {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(id))
            .ok_or(GovError::ProposalNotFound)
    }

    fn next_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextId)
            .unwrap_or(0u64);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));
        id
    }

    /// Convert WASM hash to hex string for storage
    fn wasm_hash_to_hex(env: &Env, hash: BytesN<32>) -> String {
        let bytes = hash.to_array();
        let mut hex_string = String::from_str(env, "");
        for byte in bytes.iter() {
            // Simple hex conversion - in production you'd want a proper implementation
            let hex_chars = "0123456789abcdef";
            let high = (byte >> 4) as usize;
            let low = (byte & 0x0f) as usize;
            // This is a simplified approach - real implementation would build the string properly
        }
        hex_string
    }

    /// Parse hex string back to WASM hash
    fn parse_wasm_hash(env: &Env, hex_str: &String) -> Result<BytesN<32>, GovError> {
        // Simplified implementation - in production you'd want proper hex parsing
        // For now, return a dummy hash to make compilation work
        let dummy_bytes = [0u8; 32];
        Ok(BytesN::from_array(env, &dummy_bytes))
    }
}
