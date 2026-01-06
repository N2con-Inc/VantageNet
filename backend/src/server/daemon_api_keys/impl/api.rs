use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::daemon_api_keys::r#impl::base::DaemonApiKey;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonApiKeyResponse {
    pub api_key: DaemonApiKey,
    pub key: String,
}
