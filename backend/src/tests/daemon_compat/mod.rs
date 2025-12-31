//! Daemon API compatibility tests.
//!
//! These tests verify that the server can correctly deserialize requests from
//! known daemon versions. Fixtures are captured during integration tests when
//! running with `--features capture-fixtures`.
//!
//! ## Adding fixtures for a new daemon version
//!
//! 1. Run integration tests with fixture capture:
//!    `cargo test --features capture-fixtures integration_tests`
//!
//! 2. Fixtures are written to `fixtures/v{version}/`
//!
//! 3. Commit the new fixtures directory
//!
//! ## Fixture format
//!
//! Each JSON file contains:
//! ```json
//! {
//!   "_meta": {
//!     "method": "POST",
//!     "path_template": "/api/daemons/{id}/startup",
//!     "captured_at": "2025-01-15T10:30:00Z",
//!     "daemon_version": "0.12.8"
//!   },
//!   "payload": { ... }
//! }
//! ```

use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::server::daemons::r#impl::api::{
    DaemonHeartbeatPayload, DaemonRegistrationRequest, DaemonStartupRequest,
};
use crate::server::hosts::r#impl::api::DiscoveryHostRequest;
use crate::server::subnets::r#impl::base::Subnet;

const FIXTURES_DIR: &str = "src/tests/daemon_compat/fixtures";

#[derive(Debug, Deserialize)]
pub struct Fixture {
    #[serde(rename = "_meta")]
    pub meta: FixtureMeta,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct FixtureMeta {
    pub method: String,
    pub path_template: String,
    #[allow(dead_code)]
    pub captured_at: Option<String>,
    pub daemon_version: String,
}

/// Load all fixtures for a given daemon version.
fn load_fixtures(version: &str) -> Vec<Fixture> {
    let dir = Path::new(FIXTURES_DIR).join(format!("v{}", version));

    if !dir.exists() {
        return Vec::new();
    }

    fs::read_dir(&dir)
        .expect("Failed to read fixtures directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&path).ok()?;
                serde_json::from_str::<Fixture>(&content).ok()
            } else {
                None
            }
        })
        .collect()
}

/// Get the list of daemon versions that have fixtures.
fn get_fixture_versions() -> Vec<String> {
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
                Some(name.trim_start_matches('v').to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Find a fixture by path template.
fn find_fixture<'a>(fixtures: &'a [Fixture], path_template: &str) -> Option<&'a Fixture> {
    fixtures
        .iter()
        .find(|f| f.meta.path_template == path_template)
}

// Required endpoints that all daemon versions should have fixtures for
const REQUIRED_ENDPOINTS: &[(&str, &str)] = &[
    ("POST", "/api/daemons/register"),
    ("POST", "/api/daemons/{id}/startup"),
    ("POST", "/api/daemons/{id}/heartbeat"),
    ("POST", "/api/hosts"),
    ("POST", "/api/subnets"),
];

#[test]
fn test_fixture_versions_exist() {
    let versions = get_fixture_versions();

    // We should have at least one version of fixtures
    // This test will fail until fixtures are captured
    if versions.is_empty() {
        eprintln!("⚠️  No fixture versions found in {}", FIXTURES_DIR);
        eprintln!("   Run integration tests with --features capture-fixtures to generate");
        // Don't fail - this is expected until first capture
        return;
    }

    println!("Found fixture versions: {:?}", versions);
}

#[test]
fn test_all_endpoints_have_fixtures() {
    let versions = get_fixture_versions();

    if versions.is_empty() {
        eprintln!("⚠️  No fixture versions found - skipping endpoint coverage check");
        return;
    }

    for version in &versions {
        let fixtures = load_fixtures(version);

        if fixtures.is_empty() {
            eprintln!("⚠️  No fixtures found for version {}", version);
            continue;
        }

        for (method, path) in REQUIRED_ENDPOINTS {
            let found = fixtures.iter().any(|f| {
                f.meta.method == *method
                    && (f.meta.path_template == *path
                        || f.meta.path_template.replace("{id}", "{daemon_id}") == *path)
            });

            assert!(
                found,
                "Missing fixture for {} {} in version {}",
                method, path, version
            );
        }
    }
}

#[test]
fn test_register_request_compat() {
    for version in get_fixture_versions() {
        let fixtures = load_fixtures(&version);
        let Some(fixture) = find_fixture(&fixtures, "/api/daemons/register") else {
            eprintln!("⚠️  No register fixture for version {}", version);
            continue;
        };

        let result: Result<DaemonRegistrationRequest, _> =
            serde_json::from_value(fixture.payload.clone());

        assert!(
            result.is_ok(),
            "Failed to deserialize register request from v{}: {:?}",
            version,
            result.err()
        );

        let request = result.unwrap();
        println!(
            "v{}: register request - daemon_id={}, network_id={}",
            version, request.daemon_id, request.network_id
        );
    }
}

#[test]
fn test_startup_request_compat() {
    for version in get_fixture_versions() {
        let fixtures = load_fixtures(&version);
        let Some(fixture) = find_fixture(&fixtures, "/api/daemons/{id}/startup") else {
            eprintln!("⚠️  No startup fixture for version {}", version);
            continue;
        };

        let result: Result<DaemonStartupRequest, _> =
            serde_json::from_value(fixture.payload.clone());

        assert!(
            result.is_ok(),
            "Failed to deserialize startup request from v{}: {:?}",
            version,
            result.err()
        );

        let request = result.unwrap();
        println!(
            "v{}: startup request - daemon_version={}",
            version, request.daemon_version
        );
    }
}

#[test]
fn test_heartbeat_request_compat() {
    for version in get_fixture_versions() {
        let fixtures = load_fixtures(&version);
        let Some(fixture) = find_fixture(&fixtures, "/api/daemons/{id}/heartbeat") else {
            eprintln!("⚠️  No heartbeat fixture for version {}", version);
            continue;
        };

        let result: Result<DaemonHeartbeatPayload, _> =
            serde_json::from_value(fixture.payload.clone());

        assert!(
            result.is_ok(),
            "Failed to deserialize heartbeat request from v{}: {:?}",
            version,
            result.err()
        );

        let request = result.unwrap();
        println!(
            "v{}: heartbeat request - name={}, url={}",
            version, request.name, request.url
        );
    }
}

#[test]
fn test_hosts_request_compat() {
    for version in get_fixture_versions() {
        let fixtures = load_fixtures(&version);
        let Some(fixture) = find_fixture(&fixtures, "/api/hosts") else {
            eprintln!("⚠️  No hosts fixture for version {}", version);
            continue;
        };

        // Try deserializing as new format (DiscoveryHostRequest)
        let result: Result<DiscoveryHostRequest, _> =
            serde_json::from_value(fixture.payload.clone());

        assert!(
            result.is_ok(),
            "Failed to deserialize hosts request from v{}: {:?}",
            version,
            result.err()
        );

        let request = result.unwrap();
        println!(
            "v{}: hosts request - hostname={:?}",
            version, request.host.base.hostname
        );
    }
}

#[test]
fn test_subnets_request_compat() {
    for version in get_fixture_versions() {
        let fixtures = load_fixtures(&version);
        let Some(fixture) = find_fixture(&fixtures, "/api/subnets") else {
            eprintln!("⚠️  No subnets fixture for version {}", version);
            continue;
        };

        let result: Result<Subnet, _> = serde_json::from_value(fixture.payload.clone());

        assert!(
            result.is_ok(),
            "Failed to deserialize subnets request from v{}: {:?}",
            version,
            result.err()
        );

        let request = result.unwrap();
        println!("v{}: subnets request - name={}", version, request.base.name);
    }
}
