use crate::server::{
    config::AppState,
    shared::{
        handlers::{query::NetworkFilterQuery, traits::CrudHandlers},
        storage::traits::StorableEntity,
    },
    topology::{service::main::TopologyService, types::base::Topology},
};
use uuid::Uuid;

impl CrudHandlers for Topology {
    type Service = TopologyService;
    type FilterQuery = NetworkFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.topology_service
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.id = existing.id;
        self.base.parent_id = existing.base.parent_id;
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;
    }

    fn entity_name() -> &'static str {
        Self::table_name()
    }

    fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
