/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use windmill_common::error::{Error, Result};

/// Comprehensive scope system for JWT token authorization
///
/// Scopes follow the format: {domain}:{action}[:{resource}]
/// Examples:
/// - "jobs:read" - Read access to jobs
/// - "scripts:write:f/folder/*" - Write access to scripts in a folder
/// - "*" - Full access (superuser)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDefinition {
    pub domain: String,
    pub action: String,
    pub kind: Option<String>, // For jobs:run:kind (optional)
    pub resource: Option<Vec<String>>,
}

impl ScopeDefinition {
    pub fn new(
        domain: &str,
        action: &str,
        kind: Option<&str>,
        resource: Option<Vec<String>>,
    ) -> Self {
        Self {
            domain: domain.to_string(),
            action: action.to_string(),
            kind: kind.map(|s| s.to_string()),
            resource: resource,
        }
    }

    pub fn from_scope_string(scope: &str) -> Result<Self> {
        let parts: Vec<&str> = scope.split(':').collect();

        let into_owned_vec = |resources: &str| -> Vec<String> {
            let resources = resources
                .split(",")
                .collect_vec()
                .into_iter()
                .map(ToOwned::to_owned)
                .collect_vec();

            resources
        };

        match parts.len() {
            2 => Ok(Self::new(parts[0], parts[1], None, None)), // domain:action
            3 => {
                if parts[0] == "jobs" && parts[1] == "run" {
                    Ok(Self::new(parts[0], parts[1], Some(parts[2]), None))
                } else {
                    Ok(Self::new(
                        parts[0],
                        parts[1],
                        None,
                        Some(into_owned_vec(parts[2])),
                    ))
                }
            }
            4 => {
                if parts[0] == "jobs" && parts[1] == "run" {
                    Ok(Self::new(
                        parts[0],
                        parts[1],
                        Some(parts[2]),
                        Some(into_owned_vec(parts[3])),
                    ))
                } else {
                    Err(Error::BadRequest(format!(
                        "Invalid 4-part scope: {}",
                        scope
                    )))
                }
            }
            _ => Err(Error::BadRequest(format!(
                "Invalid scope format: {}",
                scope
            ))),
        }
    }

    pub fn as_string(&self) -> String {
        match (&self.kind, &self.resource) {
            (Some(kind), Some(resource)) => {
                format!(
                    "{}:{}:{}:{}",
                    self.domain,
                    self.action,
                    kind,
                    resource.join(",")
                )
            }
            (Some(kind), None) => {
                format!("{}:{}:{}", self.domain, self.action, kind)
            }
            (None, Some(resource)) => {
                format!("{}:{}:{}", self.domain, self.action, resource.join(","))
            }
            (None, None) => format!("{}:{}", self.domain, self.action),
        }
    }

    pub fn includes(&self, other: &ScopeDefinition) -> bool {
        if self.domain != other.domain {
            return false;
        }

        match (self.action.as_str(), other.action.as_str()) {
            (a, b) if (a == "write" && b == "read") || (a == b) => {}
            _ => return false,
        }

        if self.domain == "jobs" && self.action == "run" {
            match (&self.kind, &other.kind) {
                (Some(self_kind), Some(other_kind)) => {
                    if self_kind != other_kind {
                        return false;
                    }
                }
                (Some(_), None) => {
                    return false;
                }
                (None, _) => {
                    return true;
                }
            }
        }

        match (&self.resource, &other.resource) {
            (Some(self_resources), Some(other_resources)) => {
                resources_match(self_resources, other_resources)
            }
            (Some(_), None) => false,
            (None, _) => true,
        }
    }
}

fn resources_match(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    if scope_resources.contains(&"*".to_string()) || accepted_resources.contains(&"*".to_string()) {
        return true;
    }

    if scope_resources.len() <= 4 && accepted_resources.len() <= 4 {
        return resources_match_small(scope_resources, accepted_resources);
    }

    resources_match_large(scope_resources, accepted_resources)
}

fn resources_match_small(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    for required in accepted_resources {
        for scope_resource in scope_resources {
            if resource_matches_pattern(scope_resource, required) {
                return true;
            }
        }
    }
    false
}

fn resources_match_large(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    let mut exact_matches = HashSet::new();
    let mut patterns = Vec::new();

    for scope_resource in scope_resources {
        if scope_resource.contains('*') {
            patterns.push(scope_resource);
        } else {
            exact_matches.insert(scope_resource);
        }
    }

    for accepted_resource in accepted_resources {
        if exact_matches.contains(accepted_resource) {
            return true;
        }

        for pattern in &patterns {
            if resource_matches_pattern(pattern, accepted_resource) {
                return true;
            }
        }
    }

    false
}

fn resource_matches_pattern(scope_resource: &str, accepted_resource: &str) -> bool {
    if scope_resource == accepted_resource {
        return true;
    }

    let matches_wildcard = |pattern: &str, resource: &str| -> bool {
        if !pattern.ends_with("/*") {
            return false;
        }

        let prefix = &pattern[..pattern.len() - 2];

        if !resource.starts_with(prefix) {
            return false;
        }

        // If the resource is exactly the prefix, it matches
        if resource.len() == prefix.len() {
            return true;
        }

        // If the resource is longer, the next character must be '/' for a valid match
        // This prevents "u/user" from matching "u/use/*"
        resource.chars().nth(prefix.len()) == Some('/')
    };

    // Check if either resource is a wildcard pattern and matches the other
    matches_wildcard(scope_resource, accepted_resource)
        || matches_wildcard(accepted_resource, scope_resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_definition_parsing() {
        let scope = ScopeDefinition::from_scope_string("jobs:read").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "read");
        assert_eq!(scope.kind, None);
        assert_eq!(scope.resource, None);

        let scope = ScopeDefinition::from_scope_string("jobs:run:scripts:f/folder/*").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("scripts".to_string()));
        assert_eq!(scope.resource, Some(vec!["f/folder/*".to_string()]));

        let scope = ScopeDefinition::from_scope_string("jobs:run:scripts").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("scripts".to_string()));
        assert_eq!(scope.resource, None);

        let scope = ScopeDefinition::from_scope_string("jobs:run:flows:f/folder/*").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("flows".to_string()));
        assert_eq!(scope.resource, Some(vec!["f/folder/*".to_string()]));

        let scope =
            ScopeDefinition::from_scope_string("scripts:read:path1,path2,f/folder/*").unwrap();
        assert_eq!(scope.domain, "scripts");
        assert_eq!(scope.action, "read");
        assert_eq!(scope.kind, None);
        assert_eq!(
            scope.resource,
            Some(vec![
                "path1".to_string(),
                "path2".to_string(),
                "f/folder/*".to_string()
            ])
        );
    }
}
