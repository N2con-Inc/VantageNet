use crate::server::{
    config::AppState,
    shared::handlers::{query::NoFilterQuery, traits::CrudHandlers},
    user_api_keys::{r#impl::base::UserApiKey, service::UserApiKeyService},
};
use uuid::Uuid;

impl CrudHandlers for UserApiKey {
    type Service = UserApiKeyService;
    // User API keys are filtered by user_id in the custom get_all handler
    type FilterQuery = NoFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.user_api_key_service
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // key hash cannot be changed via update (use rotate endpoint instead)
        self.base.key = existing.base.key.clone();
        // last_used is server-set only
        self.base.last_used = existing.base.last_used;
        // user_id and organization_id cannot be changed
        self.base.user_id = existing.base.user_id;
        self.base.organization_id = existing.base.organization_id;
        self.created_at = existing.created_at;
        self.id = existing.id;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
