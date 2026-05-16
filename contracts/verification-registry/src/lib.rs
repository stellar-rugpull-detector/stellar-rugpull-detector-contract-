#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Asset(String),       // asset_code -> AssetRecord
    Issuer(Address),     // issuer    -> IssuerRecord
    ReputationScore(Address), // address -> i32 score
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct AssetRecord {
    pub asset_code: String,
    pub issuer: Address,
    pub verified: bool,
    pub risk_score: u32,   // 0-100, higher = riskier
    pub flags: u32,        // bitmask: 1=freeze_enabled, 2=clawback_enabled, 4=auth_required
}

#[contracttype]
#[derive(Clone)]
pub struct IssuerRecord {
    pub issuer: Address,
    pub approved: bool,
    pub reputation: i32,   // can go negative for bad actors
    pub reported_count: u32,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct VerificationRegistry;

#[contractimpl]
impl VerificationRegistry {
    /// Initialize with an admin address.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    // ── Asset management ──────────────────────────────────────────────────────

    /// Register or update an asset record. Admin only.
    pub fn register_asset(
        env: Env,
        asset_code: String,
        issuer: Address,
        risk_score: u32,
        flags: u32,
    ) {
        Self::require_admin(&env);
        let record = AssetRecord {
            asset_code: asset_code.clone(),
            issuer,
            verified: false,
            risk_score,
            flags,
        };
        env.storage().persistent().set(&DataKey::Asset(asset_code.clone()), &record);
        env.events().publish(
            (Symbol::new(&env, "asset_registered"),),
            asset_code,
        );
    }

    /// Mark an asset as verified. Admin only.
    pub fn verify_asset(env: Env, asset_code: String) {
        Self::require_admin(&env);
        let key = DataKey::Asset(asset_code.clone());
        let mut record: AssetRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("asset not found");
        record.verified = true;
        env.storage().persistent().set(&key, &record);
        env.events().publish(
            (Symbol::new(&env, "asset_verified"),),
            asset_code,
        );
    }

    /// Update risk score for an asset. Admin only.
    pub fn update_risk_score(env: Env, asset_code: String, risk_score: u32) {
        Self::require_admin(&env);
        let key = DataKey::Asset(asset_code.clone());
        let mut record: AssetRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("asset not found");
        record.risk_score = risk_score;
        env.storage().persistent().set(&key, &record);
    }

    /// Get asset record.
    pub fn get_asset(env: Env, asset_code: String) -> Option<AssetRecord> {
        env.storage().persistent().get(&DataKey::Asset(asset_code))
    }

    // ── Issuer management ─────────────────────────────────────────────────────

    /// Register or update an issuer. Admin only.
    pub fn register_issuer(env: Env, issuer: Address) {
        Self::require_admin(&env);
        let record = IssuerRecord {
            issuer: issuer.clone(),
            approved: false,
            reputation: 0,
            reported_count: 0,
        };
        env.storage().persistent().set(&DataKey::Issuer(issuer.clone()), &record);
        env.events().publish(
            (Symbol::new(&env, "issuer_registered"),),
            issuer,
        );
    }

    /// Approve an issuer. Admin only.
    pub fn approve_issuer(env: Env, issuer: Address) {
        Self::require_admin(&env);
        let key = DataKey::Issuer(issuer.clone());
        let mut record: IssuerRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("issuer not found");
        record.approved = true;
        env.storage().persistent().set(&key, &record);
    }

    /// Adjust issuer reputation (positive or negative delta). Admin only.
    pub fn adjust_reputation(env: Env, issuer: Address, delta: i32) {
        Self::require_admin(&env);
        let key = DataKey::Issuer(issuer.clone());
        let mut record: IssuerRecord = env
            .storage()
            .persistent()
            .get(&key)
            .expect("issuer not found");
        record.reputation = record.reputation.saturating_add(delta);
        env.storage().persistent().set(&key, &record);
    }

    /// Get issuer record.
    pub fn get_issuer(env: Env, issuer: Address) -> Option<IssuerRecord> {
        env.storage().persistent().get(&DataKey::Issuer(issuer))
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).expect("not initialized")
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
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, VerificationRegistryClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(VerificationRegistry, ());
        let client = VerificationRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    #[test]
    fn test_initialize() {
        let (_, client, admin) = setup();
        assert_eq!(client.get_admin(), admin);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize() {
        let (env, client, _) = setup();
        let other = Address::generate(&env);
        client.initialize(&other);
    }

    #[test]
    fn test_register_and_verify_asset() {
        let (env, client, _) = setup();
        let issuer = Address::generate(&env);
        let code = String::from_str(&env, "SCAM");

        client.register_asset(&code, &issuer, &75, &3);
        let record = client.get_asset(&code).unwrap();
        assert_eq!(record.risk_score, 75);
        assert!(!record.verified);

        client.verify_asset(&code);
        let record = client.get_asset(&code).unwrap();
        assert!(record.verified);
    }

    #[test]
    fn test_update_risk_score() {
        let (env, client, _) = setup();
        let issuer = Address::generate(&env);
        let code = String::from_str(&env, "TOKEN");
        client.register_asset(&code, &issuer, &50, &0);
        client.update_risk_score(&code, &90);
        assert_eq!(client.get_asset(&code).unwrap().risk_score, 90);
    }

    #[test]
    fn test_register_and_approve_issuer() {
        let (env, client, _) = setup();
        let issuer = Address::generate(&env);
        client.register_issuer(&issuer);
        let rec = client.get_issuer(&issuer).unwrap();
        assert!(!rec.approved);
        assert_eq!(rec.reputation, 0);

        client.approve_issuer(&issuer);
        assert!(client.get_issuer(&issuer).unwrap().approved);
    }

    #[test]
    fn test_adjust_reputation() {
        let (env, client, _) = setup();
        let issuer = Address::generate(&env);
        client.register_issuer(&issuer);
        client.adjust_reputation(&issuer, &10);
        client.adjust_reputation(&issuer, &-30);
        assert_eq!(client.get_issuer(&issuer).unwrap().reputation, -20);
    }
}
