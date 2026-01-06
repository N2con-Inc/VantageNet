//! OpenAPI schema validation for response compatibility.

use regex::Regex;
use serde_json::Value;

/// Extract the response schema from an OpenAPI spec for a given path, method, and status.
///
/// This handles:
/// - Path parameter templating (e.g., `/api/v1/hosts/{id}` matches `/api/v1/hosts/123`)
/// - Schema references ($ref)
/// - Nested response content types
pub fn extract_response_schema(
    openapi: &Value,
    path: &str,
    method: &str,
    status: u16,
) -> Option<Value> {
    let paths = openapi.get("paths")?.as_object()?;

    // Find matching path (handle path parameters)
    let (_, path_item) = find_matching_path(paths, path)?;

    // Get the operation for this method
    let operation = path_item.get(method.to_lowercase().as_str())?;

    // Get responses
    let responses = operation.get("responses")?.as_object()?;

    // Try exact status, then "default"
    let status_str = status.to_string();
    let response = responses
        .get(&status_str)
        .or_else(|| responses.get("default"))?;

    // Get content -> application/json -> schema
    let schema = response
        .get("content")?
        .get("application/json")?
        .get("schema")?;

    // Resolve $ref if present
    Some(resolve_ref(openapi, schema))
}

/// Find a matching path in the OpenAPI paths, handling path parameters.
fn find_matching_path<'a>(
    paths: &'a serde_json::Map<String, Value>,
    actual_path: &'a str,
) -> Option<(&'a str, &'a Value)> {
    // First try exact match
    if let Some(item) = paths.get(actual_path) {
        return Some((actual_path, item));
    }

    // Build regex patterns for paths with parameters
    for (template, item) in paths {
        if path_matches(template, actual_path) {
            return Some((template.as_str(), item));
        }
    }

    None
}

/// Check if an actual path matches an OpenAPI path template.
fn path_matches(template: &str, actual: &str) -> bool {
    // Convert template to regex: /api/hosts/{id} -> /api/hosts/[^/]+
    let pattern = template
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "[^/]+".to_string()
            } else {
                regex::escape(segment)
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    let pattern = format!("^{}$", pattern);
    Regex::new(&pattern)
        .map(|re| re.is_match(actual))
        .unwrap_or(false)
}

/// Resolve a $ref in the OpenAPI schema.
fn resolve_ref(openapi: &Value, schema: &Value) -> Value {
    if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
        // Parse ref like "#/components/schemas/Host"
        let parts: Vec<&str> = ref_path.trim_start_matches("#/").split('/').collect();

        let mut current = openapi;
        for part in parts {
            current = match current.get(part) {
                Some(v) => v,
                None => return schema.clone(),
            };
        }

        // Recursively resolve refs in the resolved schema
        resolve_refs_recursive(openapi, current.clone())
    } else if schema.is_object() {
        // Recursively resolve refs in nested schemas
        resolve_refs_recursive(openapi, schema.clone())
    } else {
        schema.clone()
    }
}

/// Recursively resolve all $refs in a schema.
fn resolve_refs_recursive(openapi: &Value, schema: Value) -> Value {
    match schema {
        Value::Object(mut map) => {
            // If this object has a $ref, resolve it
            if let Some(ref_val) = map.get("$ref") {
                if let Some(ref_path) = ref_val.as_str() {
                    let parts: Vec<&str> = ref_path.trim_start_matches("#/").split('/').collect();
                    let mut current = openapi;
                    for part in parts {
                        current = match current.get(part) {
                            Some(v) => v,
                            None => return Value::Object(map),
                        };
                    }
                    return resolve_refs_recursive(openapi, current.clone());
                }
            }

            // Recursively resolve refs in all values
            for (_, value) in map.iter_mut() {
                *value = resolve_refs_recursive(openapi, value.clone());
            }
            Value::Object(map)
        }
        Value::Array(mut arr) => {
            for item in arr.iter_mut() {
                *item = resolve_refs_recursive(openapi, item.clone());
            }
            Value::Array(arr)
        }
        other => other,
    }
}

/// Validate a response body against an OpenAPI schema.
///
/// Returns Ok(()) if valid, or an error message describing validation failures.
pub fn validate_response(
    openapi: &Value,
    path: &str,
    method: &str,
    status: u16,
    response_body: &Value,
) -> Result<(), String> {
    let schema = extract_response_schema(openapi, path, method, status)
        .ok_or_else(|| format!("No schema found for {} {} -> {}", method, path, status))?;

    // Use jsonschema crate for validation
    let validator = jsonschema::validator_for(&schema)
        .map_err(|e| format!("Failed to compile schema: {}", e))?;

    let errors: Vec<String> = validator
        .iter_errors(response_body)
        .map(|e| format!("{} at {}", e, e.instance_path))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Schema validation failed:\n  {}",
            errors.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matches() {
        assert!(path_matches("/api/v1/hosts", "/api/v1/hosts"));
        assert!(path_matches(
            "/api/v1/hosts/{id}",
            "/api/v1/hosts/123e4567-e89b-12d3-a456-426614174000"
        ));
        assert!(path_matches(
            "/api/v1/daemons/{id}/startup",
            "/api/v1/daemons/abc-123/startup"
        ));
        assert!(!path_matches("/api/v1/hosts/{id}", "/api/v1/subnets/123"));
        assert!(!path_matches("/api/v1/hosts", "/api/v1/hosts/123"));
    }
}
