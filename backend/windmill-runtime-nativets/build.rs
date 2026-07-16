use deno_ast::{MediaType, ParseParams};
use deno_core::{ModuleCodeString, ModuleName, SourceMapData};
use deno_error::JsErrorBox;
use deno_fetch::FetchPermissions;
use deno_net::NetPermissions;
use deno_web::{BlobStore, TimersPermission};
use std::borrow::Cow;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct PermissionsContainer;

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
        _path: Cow<'a, Path>,
        _api_name: &str,
        _get_path: &'a dyn deno_fs::GetPath,
    ) -> Result<deno_fs::CheckedPath<'a>, deno_io::fs::FsError> {
        unreachable!("snapshotting")
    }

    #[inline(always)]
    fn check_write<'a>(
        &mut self,
        _path: Cow<'a, Path>,
        _api_name: &str,
        _get_path: &'a dyn deno_fs::GetPath,
    ) -> Result<deno_fs::CheckedPath<'a>, deno_io::fs::FsError> {
        unreachable!("snapshotting")
    }

    #[inline(always)]
    fn check_net_vsock(
        &mut self,
        _cid: u32,
        _port: u32,
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }
}

impl TimersPermission for PermissionsContainer {
    #[inline(always)]
    fn allow_hrtime(&mut self) -> bool {
        true
    }
}

impl NetPermissions for PermissionsContainer {
    fn check_read(
        &mut self,
        _p: &str,
        _api_name: &str,
    ) -> Result<PathBuf, deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }

    fn check_write(
        &mut self,
        _p: &str,
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
        _p: Cow<'a, Path>,
        _api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }

    fn check_vsock(
        &mut self,
        _cid: u32,
        _port: u32,
        _api_name: &str,
    ) -> Result<(), deno_permissions::PermissionCheckError> {
        unreachable!("snapshotting")
    }
}

deno_core::extension!(
    fetch,
    esm_entry_point = "ext:fetch/src/runtime.js",
    esm = ["src/runtime.js"],
);

// `extension_transpiler` callback for `deno_core::snapshot::create_snapshot`.
//
// Specialized to our snapshot's inputs. Of the eight deno_* extensions
// we register via `init()`, seven ship pre-built `.js` files
// in their `esm` lists (webidl/url/console/web/crypto/fetch/net) — only
// `deno_telemetry`'s `extension!` macro lists `.ts` files
// (`telemetry.ts`, `util.ts`), so the TypeScript branch is needed
// solely for that crate. Our local `fetch` extension contributes
// `src/runtime.js` (pure JS). No `node:` imports happen at snapshot
// build time, no `.mjs`, no user-supplied modules. So:
//   - `.js` → pass through.
//   - `.ts` → transpile via deno_ast (deno_telemetry only).
//   - anything else → build bug (deno shipping an unexpected file type
//     or us mislabelling one), panic loudly rather than emit a broken
//     snapshot.
//
// No source maps: the snapshot is a binary blob the runtime loads — source
// maps would never be consumed.
//
// The signature still returns `Result<_, JsErrorBox>` because that's what
// `extension_transpiler` expects, but we never construct one — parse and
// transpile failures are build-time bugs in deno's own .ts internals (or
// in our runtime.js, if we ever change its extension), so they panic.
//
// This replaces a call to `deno_runtime::transpile::maybe_transpile_source`
// from `deno_runtime 0.198.0`. The original is more general (handles
// `node:` modules, `.mjs`, emits source maps in debug builds, plumbs
// errors via `JsErrorBox`); none of that surface is reachable in our
// build. Dropping the `deno_runtime` dep eliminates a
// `deno_cache → rusqlite → libsqlite3-sys 0.35` transitive chain that
// collides with sqlx-sqlite's `libsqlite3-sys 0.30` (cargo's
// `links = "sqlite3"` rule).
fn maybe_transpile_source(
    name: ModuleName,
    source: ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), JsErrorBox> {
    let media_type = MediaType::from_path(Path::new(&name));
    match media_type {
        MediaType::JavaScript => return Ok((source, None)),
        MediaType::TypeScript => {}
        _ => panic!("unexpected media type {media_type:?} for {name} during snapshot build"),
    }

    let parsed = deno_ast::parse_module(ParseParams {
        specifier: deno_core::url::Url::parse(&name).unwrap(),
        text: source.into(),
        media_type,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })
    .unwrap_or_else(|e| panic!("snapshot transpile: parse failed for {name}: {e}"));

    let transpiled = parsed
        .transpile(
            &deno_ast::TranspileOptions {
                imports_not_used_as_values: deno_ast::ImportsNotUsedAsValues::Remove,
                ..Default::default()
            },
            &deno_ast::TranspileModuleOptions::default(),
            &deno_ast::EmitOptions::default(),
        )
        .unwrap_or_else(|e| panic!("snapshot transpile: emit failed for {name}: {e}"))
        .into_source();

    Ok((transpiled.text.into(), None))
}

fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());

    let exts = vec![
        deno_telemetry::deno_telemetry::init(),
        deno_webidl::deno_webidl::init(),
        deno_url::deno_url::init(),
        deno_console::deno_console::init(),
        deno_web::deno_web::init::<PermissionsContainer>(Arc::new(BlobStore::default()), None),
        deno_crypto::deno_crypto::init(None),
        deno_fetch::deno_fetch::init::<PermissionsContainer>(Default::default()),
        deno_net::deno_net::init::<PermissionsContainer>(None, None),
        fetch::init(),
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
                maybe_transpile_source(specifier, source)
            })),
            extensions: exts,
            with_runtime_cb: None,
            skip_op_registration: false,
        },
        None,
    )
    .unwrap();

    let mut file = std::fs::File::create(snapshot_path).unwrap();
    file.write_all(&output.output).unwrap();

    for path in output.files_loaded_during_snapshot {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
