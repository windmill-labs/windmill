#[cfg(feature = "deno_core")]
use deno_fetch::FetchPermissions;
#[cfg(feature = "deno_core")]
use deno_net::NetPermissions;
#[cfg(feature = "deno_core")]
use deno_web::{BlobStore, TimersPermission};
#[cfg(feature = "deno_core")]
use std::borrow::Cow;
#[cfg(feature = "deno_core")]
use std::env;
#[cfg(feature = "deno_core")]
use std::io::Write;
#[cfg(feature = "deno_core")]
use std::path::{Path, PathBuf};
#[cfg(feature = "deno_core")]
use std::sync::Arc;

// #[cfg(feature = "deno_core")]
pub struct PermissionsContainer;

#[cfg(feature = "deno_core")]
impl FetchPermissions for PermissionsContainer {
    #[inline(always)]
    fn check_net_url(
        &mut self,
        _url: &deno_core::url::Url,
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }

    #[inline(always)]
    fn check_read<'a>(
        &mut self,
        _resolved: bool,
        _p: &'a std::path::Path,
        _api_name: &str,
    ) -> Result<Cow<'a, std::path::Path>, deno_io::fs::FsError> {
        unreachable!("snapshotting")
    }
}

#[cfg(feature = "deno_core")]
impl TimersPermission for PermissionsContainer {
    #[inline(always)]
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

#[cfg(feature = "deno_core")]
impl NetPermissions for PermissionsContainer {
    fn check_read<'a>(
        &mut self,
        _p: &'a str,
        _api_name: &str,
    ) -> Result<PathBuf, deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }

    fn check_write<'a>(
        &mut self,
        _p: &'a str,
        _api_name: &str,
    ) -> Result<PathBuf, deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }

    fn check_net<T: AsRef<str>>(
        &mut self,
        _host: &(T, Option<u16>),
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }

    fn check_write_path<'a>(
        &mut self,
        _: &'a Path,
        _: &str,
    ) -> Result<Cow<'a, Path>, deno_permissions::PermissionCheckError> {
        todo!()
    }
}

#[cfg(feature = "deno_core")]
deno_core::extension!(
    fetch,
    esm_entry_point = "ext:fetch/src/runtime.js",
    esm = ["src/runtime.js"],
);

#[cfg(feature = "deno_core")]
fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());

    let exts = vec![
        deno_telemetry::deno_telemetry::init_ops_and_esm(),
        deno_webidl::deno_webidl::init_ops_and_esm(),
        deno_url::deno_url::init_ops_and_esm(),
        deno_console::deno_console::init_ops_and_esm(),
        deno_web::deno_web::init_ops_and_esm::<PermissionsContainer>(
            Arc::new(BlobStore::default()),
            None,
        ),
        deno_fetch::deno_fetch::init_ops_and_esm::<PermissionsContainer>(Default::default()),
        deno_net::deno_net::init_ops_and_esm::<PermissionsContainer>(None, None),
        fetch::init_ops_and_esm(),
    ];

    // Build the file path to the snapshot.
    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("FETCH_SNAPSHOT.bin");

    // Create the snapshot.
    let output = deno_core::snapshot::create_snapshot(
        deno_core::snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            startup_snapshot: None,
            extension_transpiler: Some(std::rc::Rc::new(|specifier, source| {
                deno_runtime::transpile::maybe_transpile_source(specifier, source)
            })),
            extensions: exts,
            with_runtime_cb: None,
            skip_op_registration: false,
        },
        None,
    )
    .unwrap();

    // NOTE(bartlomieju): Compressing the TSC snapshot in debug build took
    // ~45s on M1 MacBook Pro; without compression it took ~1s.
    // Thus we're not using compressed snapshot, trading off
    // a lot of build time for some startup time in debug build.
    let mut file = std::fs::File::create(snapshot_path).unwrap();
    // if cfg!(debug_assertions) {
    file.write_all(&output.output).unwrap();
    // } else {
    //     let mut vec = Vec::with_capacity(output.output.len());
    //     vec.extend((output.output.len() as u32).to_le_bytes());
    //     vec.extend_from_slice(
    //         &zstd::bulk::compress(&output.output, 22).expect("snapshot compression failed"),
    //     );
    //     file.write_all(&vec).unwrap();
    // }

    for path in output.files_loaded_during_snapshot {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

#[cfg(not(feature = "deno_core"))]
fn main() {}
