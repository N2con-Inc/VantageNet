use crate::server::auth::middleware::billing::require_billing_for_users;
use crate::server::auth::middleware::fixture_capture::capture_fixtures_middleware;
use crate::server::config::{__path_get_public_config, get_public_config};
use crate::server::github::handlers::{__path_get_stars, get_stars};
use crate::server::openapi::create_docs_router;
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::metadata::{__path_get_metadata_registry, get_metadata_registry};
use crate::server::{
    auth::handlers as auth_handlers, billing::handlers as billing_handlers,
    bindings::handlers as binding_handlers, config::AppState,
    daemon_api_keys::handlers as daemon_api_key_handlers, daemons::handlers as daemon_handlers,
    discovery::handlers as discovery_handlers, groups::handlers as group_handlers,
    hosts::handlers as host_handlers, interfaces::handlers as interface_handlers,
    invites::handlers as invite_handlers, networks::handlers as network_handlers,
    organizations::handlers as organization_handlers, ports::handlers as port_handlers,
    services::handlers as service_handlers, shares::handlers as share_handlers,
    subnets::handlers as subnet_handlers, tags::handlers as tag_handlers,
    topology::handlers as topology_handlers, user_api_keys::handlers as user_api_key_handlers,
    users::handlers as user_handlers,
};
use axum::Json;
use axum::Router;
use axum::http::HeaderValue;
use axum::middleware;
use reqwest::header;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::set_header::SetResponseHeaderLayer;
use utoipa::ToSchema;
use utoipa::openapi::OpenApi;
use utoipa_axum::router::OpenApiRouter;

/// Version information for API compatibility checking
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    /// Current API version (integer, increments on breaking changes)
    pub api_version: u32,
    /// Server version (semver)
    #[schema(value_type = String, example = "0.12.10")]
    pub server_version: Version,
    /// Minimum client version that can use this API (optional, for future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub min_compatible_client: Option<Version>,
}

/// Get API version information
#[utoipa::path(
    get,
    path = "/api/version",
    tag = "system",
    responses(
        (status = 200, description = "Version information", body = ApiResponse<VersionInfo>)
    )
)]
pub async fn get_version() -> Json<ApiResponse<VersionInfo>> {
    Json(ApiResponse::success(VersionInfo {
        api_version: 1,
        server_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
        min_compatible_client: None,
    }))
}

/// Creates the OpenApiRouter with all documented API routes.
/// This is the single source of truth for route definitions.
/// Used by both the server and OpenAPI spec generation.
/// All entity routes are versioned under /api/v1/.
pub fn create_openapi_routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .nest("/api/v1/hosts", host_handlers::create_router())
        .nest("/api/v1/interfaces", interface_handlers::create_router())
        .nest("/api/v1/subnets", subnet_handlers::create_router())
        .nest("/api/v1/networks", network_handlers::create_router())
        .nest("/api/v1/groups", group_handlers::create_router())
        .nest("/api/v1/daemons", daemon_handlers::create_router())
        .nest("/api/v1/discovery", discovery_handlers::create_router())
        .nest("/api/v1/services", service_handlers::create_router())
        .nest("/api/v1/users", user_handlers::create_router())
        .nest(
            "/api/v1/organizations",
            organization_handlers::create_router(),
        )
        .nest("/api/v1/invites", invite_handlers::create_router())
        .nest("/api/v1/tags", tag_handlers::create_router())
        .nest("/api/v1/ports", port_handlers::create_router())
        .nest("/api/v1/bindings", binding_handlers::create_router())
        // API key routes (versioned)
        .nest("/api/v1/auth/keys", user_api_key_handlers::create_router())
        .nest(
            "/api/v1/auth/daemon",
            daemon_api_key_handlers::create_router(),
        )
        // Topology endpoints (tagged as internal - hidden from public docs)
        .nest("/api/v1/topology", topology_handlers::create_router())
}

/// Creates the application router and returns both the router and OpenAPI spec.
/// The OpenAPI spec is built from annotated handlers using utoipa-axum.
pub fn create_router(state: Arc<AppState>) -> (Router<Arc<AppState>>, OpenApi) {
    // Routes that require billing for requests
    let billed_routes = create_openapi_routes();

    // Extract OpenAPI spec and convert to regular Router for middleware application
    let (billed_router, mut openapi) = billed_routes.split_for_parts();
    let billed_router = billed_router.layer(middleware::from_fn_with_state(
        state,
        require_billing_for_users,
    ));

    // Extract OpenAPI from billing, shares, and auth routes (exempt from billing middleware but need types)
    // Shares are versioned because they are user facing, auth and billing is unversioned
    let (billing_router, billing_openapi) = OpenApiRouter::new()
        .nest("/api/billing", billing_handlers::create_router())
        .split_for_parts();
    let (shares_router, shares_openapi) = OpenApiRouter::new()
        .nest("/api/v1/shares", share_handlers::create_router())
        .split_for_parts();
    let (auth_router, auth_openapi) = OpenApiRouter::new()
        .nest("/api/auth", auth_handlers::create_router()) // Unversioned - session auth
        .split_for_parts();

    // Daemon-internal endpoints (unversioned - daemons call these, not users)
    let (daemon_internal_router, daemon_internal_openapi) = OpenApiRouter::new()
        .nest("/api/daemons", daemon_handlers::create_internal_router())
        .split_for_parts();

    // Legacy routes for backwards compatibility with older daemons (v0.12.x)
    // These are not documented in OpenAPI but must remain functional
    let legacy_entity_router: Router<Arc<AppState>> = Router::new()
        .nest("/api/hosts", host_handlers::create_router().into())
        .nest("/api/subnets", subnet_handlers::create_router().into())
        .nest("/api/services", service_handlers::create_router().into())
        .nest("/api/groups", group_handlers::create_router().into())
        .nest("/api/discovery", discovery_handlers::create_router().into());

    // Version endpoint (unversioned - used to check API version)
    let (version_router, version_openapi) = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(get_version))
        .split_for_parts();

    // Merge OpenAPI specs into main spec
    openapi.merge(billing_openapi);
    openapi.merge(shares_openapi);
    openapi.merge(auth_openapi);
    openapi.merge(daemon_internal_openapi);
    openapi.merge(version_openapi);

    // Routes exempt from billing checks
    // Note: /api/health is defined in server.rs outside middleware stack
    let exempt_routes = Router::new()
        .merge(billing_router)
        .merge(shares_router)
        .merge(auth_router)
        .merge(daemon_internal_router)
        .merge(legacy_entity_router)
        .merge(version_router);

    // Cacheable routes with OpenAPI documentation (also exempt from billing)
    let (cacheable_router, cacheable_openapi) = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(get_metadata_registry))
        .routes(utoipa_axum::routes!(get_public_config))
        .routes(utoipa_axum::routes!(get_stars))
        .split_for_parts();
    let cacheable_routes = cacheable_router.layer(SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("max-age=3600, must-revalidate"),
    ));
    openapi.merge(cacheable_openapi);

    let router = Router::new()
        .merge(billed_router)
        .merge(exempt_routes)
        .merge(cacheable_routes)
        .merge(create_docs_router(openapi.clone()))
        // Fixture capture middleware (no-op unless capture-fixtures feature is enabled)
        .layer(middleware::from_fn(capture_fixtures_middleware));

    (router, openapi)
}
