//! `//fetch_response_timeout` parsing. Cheap (no V8), so it stays out of the
//! e2e file.

use windmill_runtime_nativets::get_annotation;

#[test]
fn a_value_is_parsed_and_zero_stays_distinct_from_absent() {
    // Collapsing `Some(0)` to `None` would silently reinstate the default on a
    // script that explicitly asked for no limit.
    assert_eq!(
        get_annotation("//native\n//fetch_response_timeout 30\n").fetch_response_timeout_secs,
        Some(30)
    );
    assert_eq!(
        get_annotation("//fetch_response_timeout 0\n").fetch_response_timeout_secs,
        Some(0)
    );
    assert_eq!(
        get_annotation("//native\n").fetch_response_timeout_secs,
        None
    );
}

#[test]
fn a_malformed_value_falls_back_to_the_default_not_to_no_timeout() {
    for src in [
        "//fetch_response_timeout abc\n",
        "//fetch_response_timeout\n",
        "//fetch_response_timeout -5\n",
    ] {
        assert_eq!(
            get_annotation(src).fetch_response_timeout_secs,
            None,
            "{src:?} should leave the default in force"
        );
    }
}
