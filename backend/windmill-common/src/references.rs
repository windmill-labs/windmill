//! The reference forms an argument can carry in place of a literal.
//!
//! `transform_json_value` in `windmill-worker/src/common.rs` defines the set: it
//! substitutes each of these strings for the variable, resource, or decrypted
//! payload it names before a script sees its arguments. Anything that inspects an
//! argument before that substitution — schema validation, reference bookkeeping —
//! has to agree on the same set, so a form added to that resolver has to be added
//! here too. Miss one and the form resolves at runtime but is rejected before it
//! gets there, which is what #6938 reported for `$res:`.
//!
//! Resource *values* are interpolated by a separate resolver,
//! `transform_json_value_tracked` in `windmill-store/src/resources.rs`, over a
//! subset of these forms; it is not what validates arguments.
//!
//! A `$WM_*` reserved variable is deliberately not one of these: the resolver
//! substitutes it for a string, so a value carrying one is judged by the same
//! rules as any other string.

/// A variable, substituted for its value as a string.
pub const VARIABLE_PREFIX: &str = "$var:";
/// A variable whose value is parsed as JSON before being substituted.
pub const JSON_VARIABLE_PREFIX: &str = "$jsonvar:";
/// A resource, substituted for its interpolated value.
pub const RESOURCE_PREFIX: &str = "$res:";
/// An encrypted payload, carried inline rather than looked up by path.
pub const ENCRYPTED_PREFIX: &str = "$encrypted:";

/// Every form above, for callers that treat them alike.
pub const REFERENCE_PREFIXES: [&str; 4] = [
    VARIABLE_PREFIX,
    JSON_VARIABLE_PREFIX,
    RESOURCE_PREFIX,
    ENCRYPTED_PREFIX,
];

/// Whether `value` is a reference, i.e. a string standing in for a value the
/// worker has not substituted yet.
pub fn is_reference(value: &str) -> bool {
    REFERENCE_PREFIXES.iter().any(|p| value.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_reference_covers_every_prefix() {
        for prefix in REFERENCE_PREFIXES {
            assert!(is_reference(&format!("{prefix}f/team/thing")));
        }
    }

    #[test]
    fn is_reference_rejects_non_references() {
        // `$WM_*` resolves to a string, and a bare `$` is not a reference at all.
        assert!(!is_reference("$WM_JOB_ID"));
        assert!(!is_reference("$5.00"));
        assert!(!is_reference("f/team/thing"));
        assert!(!is_reference(""));
        // The prefix has to lead: a reference is recognised by how it starts.
        assert!(!is_reference("prefixed $res:f/team/thing"));
    }
}
