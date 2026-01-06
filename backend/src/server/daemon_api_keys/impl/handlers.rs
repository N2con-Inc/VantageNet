use crate::server::{
    config::AppState,
    daemon_api_keys::{r#impl::base::DaemonApiKey, service::DaemonApiKeyService},
    shared::handlers::{query::NetworkFilterQuery, traits::CrudHandlers},
};
use uuid::Uuid;

impl CrudHandlers for DaemonApiKey {
    type Service = DaemonApiKeyService;
    type FilterQuery = NetworkFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.daemon_api_key_service
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // key hash cannot be changed via update (use rotate endpoint instead)
        self.base.key = existing.base.key.clone();
        // last_used is server-set only
        self.base.last_used = existing.base.last_used;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
