use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct VantageNetServer;

impl ServiceDefinition for VantageNetServer {
    fn name(&self) -> &'static str {
        "VantageNet Server"
    }
    fn description(&self) -> &'static str {
        "Automatically discover and visually document network infrastructure"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::VantageNet
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::new_tcp(60072), "/api/health", "vantagenet", None)
    }

    fn logo_url(&self) -> &'static str {
        "/logos/vantagenet-logo.png"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<VantageNetServer>
));
