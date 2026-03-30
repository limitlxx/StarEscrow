use crate::storage;
use soroban_sdk::{ Address, Env, String, Symbol, Vec, BytesN };

pub fn escrow_created(
    env: &Env,
    payer: &Address,
    freelancer: &Address,
    amount: i128,
    milestones: &Vec<storage::Milestone>
) {
    env.events().publish(
        (Symbol::new(env, "escrow_created"),),
        (payer.clone(), freelancer.clone(), amount, milestones.clone())
    );
}

pub fn milestone_submitted(
    env: &Env,
    freelancer: &Address,
    milestone_idx: u32,
    description: &String
) {
    env.events().publish(
        (Symbol::new(env, "milestone_submitted"),),
        (freelancer.clone(), milestone_idx, description.clone())
    );
}

pub fn milestone_approved(
    env: &Env,
    freelancer: &Address,
    milestone_idx: u32,
    description: &String,
    amount: i128
) {
    env.events().publish(
        (Symbol::new(env, "milestone_approved"),),
        (freelancer.clone(), milestone_idx, description.clone(), amount)
    );
}

pub fn work_submitted(env: &Env, freelancer: &Address) {
    env.events().publish((Symbol::new(env, "work_submitted"),), (freelancer.clone(),));
pub fn work_submitted(env: Env, freelancer: Address) {
    env.events().publish((Symbol::new(&env, "work_submitted"),), (freelancer,));
}

pub fn payment_released(env: Env, freelancer: Address, amount: i128) {
    env.events().publish((Symbol::new(&env, "payment_released"),), (freelancer, amount));
}

pub fn recurring_released(env: &Env, freelancer: &Address, amount: i128, release_num: u32) {
    env.events().publish(
        (Symbol::new(env, "recurring_released"),),
        (freelancer.clone(), amount, release_num)
    );
}

pub fn escrow_cancelled(env: &Env, payer: &Address, amount: i128) {
    env.events().publish((Symbol::new(env, "escrow_cancelled"),), (payer.clone(), amount));
}

pub fn escrow_expired(env: Env, payer: Address, amount: i128) {
    env.events().publish((Symbol::new(&env, "escrow_expired"),), (payer, amount));
}

pub fn freelancer_transferred(env: Env, old: Address, new_addr: Address) {
    env.events().publish((Symbol::new(&env, "freelancer_transferred"),), (old, new_addr));
}

pub fn payer_transferred(env: &Env, old: &Address, new: &Address) {
    env.events().publish((Symbol::new(env, "payer_transferred"),), (old.clone(), new.clone()));
}

pub fn dispute_raised(env: &Env, caller: &Address) {
    env.events().publish((Symbol::new(env, "dispute_raised"),), (caller.clone(),));
}

pub fn dispute_resolved(env: &Env, released_to: &Address) {
    env.events().publish((Symbol::new(env, "dispute_resolved"),), (released_to.clone(),));

pub fn milestone_updated(env: Env, old: String, new_desc: String) {
    env.events().publish((Symbol::new(&env, "milestone_updated"),), (old, new_desc));
}

pub fn deadline_extended(env: Env, old_deadline: u64, new_deadline: u64) {
    env.events().publish((Symbol::new(&env, "deadline_extended"),), (old_deadline, new_deadline));
}

pub fn contract_paused(env: Env, admin: Address) {
    env.events().publish((Symbol::new(&env, "contract_paused"),), (admin,));
}

pub fn contract_unpaused(env: Env, admin: Address) {
    env.events().publish((Symbol::new(&env, "contract_unpaused"),), (admin,));
}

pub fn yield_deposited(env: Env, protocol: Address, amount: i128) {
    env.events().publish((Symbol::new(&env, "yield_deposited"),), (protocol, amount));
}

pub fn contract_upgraded(env: &Env, new_wasm_hash: BytesN<32>) {
    env.events().publish((Symbol::new(env, "contract_upgraded"),), (new_wasm_hash,));
}

pub fn yield_deposited(env: &Env, protocol: &Address, amount: i128) {
    env.events().publish((Symbol::new(env, "yield_deposited"),), (protocol.clone(), amount));
pub fn payer_transferred(env: Env, old: Address, new_addr: Address) {
    env.events().publish((Symbol::new(&env, "payer_transferred"),), (old, new_addr));
}

pub fn milestone_approved(env: Env, freelancer: Address, milestone_idx: u32, description: String, amount: i128) {
    env.events().publish(
        (Symbol::new(&env, "milestone_approved"),),
        (freelancer, milestone_idx, description, amount)
    );
}

pub fn recurring_released(env: Env, freelancer: Address, amount: i128, release_num: u32) {
    env.events().publish(
        (Symbol::new(&env, "recurring_released"),),
        (freelancer, amount, release_num)
    );
}

pub fn dispute_resolved(env: Env, released_to: Address) {
    env.events().publish((Symbol::new(&env, "dispute_resolved"),), (released_to,));
}
