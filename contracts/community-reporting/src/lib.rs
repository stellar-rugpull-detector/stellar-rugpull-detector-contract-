#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    Report(u64),        // report_id -> Report
    ReportCount,
    Votes(u64),         // report_id -> VoteTally
    UserVoted(u64, Address), // (report_id, voter) -> bool
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum ReportStatus {
    Pending,
    Confirmed,
    Rejected,
}

#[contracttype]
#[derive(Clone)]
pub struct Report {
    pub id: u64,
    pub reporter: Address,
    pub asset_code: String,
    pub description: String,
    pub status: ReportStatus,
    pub created_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct VoteTally {
    pub confirm_votes: u32,
    pub reject_votes: u32,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct CommunityReporting;

#[contractimpl]
impl CommunityReporting {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::ReportCount, &0u64);
    }

    // ── Reports ───────────────────────────────────────────────────────────────

    /// Submit a scam report. Any address can report.
    pub fn submit_report(
        env: Env,
        reporter: Address,
        asset_code: String,
        description: String,
    ) -> u64 {
        reporter.require_auth();
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ReportCount)
            .unwrap_or(0);

        let report = Report {
            id,
            reporter: reporter.clone(),
            asset_code: asset_code.clone(),
            description,
            status: ReportStatus::Pending,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&DataKey::Report(id), &report);
        env.storage().persistent().set(
            &DataKey::Votes(id),
            &VoteTally { confirm_votes: 0, reject_votes: 0 },
        );
        env.storage().instance().set(&DataKey::ReportCount, &(id + 1));

        env.events().publish(
            (Symbol::new(&env, "report_submitted"),),
            (id, asset_code),
        );
        id
    }

    /// Vote on a report. Each address can vote once per report.
    pub fn vote(env: Env, voter: Address, report_id: u64, confirm: bool) {
        voter.require_auth();

        let voted_key = DataKey::UserVoted(report_id, voter.clone());
        if env.storage().temporary().has(&voted_key) {
            panic!("already voted");
        }

        let mut tally: VoteTally = env
            .storage()
            .persistent()
            .get(&DataKey::Votes(report_id))
            .expect("report not found");

        if confirm {
            tally.confirm_votes += 1;
        } else {
            tally.reject_votes += 1;
        }
        env.storage().persistent().set(&DataKey::Votes(report_id), &tally);
        env.storage().temporary().set(&voted_key, &true);

        // Auto-resolve: 5 votes threshold
        Self::try_resolve(&env, report_id, &tally);
    }

    /// Admin can manually resolve a report.
    pub fn resolve_report(env: Env, report_id: u64, confirmed: bool) {
        Self::require_admin(&env);
        Self::set_status(
            &env,
            report_id,
            if confirmed { ReportStatus::Confirmed } else { ReportStatus::Rejected },
        );
    }

    pub fn get_report(env: Env, report_id: u64) -> Option<Report> {
        env.storage().persistent().get(&DataKey::Report(report_id))
    }

    pub fn get_votes(env: Env, report_id: u64) -> Option<VoteTally> {
        env.storage().persistent().get(&DataKey::Votes(report_id))
    }

    pub fn report_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::ReportCount).unwrap_or(0)
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).expect("not initialized")
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn try_resolve(env: &Env, report_id: u64, tally: &VoteTally) {
        const THRESHOLD: u32 = 5;
        if tally.confirm_votes >= THRESHOLD {
            Self::set_status(env, report_id, ReportStatus::Confirmed);
        } else if tally.reject_votes >= THRESHOLD {
            Self::set_status(env, report_id, ReportStatus::Rejected);
        }
    }

    fn set_status(env: &Env, report_id: u64, status: ReportStatus) {
        let key = DataKey::Report(report_id);
        let mut report: Report = env
            .storage()
            .persistent()
            .get(&key)
            .expect("report not found");
        report.status = status;
        env.storage().persistent().set(&key, &report);
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

    fn setup() -> (Env, CommunityReportingClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let id = env.register(CommunityReporting, ());
        let client = CommunityReportingClient::new(&env, &id);
        let admin = Address::generate(&env);
        client.initialize(&admin);
        (env, client, admin)
    }

    #[test]
    fn test_submit_report() {
        let (env, client, _) = setup();
        let reporter = Address::generate(&env);
        let code = String::from_str(&env, "SCAM");
        let desc = String::from_str(&env, "Fake USDC clone");

        let id = client.submit_report(&reporter, &code, &desc);
        assert_eq!(id, 0);
        assert_eq!(client.report_count(), 1);

        let report = client.get_report(&0).unwrap();
        assert_eq!(report.status, ReportStatus::Pending);
    }

    #[test]
    fn test_vote_and_auto_confirm() {
        let (env, client, _) = setup();
        let reporter = Address::generate(&env);
        let code = String::from_str(&env, "RUG");
        let desc = String::from_str(&env, "Rug pull token");
        client.submit_report(&reporter, &code, &desc);

        for _ in 0..5 {
            let voter = Address::generate(&env);
            client.vote(&voter, &0, &true);
        }

        let report = client.get_report(&0).unwrap();
        assert_eq!(report.status, ReportStatus::Confirmed);
    }

    #[test]
    fn test_admin_resolve() {
        let (env, client, _) = setup();
        let reporter = Address::generate(&env);
        let code = String::from_str(&env, "TOKEN");
        let desc = String::from_str(&env, "Suspicious");
        client.submit_report(&reporter, &code, &desc);
        client.resolve_report(&0, &false);
        assert_eq!(client.get_report(&0).unwrap().status, ReportStatus::Rejected);
    }

    #[test]
    #[should_panic(expected = "already voted")]
    fn test_double_vote_rejected() {
        let (env, client, _) = setup();
        let reporter = Address::generate(&env);
        let code = String::from_str(&env, "X");
        let desc = String::from_str(&env, "desc");
        client.submit_report(&reporter, &code, &desc);
        let voter = Address::generate(&env);
        client.vote(&voter, &0, &true);
        client.vote(&voter, &0, &true);
    }
}
