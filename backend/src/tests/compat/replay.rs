//! Replay tests for API compatibility.
//!
//! These tests load captured fixtures and replay them against a running
//! server or daemon to verify backwards compatibility.

use super::schema::validate_response;
use super::types::{
    CapturedExchange, FixtureManifest, get_fixture_versions, load_manifest, load_openapi_spec,
};
use regex::Regex;
use uuid::Uuid;

/// Context for replaying requests with substituted IDs.
pub struct ReplayContext {
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub api_key: String,
}

impl ReplayContext {
    /// Substitute IDs in a path.
    /// Replaces any UUID in the path with the daemon_id.
    pub fn substitute_path(&self, path: &str) -> String {
        let uuid_regex = Regex::new(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        )
        .unwrap();

        uuid_regex
            .replace_all(path, self.daemon_id.to_string().as_str())
            .to_string()
    }

    /// Substitute IDs in a request body.
    /// Replaces known ID fields with test context values.
    pub fn substitute_body(&self, body: &serde_json::Value) -> serde_json::Value {
        let mut body = body.clone();

        if let Some(obj) = body.as_object_mut() {
            // Replace known ID fields
            if obj.contains_key("daemon_id") {
                obj.insert(
                    "daemon_id".to_string(),
                    serde_json::json!(self.daemon_id.to_string()),
                );
            }
            if obj.contains_key("network_id") {
                obj.insert(
                    "network_id".to_string(),
                    serde_json::json!(self.network_id.to_string()),
                );
            }
            if obj.contains_key("user_id") {
                obj.insert(
                    "user_id".to_string(),
                    serde_json::json!(self.user_id.to_string()),
                );
            }
            if obj.contains_key("organization_id") {
                obj.insert(
                    "organization_id".to_string(),
                    serde_json::json!(self.organization_id.to_string()),
                );
            }

            // Recursively process nested objects
            for (_, value) in obj.iter_mut() {
                if value.is_object() {
                    *value = self.substitute_body(value);
                }
            }
        }

        body
    }
}

/// Result of replaying an exchange.
pub struct ReplayResult {
    pub exchange: CapturedExchange,
    pub actual_status: u16,
    pub actual_body: serde_json::Value,
    pub status_ok: bool,
    pub schema_validation: Option<Result<(), String>>,
}

impl ReplayResult {
    /// Check if the replay was fully successful (2xx status and valid schema).
    pub fn is_success(&self) -> bool {
        self.status_ok && self.schema_validation.as_ref().map_or(true, |r| r.is_ok())
    }
}

/// Replay a single exchange against a server/daemon.
pub async fn replay_exchange(
    client: &reqwest::Client,
    base_url: &str,
    exchange: &CapturedExchange,
    ctx: &ReplayContext,
    openapi: Option<&serde_json::Value>,
) -> Result<ReplayResult, String> {
    let path = ctx.substitute_path(&exchange.path);
    let url = format!("{}{}", base_url, path);
    let body = ctx.substitute_body(&exchange.request_body);

    let mut req = match exchange.method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url).json(&body),
        "PUT" => client.put(&url).json(&body),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url).json(&body),
        _ => client.get(&url),
    };

    // Add daemon headers for server requests
    req = req
        .header("X-Daemon-ID", ctx.daemon_id.to_string())
        .header("X-API-Key", &ctx.api_key);

    let response = req.send().await.map_err(|e| e.to_string())?;
    let actual_status = response.status().as_u16();
    let actual_body = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::json!({}));

    // Check status is 2xx
    let status_ok = (200..300).contains(&actual_status);

    // Validate response against OpenAPI schema if available
    let schema_validation = openapi.map(|spec| {
        validate_response(
            spec,
            &exchange.path, // Use original path for schema lookup
            &exchange.method,
            actual_status,
            &actual_body,
        )
    });

    Ok(ReplayResult {
        exchange: exchange.clone(),
        actual_status,
        actual_body,
        status_ok,
        schema_validation,
    })
}

/// Replay all exchanges from a manifest.
pub async fn replay_manifest(
    client: &reqwest::Client,
    base_url: &str,
    manifest: &FixtureManifest,
    ctx: &ReplayContext,
    openapi: Option<&serde_json::Value>,
) -> Vec<Result<ReplayResult, String>> {
    let mut results = Vec::new();

    for exchange in &manifest.exchanges {
        let result = replay_exchange(client, base_url, exchange, ctx, openapi).await;
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test server compatibility with daemon requests from previous versions.
    ///
    /// This test replays daemon→server exchanges captured from previous daemon
    /// versions to ensure the server still handles them correctly and returns
    /// schema-compatible responses.
    #[tokio::test]
    #[ignore = "requires running server with test data"]
    async fn test_server_handles_old_daemon_requests() {
        let versions = get_fixture_versions("daemon_to_server.json");

        if versions.is_empty() {
            eprintln!("No daemon_to_server fixtures found, skipping test");
            return;
        }

        let client = reqwest::Client::new();
        let base_url =
            std::env::var("TEST_SERVER_URL").unwrap_or_else(|_| "http://localhost:60072".into());

        // These would come from test setup
        let ctx = ReplayContext {
            daemon_id: std::env::var("TEST_DAEMON_ID")
                .ok()
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_else(Uuid::new_v4),
            network_id: std::env::var("TEST_NETWORK_ID")
                .ok()
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_else(Uuid::new_v4),
            user_id: std::env::var("TEST_USER_ID")
                .ok()
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_else(Uuid::new_v4),
            organization_id: std::env::var("TEST_ORG_ID")
                .ok()
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_else(Uuid::new_v4),
            api_key: std::env::var("TEST_API_KEY").unwrap_or_default(),
        };

        for version in versions {
            let Some(manifest) = load_manifest(&version, "daemon_to_server.json") else {
                continue;
            };

            // Load OpenAPI spec for schema validation
            let openapi = load_openapi_spec(&version);
            if openapi.is_none() {
                eprintln!(
                    "Warning: No OpenAPI spec for v{}, skipping schema validation",
                    version
                );
            }

            println!("Testing server compatibility with daemon v{}", version);

            let results =
                replay_manifest(&client, &base_url, &manifest, &ctx, openapi.as_ref()).await;

            for result in results {
                match result {
                    Ok(r) if r.is_success() => {
                        println!(
                            "  ✓ {} {} -> {} (schema: valid)",
                            r.exchange.method, r.exchange.path, r.actual_status
                        );
                    }
                    Ok(r) if r.status_ok => {
                        // Status OK but schema validation failed
                        let schema_err = r
                            .schema_validation
                            .as_ref()
                            .and_then(|r| r.as_ref().err())
                            .map(|s| s.as_str())
                            .unwrap_or("unknown");
                        panic!(
                            "  ✗ {} {} -> {} (schema validation failed)\n    {}",
                            r.exchange.method, r.exchange.path, r.actual_status, schema_err
                        );
                    }
                    Ok(r) => {
                        panic!(
                            "  ✗ {} {} -> {} (expected 2xx)\n    Response: {}",
                            r.exchange.method,
                            r.exchange.path,
                            r.actual_status,
                            serde_json::to_string_pretty(&r.actual_body).unwrap_or_default()
                        );
                    }
                    Err(e) => {
                        panic!("  ✗ Request failed: {}", e);
                    }
                }
            }
        }
    }

    /// Test daemon compatibility with server requests from previous versions.
    ///
    /// This test replays server→daemon exchanges captured from previous server
    /// versions to ensure the current daemon still handles them correctly.
    #[tokio::test]
    #[ignore = "requires running daemon"]
    async fn test_daemon_handles_old_server_requests() {
        let versions = get_fixture_versions("server_to_daemon.json");

        if versions.is_empty() {
            eprintln!("No server_to_daemon fixtures found, skipping test");
            return;
        }

        let client = reqwest::Client::new();
        let base_url =
            std::env::var("TEST_DAEMON_URL").unwrap_or_else(|_| "http://localhost:60073".into());

        let ctx = ReplayContext {
            daemon_id: Uuid::new_v4(),
            network_id: std::env::var("TEST_NETWORK_ID")
                .ok()
                .and_then(|s| Uuid::parse_str(&s).ok())
                .unwrap_or_else(Uuid::new_v4),
            user_id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            api_key: std::env::var("TEST_API_KEY").unwrap_or_default(),
        };

        for version in versions {
            let Some(manifest) = load_manifest(&version, "server_to_daemon.json") else {
                continue;
            };

            // Load OpenAPI spec for schema validation
            let openapi = load_openapi_spec(&version);

            println!("Testing daemon compatibility with server v{}", version);

            let results =
                replay_manifest(&client, &base_url, &manifest, &ctx, openapi.as_ref()).await;

            for result in results {
                match result {
                    Ok(r) if r.is_success() => {
                        println!(
                            "  ✓ {} {} -> {} (schema: valid)",
                            r.exchange.method, r.exchange.path, r.actual_status
                        );
                    }
                    Ok(r) if r.status_ok => {
                        let schema_err = r
                            .schema_validation
                            .as_ref()
                            .and_then(|r| r.as_ref().err())
                            .map(|s| s.as_str())
                            .unwrap_or("unknown");
                        panic!(
                            "  ✗ {} {} -> {} (schema validation failed)\n    {}",
                            r.exchange.method, r.exchange.path, r.actual_status, schema_err
                        );
                    }
                    Ok(r) => {
                        panic!(
                            "  ✗ {} {} -> {} (expected 2xx)\n    Response: {}",
                            r.exchange.method,
                            r.exchange.path,
                            r.actual_status,
                            serde_json::to_string_pretty(&r.actual_body).unwrap_or_default()
                        );
                    }
                    Err(e) => {
                        panic!("  ✗ Request failed: {}", e);
                    }
                }
            }
        }
    }
}
