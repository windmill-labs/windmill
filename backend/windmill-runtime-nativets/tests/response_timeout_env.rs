//! Its own test binary: the setting is read once per process through a
//! `LazyLock`, so nothing else may resolve it first. Adding a second test to
//! this file breaks that isolation.

use windmill_runtime_nativets::default_fetch_response_timeout_secs;

#[test]
fn the_env_var_is_what_operators_actually_set() {
    // A typo in the variable's name would compile, pass every other test, and
    // silently hand every operator the built-in default instead.
    std::env::set_var("WINDMILL_FETCH_RESPONSE_TIMEOUT_SECS", "17");
    assert_eq!(default_fetch_response_timeout_secs(), 17);
}
