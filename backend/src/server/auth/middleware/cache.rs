//! Request-scoped entity caching utilities.
//!
//! Provides caching for frequently looked-up entities within a single request
//! to avoid duplicate database queries across middleware and handlers.
//!
//! # Usage
//!
//! ```ignore
//! // In middleware or extractors:
//! let org = CachedOrganization::get_or_load(parts, app_state, &org_id).await?;
//!
//! // Subsequent calls with same ID return cached value:
//! let org = CachedOrganization::get_or_load(parts, app_state, &org_id).await?;
//! ```

use crate::server::{
    config::AppState,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    shared::{services::traits::CrudService, types::api::ApiError},
    users::r#impl::base::User,
};
use axum::http::request::Parts;
use uuid::Uuid;

/// Cached organization lookup stored in request extensions.
#[derive(Clone)]
pub struct CachedOrganization(pub Organization);

impl CachedOrganization {
    /// Get organization from cache or load from DB and cache it.
    pub async fn get_or_load(
        parts: &mut Parts,
        app_state: &AppState,
        organization_id: &Uuid,
    ) -> Result<Organization, ApiError> {
        // Check cache first
        if let Some(cached) = parts.extensions.get::<CachedOrganization>()
            && cached.0.id == *organization_id
        {
            return Ok(cached.0.clone());
        }

        // Load from DB
        let organization = app_state
            .services
            .organization_service
            .get_by_id(organization_id)
            .await
            .map_err(|_| ApiError::internal_error("Failed to load organization"))?
            .ok_or_else(|| ApiError::forbidden("Organization not found"))?;

        // Cache for subsequent extractors
        parts
            .extensions
            .insert(CachedOrganization(organization.clone()));

        Ok(organization)
    }
}

/// Cached user lookup stored in request extensions.
#[derive(Clone)]
pub struct CachedUser(pub User);

impl CachedUser {
    /// Get user from cache or load from DB and cache it.
    pub async fn get_or_load(
        parts: &mut Parts,
        app_state: &AppState,
        user_id: &Uuid,
    ) -> Result<User, ApiError> {
        // Check cache first
        if let Some(cached) = parts.extensions.get::<CachedUser>()
            && cached.0.id == *user_id
        {
            return Ok(cached.0.clone());
        }

        // Load from DB
        let user = app_state
            .services
            .user_service
            .get_by_id(user_id)
            .await
            .map_err(|_| ApiError::internal_error("Failed to load user"))?
            .ok_or_else(|| ApiError::forbidden("User not found"))?;

        // Cache for subsequent extractors
        parts.extensions.insert(CachedUser(user.clone()));

        Ok(user)
    }
}

/// Cached network lookup stored in request extensions.
#[derive(Clone)]
pub struct CachedNetwork(pub Network);

impl CachedNetwork {
    /// Get network from cache or load from DB and cache it.
    pub async fn get_or_load(
        parts: &mut Parts,
        app_state: &AppState,
        network_id: &Uuid,
    ) -> Result<Network, ApiError> {
        // Check cache first
        if let Some(cached) = parts.extensions.get::<CachedNetwork>()
            && cached.0.id == *network_id
        {
            return Ok(cached.0.clone());
        }

        // Load from DB
        let network = app_state
            .services
            .network_service
            .get_by_id(network_id)
            .await
            .map_err(|_| ApiError::internal_error("Failed to load network"))?
            .ok_or_else(|| ApiError::forbidden("Network not found"))?;

        // Cache for subsequent extractors
        parts.extensions.insert(CachedNetwork(network.clone()));

        Ok(network)
    }
}
