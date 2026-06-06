//! Tenant/application/worker-pool access-scope helpers.

use super::{dto::AccessScopeBinding, error::ApiError};

const WILDCARD: &str = "*";

/// Validate caller-provided scope bindings and inherit narrower caller scopes.
///
/// Empty bindings mean unrestricted access for unbound principals. Bound
/// principals may only mint tokens inside their current bindings.
///
/// # Errors
///
/// Returns bad request errors for malformed binding values and forbidden errors
/// when a bound principal tries to mint a token outside its own bindings.
pub fn validate_scope_bindings(
    requested: Vec<AccessScopeBinding>,
    inherited: &[AccessScopeBinding],
) -> Result<Vec<AccessScopeBinding>, ApiError> {
    let mut normalized = requested
        .into_iter()
        .filter_map(normalize_binding)
        .collect::<Result<Vec<_>, _>>()?;
    normalized.sort_by(|left, right| {
        (
            left.namespace.as_deref(),
            left.app.as_deref(),
            left.worker_pool.as_deref(),
        )
            .cmp(&(
                right.namespace.as_deref(),
                right.app.as_deref(),
                right.worker_pool.as_deref(),
            ))
    });
    normalized.dedup();

    if inherited.is_empty() {
        return Ok(normalized);
    }
    let effective = if normalized.is_empty() {
        inherited.to_vec()
    } else {
        normalized
    };
    for binding in &effective {
        if !inherited
            .iter()
            .any(|parent| binding_contains(parent, binding))
        {
            return Err(ApiError::forbidden(
                "api token scope binding is outside the current principal scope",
            ));
        }
    }
    Ok(effective)
}

/// Return true when a principal binding list allows one resource tuple.
#[must_use]
pub fn allows_resource(
    bindings: &[AccessScopeBinding],
    namespace: &str,
    app: &str,
    worker_pool: Option<&str>,
) -> bool {
    bindings.is_empty()
        || bindings
            .iter()
            .any(|binding| binding_allows(binding, namespace, app, worker_pool))
}

fn normalize_binding(binding: AccessScopeBinding) -> Option<Result<AccessScopeBinding, ApiError>> {
    let namespace = normalize_part(binding.namespace);
    let app = normalize_part(binding.app);
    let worker_pool = normalize_part(binding.worker_pool);
    if namespace.is_none() && app.is_none() && worker_pool.is_none() {
        return None;
    }
    Some(validate_part(namespace, "namespace").and_then(|namespace| {
        validate_part(app, "app").and_then(|app| {
            validate_part(worker_pool, "worker_pool").map(|worker_pool| AccessScopeBinding {
                namespace,
                app,
                worker_pool,
            })
        })
    }))
}

fn normalize_part(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty() && value != WILDCARD)
}

fn validate_part(value: Option<String>, field: &str) -> Result<Option<String>, ApiError> {
    if let Some(item) = value.as_deref()
        && item
            .chars()
            .any(|character| matches!(character, ',' | ';' | '|'))
    {
        return Err(ApiError::bad_request(format!(
            "api token scope binding {field} cannot contain ',', ';', or '|'"
        )));
    }
    Ok(value)
}

fn binding_contains(parent: &AccessScopeBinding, child: &AccessScopeBinding) -> bool {
    optional_contains(parent.namespace.as_deref(), child.namespace.as_deref())
        && optional_contains(parent.app.as_deref(), child.app.as_deref())
        && optional_contains(parent.worker_pool.as_deref(), child.worker_pool.as_deref())
}

fn binding_allows(
    binding: &AccessScopeBinding,
    namespace: &str,
    app: &str,
    worker_pool: Option<&str>,
) -> bool {
    optional_matches(binding.namespace.as_deref(), namespace)
        && optional_matches(binding.app.as_deref(), app)
        && worker_pool.is_none_or(|actual| optional_matches(binding.worker_pool.as_deref(), actual))
}

fn optional_contains(parent: Option<&str>, child: Option<&str>) -> bool {
    parent.is_none() || parent == child
}

fn optional_matches(expected: Option<&str>, actual: &str) -> bool {
    expected.is_none_or(|expected| expected == actual)
}
