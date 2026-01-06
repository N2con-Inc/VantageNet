use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::storage::traits::{SqlValue, StorableEntity},
    user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase},
    users::r#impl::permissions::UserOrgPermissions,
};

impl StorableEntity for UserApiKey {
    type BaseData = UserApiKeyBase;

    fn table_name() -> &'static str {
        "user_api_keys"
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn network_id(&self) -> Option<Uuid> {
        // User API keys use a junction table for network access
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    key,
                    name,
                    user_id,
                    organization_id,
                    permissions,
                    last_used,
                    expires_at,
                    is_enabled,
                    tags,
                    network_ids: _, // Stored in junction table, not here
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "key",
                "name",
                "user_id",
                "organization_id",
                "permissions",
                "last_used",
                "expires_at",
                "is_enabled",
                "tags",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(key),
                SqlValue::String(name),
                SqlValue::Uuid(user_id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(permissions.to_string()),
                SqlValue::OptionTimestamp(last_used),
                SqlValue::OptionTimestamp(expires_at),
                SqlValue::Bool(is_enabled),
                SqlValue::UuidArray(tags),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let permissions_str: String = row.get("permissions");
        let permissions = permissions_str
            .parse::<UserOrgPermissions>()
            .unwrap_or_default();

        Ok(UserApiKey {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: UserApiKeyBase {
                key: row.get("key"),
                name: row.get("name"),
                user_id: row.get("user_id"),
                organization_id: row.get("organization_id"),
                permissions,
                last_used: row.get("last_used"),
                expires_at: row.get("expires_at"),
                is_enabled: row.get("is_enabled"),
                tags: row.get("tags"),
                network_ids: Vec::new(), // Hydrated separately from junction table
            },
        })
    }
}
