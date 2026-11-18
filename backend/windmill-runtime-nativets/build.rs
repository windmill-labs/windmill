use deno_ast::{MediaType, ParseParams};
use deno_core::{Extension, ModuleCodeString, ModuleName, SourceMapData};
use deno_error::JsErrorBox;
use std::collections::HashSet;
use std::env;
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};

deno_core::extension!(
    fetch,
    esm_entry_point = "ext:fetch/src/runtime.js",
    esm = ["src/runtime.js"],
);

// `extension_transpiler` callback for `deno_core::snapshot::create_snapshot`,
// also applied to the residual lazy sources below.
//
// Specialized to our inputs: the deno_* extensions ship `.js`, except
// `deno_telemetry` whose lazy scripts are `.ts`. Our local `fetch` extension
// contributes `src/runtime.js`. No `node:` imports, no `.mjs`, no
// user-supplied modules. So:
//   - `.js` → pass through.
//   - `.ts` → transpile via deno_ast (deno_telemetry only).
//   - anything else → build bug (deno shipping an unexpected file type
//     or us mislabelling one), panic loudly rather than emit a broken
//     snapshot.
//
// No source maps: nothing consumes them.
//
// The signature returns `Result<_, JsErrorBox>` because that's what
// `extension_transpiler` expects, but we never construct one: parse and
// transpile failures are build-time bugs, so they panic.
//
// `deno_runtime::transpile::maybe_transpile_source` is the general version;
// depending on `deno_runtime` would pull a
// `deno_cache → rusqlite → libsqlite3-sys` chain that collides with
// sqlx-sqlite's `libsqlite3-sys` (cargo's `links = "sqlite3"` rule).
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
        text: source.as_str().into(),
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
            // The default appends an inline source map comment, which is a
            // syntax error where deno_core evaluates a lazy script as an
            // expression.
            &deno_ast::EmitOptions {
                source_map: deno_ast::SourceMapOption::None,
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("snapshot transpile: emit failed for {name}: {e}"))
        .into_source();

    Ok((transpiled.text.into(), None))
}

/// One `(specifier, source)` table of the extension scripts the snapshot did
/// not evaluate. The snapshot only carries scripts that ran while it was built,
/// so a script first loaded in a live isolate has to come from here.
///
/// deno_core binary-searches the table and reads each source as ASCII without
/// checking, so it must be sorted by specifier, free of duplicates, and ASCII.
fn residual_table(
    name: &str,
    files: impl Iterator<Item = deno_core::ExtensionFileSource>,
    consumed: &HashSet<String>,
) -> String {
    let mut files: Vec<_> = files.filter(|f| !consumed.contains(f.specifier)).collect();
    files.sort_by_key(|f| f.specifier);
    files.dedup_by_key(|f| f.specifier);

    let mut out = format!("pub static {name}: &[(&str, &str)] = &[\n");
    for file in files {
        let source = file
            .load()
            .unwrap_or_else(|e| panic!("could not load {}: {e}", file.specifier));
        let (code, _) =
            maybe_transpile_source(ModuleName::from_static(file.specifier), source).unwrap();
        assert!(
            code.as_str().is_ascii(),
            "{} is not ASCII after transpiling",
            file.specifier
        );
        writeln!(out, "    ({:?}, {:?}),", file.specifier, code.as_str()).unwrap();
    }
    out.push_str("];\n");
    out
}

fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());

    // Same list and order as `create_nativets_runtime`, which supplies their
    // state through `lazy_init_extensions`.
    let exts: Vec<Extension> = vec![
        deno_telemetry::deno_telemetry::lazy_init(),
        deno_webidl::deno_webidl::lazy_init(),
        deno_web::deno_web::lazy_init(),
        deno_crypto::deno_crypto::lazy_init(),
        deno_fetch::deno_fetch::lazy_init(),
        deno_net::deno_net::lazy_init(),
        fetch::init(),
    ];

    let lazy_js: Vec<_> = exts
        .iter()
        .flat_map(|e| e.lazy_loaded_js_files.iter().cloned())
        .collect();
    let lazy_esm: Vec<_> = exts
        .iter()
        .flat_map(|e| e.lazy_loaded_esm_files.iter().cloned())
        .collect();

    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());

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

    let mut file = std::fs::File::create(o.join("FETCH_SNAPSHOT.bin")).unwrap();
    file.write_all(&output.output).unwrap();

    let consumed: HashSet<String> = output.consumed_lazy_specifiers.into_iter().collect();
    let residual = format!(
        "{}{}",
        residual_table("RESIDUAL_LAZY_JS", lazy_js.into_iter(), &consumed),
        residual_table("RESIDUAL_LAZY_ESM", lazy_esm.into_iter(), &consumed),
    );
    std::fs::write(o.join("residual_lazy_sources.rs"), residual).unwrap();

    for path in output.files_loaded_during_snapshot {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
