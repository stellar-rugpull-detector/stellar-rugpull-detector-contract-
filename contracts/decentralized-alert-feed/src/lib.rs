#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Alert(u64),       // alert_id -> Alert
    AlertCount,
    Publisher(Address), // authorized publishers
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[contracttype]
#[derive(Clone)]
pub struct Alert {
    pub id: u64,
    pub publisher: Address,
    pub asset_code: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: u64,
    pub active: bool,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct DecentralizedAlertFeed;

#[contractimpl]
impl DecentralizedAlertFeed {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::AlertCount, &0u64);
    }

    // ── Publisher management ──────────────────────────────────────────────────

    /// Authorize a publisher. Admin only.
    pub fn add_publisher(env: Env, publisher: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Publisher(publisher.clone()), &true);
        env.events().publish(
            (Symbol::new(&env, "publisher_added"),),
            publisher,
        );
    }

    /// Revoke a publisher. Admin only.
    pub fn remove_publisher(env: Env, publisher: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .remove(&DataKey::Publisher(publisher));
    }

    pub fn is_publisher(env: Env, publisher: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Publisher(publisher))
            .unwrap_or(false)
    }

    // ── Alerts ────────────────────────────────────────────────────────────────

    /// Publish a new alert. Authorized publishers only.
    pub fn publish_alert(
        env: Env,
        publisher: Address,
        asset_code: String,
        severity: AlertSeverity,
        message: String,
    ) -> u64 {
        publisher.require_auth();
        if !env
            .storage()
            .persistent()
            .get::<DataKey, bool>(&DataKey::Publisher(publisher.clone()))
            .unwrap_or(false)
        {
            panic!("not authorized publisher");
        }

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::AlertCount)
            .unwrap_or(0);

        let alert = Alert {
            id,
            publisher: publisher.clone(),
            asset_code: asset_code.clone(),
            severity: severity.clone(),
            message,
            timestamp: env.ledger().timestamp(),
            active: true,
        };
        env.storage().persistent().set(&DataKey::Alert(id), &alert);
        env.storage().instance().set(&DataKey::AlertCount, &(id + 1));

        env.events().publish(
            (Symbol::new(&env, "alert_published"),),
            (id, asset_code, severity),
        );
        id
    }

    /// Deactivate an alert (publisher or admin).
    pub fn deactivate_alert(env: Env, caller: Address, alert_id: u64) {
        caller.require_auth();
        let key = DataKey::Alert(alert_id);
        let mut alert: Alert = env
            .storage()
            .persistent()
            .get(&key)
            .expect("alert not found");

        let is_admin = Self::is_admin(&env, &caller);
        let is_publisher = alert.publisher == caller;
        if !is_admin && !is_publisher {
            panic!("unauthorized");
        }

        alert.active = false;
        env.storage().persistent().set(&key, &alert);
    }

    pub fn get_alert(env: Env, alert_id: u64) -> Option<Alert> {
        env.storage().persistent().get(&DataKey::Alert(alert_id))
    }

    pub fn alert_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::AlertCount).unwrap_or(0)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).expect("not initialized")
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn is_admin(env: &Env, addr: &Address) -> bool {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("not initialized");
        admin == *addr
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

    fn setup() -> (Env, DecentralizedAlertFeedClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(DecentralizedAlertFeed, ());
        let client = DecentralizedAlertFeedClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    #[test]
    fn test_add_publisher_and_publish() {
        let (env, client, _) = setup();
        let publisher = Address::generate(&env);
        client.add_publisher(&publisher);
        assert!(client.is_publisher(&publisher));

        let code = String::from_str(&env, "SCAM");
        let msg = String::from_str(&env, "Liquidity drained 90%");
        let id = client.publish_alert(&publisher, &code, &AlertSeverity::Critical, &msg);
        assert_eq!(id, 0);
        assert_eq!(client.alert_count(), 1);

        let alert = client.get_alert(&0).unwrap();
        assert!(alert.active);
        assert_eq!(alert.severity, AlertSeverity::Critical);
    }

    #[test]
    #[should_panic(expected = "not authorized publisher")]
    fn test_unauthorized_publish_fails() {
        let (env, client, _) = setup();
        let rando = Address::generate(&env);
        let code = String::from_str(&env, "X");
        let msg = String::from_str(&env, "msg");
        client.publish_alert(&rando, &code, &AlertSeverity::Info, &msg);
    }

    #[test]
    fn test_deactivate_alert() {
        let (env, client, _) = setup();
        let publisher = Address::generate(&env);
        client.add_publisher(&publisher);
        let code = String::from_str(&env, "RUG");
        let msg = String::from_str(&env, "Rug detected");
        client.publish_alert(&publisher, &code, &AlertSeverity::Warning, &msg);

        client.deactivate_alert(&publisher, &0);
        assert!(!client.get_alert(&0).unwrap().active);
    }

    #[test]
    fn test_remove_publisher() {
        let (env, client, _) = setup();
        let publisher = Address::generate(&env);
        client.add_publisher(&publisher);
        client.remove_publisher(&publisher);
        assert!(!client.is_publisher(&publisher));
    }

    #[test]
    fn test_multiple_alerts() {
        let (env, client, _) = setup();
        let publisher = Address::generate(&env);
        client.add_publisher(&publisher);

        for i in 0..3u64 {
            let code = String::from_str(&env, "TOKEN");
            let msg = String::from_str(&env, "alert");
            let id = client.publish_alert(&publisher, &code, &AlertSeverity::Info, &msg);
            assert_eq!(id, i);
        }
        assert_eq!(client.alert_count(), 3);
    }
}
