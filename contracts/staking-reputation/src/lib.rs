#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Symbol};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    StakeToken,
    Stake(Address),      // staker -> StakeRecord
    Reputation(Address), // address -> i64
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct StakeRecord {
    pub staker: Address,
    pub amount: i128,
    pub staked_at: u64,
    pub locked_until: u64, // ledger timestamp
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct StakingReputation;

#[contractimpl]
impl StakingReputation {
    pub fn initialize(env: Env, admin: Address, stake_token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::StakeToken, &stake_token);
    }

    // ── Staking ───────────────────────────────────────────────────────────────

    /// Stake tokens for a lock period (in seconds).
    pub fn stake(env: Env, staker: Address, amount: i128, lock_duration: u64) {
        staker.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::StakeToken)
            .expect("not initialized");
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&staker, &env.current_contract_address(), &amount);

        let locked_until = env.ledger().timestamp() + lock_duration;
        let record = StakeRecord {
            staker: staker.clone(),
            amount,
            staked_at: env.ledger().timestamp(),
            locked_until,
        };
        env.storage().persistent().set(&DataKey::Stake(staker.clone()), &record);

        // Reputation increases proportionally to stake
        let rep_delta = (amount / 1_000_000) as i64; // 1 rep per 1 token (7 decimals)
        Self::add_reputation(&env, &staker, rep_delta);

        env.events().publish(
            (Symbol::new(&env, "staked"),),
            (staker, amount, locked_until),
        );
    }

    /// Unstake after lock period expires.
    pub fn unstake(env: Env, staker: Address) {
        staker.require_auth();
        let key = DataKey::Stake(staker.clone());
        let record: StakeRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("no stake found");

        if env.ledger().timestamp() < record.locked_until {
            panic!("stake still locked");
        }

        let token_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::StakeToken)
            .expect("not initialized");
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &staker, &record.amount);

        env.storage().persistent().remove(&key);
        env.events().publish(
            (Symbol::new(&env, "unstaked"),),
            (staker, record.amount),
        );
    }

    // ── Reputation ────────────────────────────────────────────────────────────

    /// Slash a staker's reputation and optionally confiscate stake. Admin only.
    pub fn slash(env: Env, staker: Address, rep_penalty: i64) {
        Self::require_admin(&env);
        Self::add_reputation(&env, &staker, -rep_penalty);
        env.events().publish(
            (Symbol::new(&env, "slashed"),),
            (staker, rep_penalty),
        );
    }

    /// Reward a staker's reputation. Admin only.
    pub fn reward(env: Env, staker: Address, rep_bonus: i64) {
        Self::require_admin(&env);
        Self::add_reputation(&env, &staker, rep_bonus);
    }

    pub fn get_stake(env: Env, staker: Address) -> Option<StakeRecord> {
        env.storage().persistent().get(&DataKey::Stake(staker))
    }

    pub fn get_reputation(env: Env, addr: Address) -> i64 {
        env.storage()
            .persistent()
            .get(&DataKey::Reputation(addr))
            .unwrap_or(0)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).expect("not initialized")
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn add_reputation(env: &Env, addr: &Address, delta: i64) {
        let key = DataKey::Reputation(addr.clone());
        let current: i64 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&key, &current.saturating_add(delta));
    }

    fn require_admin(env: &Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin.require_auth();
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::{StellarAssetClient},
        Env,
    };

    fn setup() -> (Env, StakingReputationClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(Address::generate(&env));
        let token_addr = token_id.address();

        let contract_id = env.register(StakingReputation, ());
        let client = StakingReputationClient::new(&env, &contract_id);
        client.initialize(&admin, &token_addr);

        (env, client, admin, token_addr)
    }

    fn mint(env: &Env, token_addr: &Address, to: &Address, amount: i128) {
        let sac = StellarAssetClient::new(env, token_addr);
        sac.mint(to, &amount);
    }

    #[test]
    fn test_stake_and_reputation() {
        let (env, client, _, token_addr) = setup();
        let staker = Address::generate(&env);
        mint(&env, &token_addr, &staker, 10_000_000);

        client.stake(&staker, &10_000_000i128, &100u64);

        let record = client.get_stake(&staker).unwrap();
        assert_eq!(record.amount, 10_000_000);

        // 10_000_000 / 1_000_000 = 10 rep
        assert_eq!(client.get_reputation(&staker), 10);
    }

    #[test]
    fn test_unstake_after_lock() {
        let (env, client, _, token_addr) = setup();
        let staker = Address::generate(&env);
        mint(&env, &token_addr, &staker, 5_000_000);

        client.stake(&staker, &5_000_000i128, &100u64);

        // Advance ledger past lock
        env.ledger().with_mut(|l| l.timestamp = 200);
        client.unstake(&staker);
        assert!(client.get_stake(&staker).is_none());
    }

    #[test]
    #[should_panic(expected = "stake still locked")]
    fn test_unstake_before_lock_fails() {
        let (env, client, _, token_addr) = setup();
        let staker = Address::generate(&env);
        mint(&env, &token_addr, &staker, 5_000_000);
        client.stake(&staker, &5_000_000i128, &1000u64);
        client.unstake(&staker);
    }

    #[test]
    fn test_slash_and_reward() {
        let (env, client, _, token_addr) = setup();
        let staker = Address::generate(&env);
        mint(&env, &token_addr, &staker, 10_000_000);
        client.stake(&staker, &10_000_000i128, &100u64);

        client.slash(&staker, &5i64);
        assert_eq!(client.get_reputation(&staker), 5);

        client.reward(&staker, &3i64);
        assert_eq!(client.get_reputation(&staker), 8);
    }
}
