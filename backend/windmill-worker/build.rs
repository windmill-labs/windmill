use deno_fetch::FetchPermissions;
use deno_web::{BlobStore, TimersPermission};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

pub struct PermissionsContainer;

impl FetchPermissions for PermissionsContainer {
    #[inline(always)]
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    #[inline(always)]
    fn check_read(
        &mut self,
        _p: &std::path::Path,
        _api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

impl TimersPermission for PermissionsContainer {
    #[inline(always)]
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

deno_core::extension!(
    fetch,
    esm_entry_point = "ext:fetch/src/runtime.js",
    esm = ["src/runtime.js"],
);

fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());

    let exts = vec![
        deno_webidl::deno_webidl::init_ops_and_esm(),
        deno_url::deno_url::init_ops_and_esm(),
        deno_console::deno_console::init_ops_and_esm(),
        deno_web::deno_web::init_ops_and_esm::<PermissionsContainer>(
            Arc::new(BlobStore::default()),
            None,
        ),
        deno_fetch::deno_fetch::init_ops_and_esm::<PermissionsContainer>(Default::default()),
        fetch::init_ops_and_esm(),
    ];

    // Build the file path to the snapshot.
    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("FETCH_SNAPSHOT.bin");

    // Create the snapshot.
    let _ = deno_core::snapshot_util::create_snapshot(
        deno_core::snapshot_util::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            snapshot_path: snapshot_path,
            startup_snapshot: None,
            extensions: exts,
            compression_cb: None,
            with_runtime_cb: None,
            skip_op_registration: false,
        },
        None,
    );
}
