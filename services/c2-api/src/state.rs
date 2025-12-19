use c2_config::ServiceConfig;
use c2_policy::BasicPolicyEngine;
use c2_storage_surreal::SurrealStore;

pub struct AppState {
    pub config: ServiceConfig,
    pub policy: BasicPolicyEngine,
    pub store: SurrealStore,
}
