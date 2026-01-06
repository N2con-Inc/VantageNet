//! Shared types for compatibility testing.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A captured HTTP request/response exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedExchange {
    pub method: String,
    pub path: String,
    pub request_body: serde_json::Value,
    pub response_status: u16,
    pub response_body: serde_json::Value,
}

/// A manifest of captured exchanges for a specific version.
#[derive(Debug, Serialize, Deserialize)]
pub struct FixtureManifest {
    pub version: String,
    pub exchanges: Vec<CapturedExchange>,
}

const FIXTURES_DIR: &str = "src/tests/compat/fixtures";

/// Load all fixture versions that have the specified manifest file.
pub fn get_fixture_versions(manifest_name: &str) -> Vec<String> {
    let dir = Path::new(FIXTURES_DIR);

    if !dir.exists() {
        return Vec::new();
    }

    fs::read_dir(dir)
        .expect("Failed to read fixtures directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().into_string().ok()?;

            if name.starts_with('v') && entry.path().is_dir() {
                let manifest_path = entry.path().join(manifest_name);
                if manifest_path.exists() {
                    Some(name.trim_start_matches('v').to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Load a fixture manifest for a specific version.
pub fn load_manifest(version: &str, manifest_name: &str) -> Option<FixtureManifest> {
    let path = Path::new(FIXTURES_DIR)
        .join(format!("v{}", version))
        .join(manifest_name);

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Load the OpenAPI spec for a specific version.
pub fn load_openapi_spec(version: &str) -> Option<serde_json::Value> {
    let path = Path::new(FIXTURES_DIR)
        .join(format!("v{}", version))
        .join("openapi.json");

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}
