use std::fmt::Display;

use crate::server::shared::api_key_common::{ApiKeyCommon, ApiKeyType};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::serialize_sensitive_info;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct UserApiKeyBase {
    #[serde(default)]
    #[serde(serialize_with = "serialize_sensitive_info")]
    #[schema(read_only, required)]
    pub key: String,
    pub name: String,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    #[serde(default)]
    pub permissions: UserOrgPermissions,
    #[serde(default)]
    #[schema(read_only, required)]
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub is_enabled: bool,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Network IDs this key has access to (hydrated from junction table)
    #[serde(default)]
    pub network_ids: Vec<Uuid>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct UserApiKey {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: UserApiKeyBase,
}

impl UserApiKey {
    /// Check if the key changes should suppress logging
    /// (only logs significant changes, not just last_used updates)
    pub fn suppress_logs(&self, other: &Self) -> bool {
        self.base.key == other.base.key
            && self.base.name == other.base.name
            && self.base.expires_at == other.base.expires_at
            && self.base.is_enabled == other.base.is_enabled
            && self.base.permissions == other.base.permissions
            && self.base.network_ids == other.base.network_ids
    }
}

impl Display for UserApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<UserApiKey> for UserApiKey {
    fn triggers_staleness(&self, _other: Option<UserApiKey>) -> bool {
        false
    }
}

impl ApiKeyCommon for UserApiKey {
    const KEY_TYPE: ApiKeyType = ApiKeyType::User;

    fn key(&self) -> &str {
        &self.base.key
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_enabled(&self) -> bool {
        self.base.is_enabled
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.base.expires_at
    }

    fn last_used(&self) -> Option<DateTime<Utc>> {
        self.base.last_used
    }

    fn tags(&self) -> &[Uuid] {
        &self.base.tags
    }

    fn set_key(&mut self, key: String) {
        self.base.key = key;
    }

    fn set_is_enabled(&mut self, enabled: bool) {
        self.base.is_enabled = enabled;
    }

    fn set_last_used(&mut self, time: Option<DateTime<Utc>>) {
        self.base.last_used = time;
    }
}
