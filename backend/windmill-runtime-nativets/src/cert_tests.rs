//! Regression tests for custom CA support in the in-process nativets fetch
//! runtime (WIN-2055).
//!
//! `deno_fetch` with `root_cert_store_provider: None` trusts only the Mozilla
//! webpki roots, so scripts calling internal APIs fronted by a corporate CA
//! failed with `invalid peer certificate: UnknownIssuer`. The provider built by
//! `build_native_root_cert_store_provider` merges CAs from `DENO_CERT` /
//! `SSL_CERT_FILE` / `NODE_EXTRA_CA_CERTS` / `DENO_TLS_CA_STORE=system` into the
//! default store. These tests pin that behaviour.

use crate::{build_native_root_cert_store_provider, load_pem_certs_from_path};

/// The CA-related env vars the provider inspects. Cleared around each test so a
/// CI runner that happens to set one of them can't perturb the result.
const CA_ENV_VARS: &[&str] = &[
    "DENO_CERT",
    "SSL_CERT_FILE",
    "NODE_EXTRA_CA_CERTS",
    "DENO_TLS_CA_STORE",
];

fn write_test_ca(suffix: &str) -> std::path::PathBuf {
    let cert = rcgen::generate_simple_self_signed(vec!["windmill-test-ca".to_string()])
        .expect("generate self-signed cert");
    let pem = cert.cert.pem();
    let path = std::env::temp_dir().join(format!(
        "windmill-nativets-ca-{}-{}.pem",
        std::process::id(),
        suffix
    ));
    std::fs::write(&path, pem).expect("write cert");
    path
}

/// Serializes the env-mutating tests against each other — env is process-global,
/// so concurrent `set_var`/`remove_var` would otherwise interleave.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_cleared_ca_env<T>(f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let saved: Vec<(&str, Option<String>)> = CA_ENV_VARS
        .iter()
        .map(|k| (*k, std::env::var(k).ok()))
        .collect();
    for k in CA_ENV_VARS {
        std::env::remove_var(k);
    }
    let out = f();
    for (k, v) in saved {
        match v {
            Some(v) => std::env::set_var(k, v),
            None => std::env::remove_var(k),
        }
    }
    out
}

#[test]
fn load_pem_certs_parses_self_signed_cert() {
    let path = write_test_ca("load");
    let certs = load_pem_certs_from_path(path.to_str().unwrap()).expect("load certs");
    assert_eq!(certs.len(), 1, "expected exactly one cert in the bundle");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_pem_certs_errors_on_missing_file() {
    let missing = std::env::temp_dir().join("windmill-nativets-does-not-exist.pem");
    assert!(load_pem_certs_from_path(missing.to_str().unwrap()).is_err());
}

#[test]
fn no_ca_env_yields_no_provider() {
    // serialize against the env-mutating tests via the shared guard
    with_cleared_ca_env(|| {
        assert!(
            build_native_root_cert_store_provider().is_none(),
            "without any CA env var the provider must stay None (default-only behaviour)"
        );
    });
}

#[test]
fn ssl_cert_file_adds_custom_root() {
    let path = write_test_ca("ssl");
    with_cleared_ca_env(|| {
        std::env::set_var("SSL_CERT_FILE", &path);
        let provider = build_native_root_cert_store_provider()
            .expect("a custom CA was configured, provider must be Some");
        let store = provider.get_or_try_init().expect("store init");
        let default_len = deno_tls::create_default_root_cert_store().len();
        assert_eq!(
            store.len(),
            default_len + 1,
            "custom CA should be added on top of the Mozilla defaults"
        );
    });
    let _ = std::fs::remove_file(&path);
}

#[test]
fn node_extra_ca_certs_adds_custom_root() {
    let path = write_test_ca("node");
    with_cleared_ca_env(|| {
        std::env::set_var("NODE_EXTRA_CA_CERTS", &path);
        let provider = build_native_root_cert_store_provider()
            .expect("provider must be Some for NODE_EXTRA_CA_CERTS");
        let store = provider.get_or_try_init().expect("store init");
        assert_eq!(
            store.len(),
            deno_tls::create_default_root_cert_store().len() + 1
        );
    });
    let _ = std::fs::remove_file(&path);
}

#[test]
fn multiple_ca_env_vars_pointing_at_same_file_dedupe_to_one_root() {
    // The tracing proxy points SSL_CERT_FILE, NODE_EXTRA_CA_CERTS and DENO_CERT at
    // the same bundle; the path dedupe in build_native_root_cert_store_provider
    // loads it once, so the store grows by exactly one (rustls' add does not dedupe).
    let path = write_test_ca("dupe");
    with_cleared_ca_env(|| {
        std::env::set_var("SSL_CERT_FILE", &path);
        std::env::set_var("NODE_EXTRA_CA_CERTS", &path);
        std::env::set_var("DENO_CERT", &path);
        let provider = build_native_root_cert_store_provider().expect("provider must be Some");
        let store = provider.get_or_try_init().expect("store init");
        assert_eq!(
            store.len(),
            deno_tls::create_default_root_cert_store().len() + 1
        );
    });
    let _ = std::fs::remove_file(&path);
}

#[test]
fn worker_config_env_var_adds_custom_root() {
    // A CA configured only through the worker-group config (DB `env_vars_static`
    // / allowlisted forwarded vars) lands in `WORKER_CONFIG.env_vars`, not the
    // worker's own process env. Child Deno/Bun jobs receive it via `.envs(...)`;
    // nativets must pick it up from the same place. Regression for the in-process
    // path missing that source.
    use windmill_common::worker::WORKER_CONFIG;

    let path = write_test_ca("workercfg");
    with_cleared_ca_env(|| {
        let prev = WORKER_CONFIG.load_full();
        let mut cfg = (*prev).clone();
        cfg.env_vars.insert(
            "SSL_CERT_FILE".to_string(),
            path.to_string_lossy().into_owned(),
        );
        WORKER_CONFIG.store(std::sync::Arc::new(cfg));

        let provider = build_native_root_cert_store_provider();
        // restore before asserting so a failure can't leak the mutated global
        WORKER_CONFIG.store(prev);

        let provider = provider.expect("worker-config CA must produce a provider");
        let store = provider.get_or_try_init().expect("store init");
        assert_eq!(
            store.len(),
            deno_tls::create_default_root_cert_store().len() + 1
        );
    });
    let _ = std::fs::remove_file(&path);
}

#[test]
fn deno_cert_adds_custom_root() {
    let path = write_test_ca("deno");
    with_cleared_ca_env(|| {
        std::env::set_var("DENO_CERT", &path);
        let provider =
            build_native_root_cert_store_provider().expect("provider must be Some for DENO_CERT");
        let store = provider.get_or_try_init().expect("store init");
        assert_eq!(
            store.len(),
            deno_tls::create_default_root_cert_store().len() + 1
        );
    });
    let _ = std::fs::remove_file(&path);
}
