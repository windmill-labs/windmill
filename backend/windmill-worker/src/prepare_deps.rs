/*
 * CLI command for preparing dependencies for the debugger.
 * This module provides a standalone dependency installation mechanism
 * that works without requiring a database connection.
 */

use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};
use std::process::Stdio;

use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::bun_executor::{write_bun_registry_config, BUN_CONFIG_FILE, BUN_NPMRC_FILE};
use crate::worker::non_empty_env;
use crate::{
    BUN_CACHE_DIR, BUN_PATH, HOME_ENV, INDEX_CERT, NATIVE_CERT, PATH_ENV, PROXY_ENVS, TRUSTED_HOST,
    UV_CACHE_DIR, UV_HTTP_TIMEOUT,
};
use windmill_common::worker::write_file;

const LOADER_BUILDER_CONTENT: &str = include_str!("../loader_builder.bun.js");

lazy_static::lazy_static! {
    /// Regex to parse Python import statements
    /// Matches: `import foo`, `import foo.bar`, `from foo import bar`, `from foo.bar import baz`
    static ref PYTHON_IMPORT_REGEX: Regex = Regex::new(
        r"(?m)^(?:from\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)\s+import|import\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*))"
    ).unwrap();

    /// Python standard library modules (Python 3.10+)
    /// This is a subset - most common ones that users might try to import
    static ref PYTHON_STDLIB: HashSet<&'static str> = {
        let mut s = HashSet::new();
        // Built-in modules
        s.insert("abc"); s.insert("aifc"); s.insert("argparse"); s.insert("array");
        s.insert("ast"); s.insert("asyncio"); s.insert("atexit"); s.insert("base64");
        s.insert("bdb"); s.insert("binascii"); s.insert("binhex"); s.insert("bisect");
        s.insert("builtins"); s.insert("bz2"); s.insert("calendar"); s.insert("cgi");
        s.insert("cgitb"); s.insert("chunk"); s.insert("cmath"); s.insert("cmd");
        s.insert("code"); s.insert("codecs"); s.insert("codeop"); s.insert("collections");
        s.insert("colorsys"); s.insert("compileall"); s.insert("concurrent");
        s.insert("configparser"); s.insert("contextlib"); s.insert("contextvars");
        s.insert("copy"); s.insert("copyreg"); s.insert("cProfile"); s.insert("crypt");
        s.insert("csv"); s.insert("ctypes"); s.insert("curses"); s.insert("dataclasses");
        s.insert("datetime"); s.insert("dbm"); s.insert("decimal"); s.insert("difflib");
        s.insert("dis"); s.insert("distutils"); s.insert("doctest"); s.insert("email");
        s.insert("encodings"); s.insert("enum"); s.insert("errno"); s.insert("faulthandler");
        s.insert("fcntl"); s.insert("filecmp"); s.insert("fileinput"); s.insert("fnmatch");
        s.insert("fractions"); s.insert("ftplib"); s.insert("functools"); s.insert("gc");
        s.insert("getopt"); s.insert("getpass"); s.insert("gettext"); s.insert("glob");
        s.insert("graphlib"); s.insert("grp"); s.insert("gzip"); s.insert("hashlib");
        s.insert("heapq"); s.insert("hmac"); s.insert("html"); s.insert("http");
        s.insert("idlelib"); s.insert("imaplib"); s.insert("imghdr"); s.insert("imp");
        s.insert("importlib"); s.insert("inspect"); s.insert("io"); s.insert("ipaddress");
        s.insert("itertools"); s.insert("json"); s.insert("keyword"); s.insert("lib2to3");
        s.insert("linecache"); s.insert("locale"); s.insert("logging"); s.insert("lzma");
        s.insert("mailbox"); s.insert("mailcap"); s.insert("marshal"); s.insert("math");
        s.insert("mimetypes"); s.insert("mmap"); s.insert("modulefinder"); s.insert("multiprocessing");
        s.insert("netrc"); s.insert("nis"); s.insert("nntplib"); s.insert("numbers");
        s.insert("operator"); s.insert("optparse"); s.insert("os"); s.insert("ossaudiodev");
        s.insert("pathlib"); s.insert("pdb"); s.insert("pickle"); s.insert("pickletools");
        s.insert("pipes"); s.insert("pkgutil"); s.insert("platform"); s.insert("plistlib");
        s.insert("poplib"); s.insert("posix"); s.insert("posixpath"); s.insert("pprint");
        s.insert("profile"); s.insert("pstats"); s.insert("pty"); s.insert("pwd");
        s.insert("py_compile"); s.insert("pyclbr"); s.insert("pydoc"); s.insert("queue");
        s.insert("quopri"); s.insert("random"); s.insert("re"); s.insert("readline");
        s.insert("reprlib"); s.insert("resource"); s.insert("rlcompleter"); s.insert("runpy");
        s.insert("sched"); s.insert("secrets"); s.insert("select"); s.insert("selectors");
        s.insert("shelve"); s.insert("shlex"); s.insert("shutil"); s.insert("signal");
        s.insert("site"); s.insert("smtpd"); s.insert("smtplib"); s.insert("sndhdr");
        s.insert("socket"); s.insert("socketserver"); s.insert("spwd"); s.insert("sqlite3");
        s.insert("ssl"); s.insert("stat"); s.insert("statistics"); s.insert("string");
        s.insert("stringprep"); s.insert("struct"); s.insert("subprocess"); s.insert("sunau");
        s.insert("symtable"); s.insert("sys"); s.insert("sysconfig"); s.insert("syslog");
        s.insert("tabnanny"); s.insert("tarfile"); s.insert("telnetlib"); s.insert("tempfile");
        s.insert("termios"); s.insert("test"); s.insert("textwrap"); s.insert("threading");
        s.insert("time"); s.insert("timeit"); s.insert("tkinter"); s.insert("token");
        s.insert("tokenize"); s.insert("tomllib"); s.insert("trace"); s.insert("traceback");
        s.insert("tracemalloc"); s.insert("tty"); s.insert("turtle"); s.insert("turtledemo");
        s.insert("types"); s.insert("typing"); s.insert("unicodedata"); s.insert("unittest");
        s.insert("urllib"); s.insert("uu"); s.insert("uuid"); s.insert("venv");
        s.insert("warnings"); s.insert("wave"); s.insert("weakref"); s.insert("webbrowser");
        s.insert("winreg"); s.insert("winsound"); s.insert("wsgiref"); s.insert("xdrlib");
        s.insert("xml"); s.insert("xmlrpc"); s.insert("zipapp"); s.insert("zipfile");
        s.insert("zipimport"); s.insert("zlib"); s.insert("zoneinfo");
        // Common aliases/shortcuts
        s.insert("_thread"); s.insert("__future__");
        s
    };

    /// UV binary path
    static ref UV_PATH: String = std::env::var("UV_PATH").unwrap_or_else(|_| "/usr/local/bin/uv".to_string());

    /// Fallbacks for the registry settings, used when the caller sends none: this process has
    /// no database, so the instance settings only reach it through the request (see
    /// [`RegistryConfig`]), and a debug service that cannot fetch them still configures the
    /// installer from its own environment.
    static ref PY_INDEX_URL: Option<String> = non_empty_env("PY_INDEX_URL").or_else(|| non_empty_env("PIP_INDEX_URL"));
    static ref PY_EXTRA_INDEX_URL: Option<String> = non_empty_env("PY_EXTRA_INDEX_URL").or_else(|| non_empty_env("PIP_EXTRA_INDEX_URL"));
    /// uv defaults to `first-index`; the job path overrides it so a package missing from the
    /// first index is still resolved from the others. Same default here.
    static ref PY_INDEX_STRATEGY: String = non_empty_env("UV_INDEX_STRATEGY").unwrap_or_else(|| "unsafe-best-match".to_string());
}

/// Simple loader that doesn't require Windmill API for relative imports
const SIMPLE_LOADER: &str = r#"
const p = {
  name: "simple-resolver",
  async setup(build) {
    // No-op plugin - we just want to scan imports
  },
};
"#;

/// The registry settings resolved from the instance settings by the caller.
///
/// This process runs without a database, so `GET /api/debug/registry_config` is where the
/// debug service reads them and this request field is how they get here. They are consumed
/// to configure the installer and never handed to the debug session itself: an index URL
/// embeds credentials and the session executes user-supplied code.
///
/// Field names are the `global_settings` keys, so the service forwards the endpoint's
/// response verbatim.
#[derive(Deserialize, Default)]
pub struct RegistryConfig {
    pub npm_config_registry: Option<String>,
    pub npmrc: Option<String>,
    pub bunfig_install_scopes: Option<String>,
    pub pip_index_url: Option<String>,
    pub pip_extra_index_url: Option<String>,
    pub uv_index_strategy: Option<String>,
}

#[derive(Deserialize)]
pub struct PrepareRequest {
    pub code: String,
    pub language: String,
    /// Interpreter the caller will run the script with. The venv must be built against it:
    /// site-packages is put on that interpreter's sys.path, and a wheel with a compiled
    /// extension built for another version is simply invisible there.
    #[serde(default)]
    pub python_path: Option<String>,
    #[serde(default)]
    pub registry: RegistryConfig,
}

/// A blank value means unset, as it does for the same setting on a worker.
fn configured(value: &Option<String>) -> Option<String> {
    value.clone().filter(|v| !v.trim().is_empty())
}

/// Where the registry configuration for one `bun install` is written.
///
/// Deliberately not the install directory: the debug session resolves its `node_modules`
/// symlink into that directory, and the sandbox bind-mounts all of `/tmp` into every session,
/// so a concurrent session could read the credentials of an install in flight. `/var/tmp` is a
/// tmpfs private to each jail (`debugger/nsjail.debug.config.proto`), which also takes the
/// credentials with it when a jailed install is killed. `bun install` reads them from here
/// through `--config` and `HOME`.
const REGISTRY_CONFIG_ROOT: &str = "/var/tmp/windmill-debug-registry";

/// How long a configuration directory may survive before the next install treats it as debris.
/// The caller kills an install with SIGKILL, leaving nothing able to clean up after it, and an
/// unjailed install is the only one that writes somewhere outliving the process at all. Well
/// past any install: the caller's own timeout is two minutes by default.
const REGISTRY_CONFIG_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(60 * 60);

/// Removes the registry configuration when the install ends, whichever way it ends.
struct RegistryConfigDir(Option<String>);

impl Drop for RegistryConfigDir {
    fn drop(&mut self) {
        if let Some(dir) = self.0.as_deref() {
            remove_registry_config_dir(dir);
        }
    }
}

/// Write the registry configuration for one install and return the directory holding it, empty
/// when there is nothing to write.
///
/// Falls back to the install directory if the private root is not writable: an install that
/// reaches its registry matters more than the isolation above, which only holds for sessions
/// that run under nsjail in the first place.
fn write_registry_config_dir(
    job_id: &uuid::Uuid,
    job_dir: &str,
    registry: &RegistryConfig,
) -> anyhow::Result<RegistryConfigDir> {
    let npmrc = configured(&registry.npmrc);
    let npm_config_registry = configured(&registry.npm_config_registry);
    let bunfig_install_scopes = configured(&registry.bunfig_install_scopes);
    if npmrc.is_none() && npm_config_registry.is_none() && bunfig_install_scopes.is_none() {
        return Ok(RegistryConfigDir(None));
    }

    let dir = format!("{}/{}", REGISTRY_CONFIG_ROOT, job_id);
    let dir = match create_private_dir(&dir) {
        Ok(()) => dir,
        Err(e) => {
            tracing::warn!("Could not create {dir} ({e}), keeping the registry configuration in the install directory");
            job_dir.to_string()
        }
    };
    // Claimed before anything is written to it, so a failure below still takes it down.
    let held = RegistryConfigDir(Some(dir.clone()));
    write_bun_registry_config(&dir, npmrc, npm_config_registry, bunfig_install_scopes)?;
    Ok(held)
}

fn create_private_dir(dir: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::create_dir_all(REGISTRY_CONFIG_ROOT)?;
        sweep_stale_registry_config();
        std::fs::DirBuilder::new().mode(0o700).create(dir)
    }
    #[cfg(not(unix))]
    {
        sweep_stale_registry_config();
        std::fs::create_dir_all(dir)
    }
}

/// Drop what an install that was killed could not clean up itself.
fn sweep_stale_registry_config() {
    let Ok(entries) = std::fs::read_dir(REGISTRY_CONFIG_ROOT) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|m| m.elapsed().is_ok_and(|age| age > REGISTRY_CONFIG_MAX_AGE));
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Delete the registry configuration once the install that needed it is over. A failure to
/// remove credentials has to be visible.
fn remove_registry_config_dir(dir: &str) {
    if dir.starts_with(REGISTRY_CONFIG_ROOT) {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::error!("Failed to remove registry configuration {dir}: {e}");
            }
        }
        return;
    }
    // The fallback path above put them in the install directory, which has to survive.
    for file in [BUN_NPMRC_FILE, BUN_CONFIG_FILE] {
        let path = format!("{}/{}", dir, file);
        if let Err(e) = std::fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::error!("Failed to remove registry configuration {path}: {e}");
            }
        }
    }
}

#[derive(Serialize)]
pub struct PrepareResponse {
    /// Path to node_modules for JS/TS scripts
    pub node_modules_path: Option<String>,
    /// Path to Python virtual environment's site-packages
    pub venv_path: Option<String>,
    pub job_dir: String,
    pub success: bool,
    pub error: Option<String>,
    /// Raw stderr of the dependency installer when it exited non-zero, so a caller can show the
    /// registry/TLS failure verbatim instead of the bare ModuleNotFoundError that follows.
    /// Omitted from the JSON when absent, so callers that only know `success`/`error` are unaffected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_stderr: Option<String>,
}

/// Parse Python imports and return a list of package names that need to be installed.
/// Filters out standard library modules.
fn parse_python_imports(code: &str) -> Vec<String> {
    let mut packages = HashSet::new();

    for cap in PYTHON_IMPORT_REGEX.captures_iter(code) {
        // Get either group 1 (from X import) or group 2 (import X)
        let module = cap.get(1).or_else(|| cap.get(2));
        if let Some(m) = module {
            let full_module = m.as_str();
            // Get the top-level package name (e.g., "foo" from "foo.bar.baz")
            let package = full_module.split('.').next().unwrap_or(full_module);

            // Skip standard library modules
            if !PYTHON_STDLIB.contains(package) {
                // Skip relative imports (starting with .)
                if !package.starts_with('.') {
                    packages.insert(package.to_string());
                }
            }
        }
    }

    packages.into_iter().collect()
}

/// Get common environment variables for external processes (UV, Bun, etc.)
fn get_proc_envs(cache_env: Option<(&str, &str)>) -> HashMap<String, String> {
    let mut envs = HashMap::new();
    envs.insert("PATH".to_string(), PATH_ENV.to_string());
    envs.insert("HOME".to_string(), HOME_ENV.to_string());

    if let Some((key, value)) = cache_env {
        envs.insert(key.to_string(), value.to_string());
    }

    // Add proxy envs
    for (k, v) in PROXY_ENVS.iter() {
        envs.insert(k.to_string(), v.clone());
    }

    envs
}

/// CA bundle for the package index, most specific spelling first. A host behind a TLS-intercepting
/// proxy configures it under whichever name its other tooling uses, and the environment is cleared
/// below, so falling back past `PY_INDEX_CERT` is what makes those hosts work at all.
fn index_ca_bundle() -> Option<String> {
    INDEX_CERT
        .clone()
        .or_else(|| non_empty_env("SSL_CERT_FILE"))
        .or_else(|| non_empty_env("REQUESTS_CA_BUNDLE"))
        .or_else(|| non_empty_env("CURL_CA_BUNDLE"))
}

/// uv registry arguments, mirroring what the job path passes in `python_executor`.
fn uv_registry_args(registry: &RegistryConfig) -> Vec<String> {
    let index_url = configured(&registry.pip_index_url).or_else(|| PY_INDEX_URL.clone());
    let extra_index_url =
        configured(&registry.pip_extra_index_url).or_else(|| PY_EXTRA_INDEX_URL.clone());

    let mut args: Vec<String> = vec![];
    if let Some(urls) = extra_index_url.as_ref() {
        for url in urls.split(',') {
            args.extend(["--extra-index-url".to_string(), url.to_string()]);
        }
    }
    if let Some(url) = index_url.as_ref() {
        args.extend(["--index-url".to_string(), url.to_string()]);
    }
    if let Some(hosts) = TRUSTED_HOST.as_ref() {
        for host in hosts.split_whitespace() {
            args.extend(["--trusted-host".to_string(), host.to_string()]);
        }
    }
    if *NATIVE_CERT {
        args.push("--native-tls".to_string());
    }
    args
}

/// Prepare Python dependencies using uv
async fn prepare_python_deps_standalone(
    code: &str,
    python_path: Option<&str>,
    registry: &RegistryConfig,
) -> PrepareResponse {
    // Parse imports from the code
    let packages = parse_python_imports(code);

    if packages.is_empty() {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: String::new(),
            success: true,
            error: None,
            install_stderr: None,
        };
    }

    tracing::debug!("Detected Python packages: {:?}", packages);

    // Create a temporary directory for the virtual environment
    let job_id = uuid::Uuid::new_v4();
    let job_dir = format!("/tmp/windmill-deps/{}", job_id);
    let venv_dir = format!("{}/venv", job_dir);

    if let Err(e) = std::fs::create_dir_all(&job_dir) {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("Failed to create job directory: {}", e)),
            install_stderr: None,
        };
    }

    let mut common_uv_envs = get_proc_envs(Some(("UV_CACHE_DIR", &UV_CACHE_DIR)));
    common_uv_envs.insert(
        "UV_INDEX_STRATEGY".to_string(),
        configured(&registry.uv_index_strategy).unwrap_or_else(|| PY_INDEX_STRATEGY.to_string()),
    );
    if let Some(timeout) = UV_HTTP_TIMEOUT.as_ref() {
        common_uv_envs.insert("UV_HTTP_TIMEOUT".to_string(), timeout.to_string());
    }
    if let Some(cert_path) = index_ca_bundle() {
        // uv has no `--cert` on `venv`/`pip install` (astral-sh/uv#6715), so a custom CA bundle
        // reaches it through SSL_CERT_FILE, as in the job path. It replaces uv's own roots rather
        // than adding to them, so the file has to be a complete bundle.
        common_uv_envs.insert("SSL_CERT_FILE".to_string(), cert_path);
    }
    // The other spelling uv accepts. Like the bundle above it replaces uv's own roots rather than
    // adding to them, so a directory holding only a private CA leaves public indexes untrusted.
    if let Some(cert_dir) = non_empty_env("SSL_CERT_DIR") {
        common_uv_envs.insert("SSL_CERT_DIR".to_string(), cert_dir);
    }

    let registry_args = uv_registry_args(registry);

    // Step 1: Create virtual environment using uv
    // `--seed` resolves pip/setuptools from the index, so the venv also needs the registry
    // arguments: on a network that only reaches a private mirror it fails without them.
    let mut venv_args = vec!["venv".to_string(), venv_dir.clone(), "--seed".to_string()];
    // Without this uv picks its own interpreter, and the caller then puts a site-packages built
    // for that version on a different interpreter's sys.path: pure-Python packages still import,
    // anything with a compiled extension does not, and the error names the missing extension
    // rather than the mismatch.
    if let Some(python_path) = python_path {
        venv_args.extend(["-p".to_string(), python_path.to_string()]);
    }
    venv_args.extend(registry_args.iter().cloned());

    let output = Command::new(UV_PATH.as_str())
        .current_dir(&job_dir)
        .env_clear()
        .envs(common_uv_envs.clone())
        .args(&venv_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    if let Err(e) = output {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("Failed to create venv: {}", e)),
            install_stderr: None,
        };
    }

    let out = output.unwrap();
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("uv venv failed: {}", stderr)),
            install_stderr: Some(stderr),
        };
    }

    // Step 2: Install packages using uv pip install
    let python_path = format!("{}/bin/python", venv_dir);
    let mut args = vec![
        "pip".to_string(),
        "install".to_string(),
        "--python".to_string(),
        python_path,
    ];
    args.extend(packages.iter().cloned());
    args.extend(registry_args);

    let output = Command::new(UV_PATH.as_str())
        .current_dir(&job_dir)
        .env_clear()
        .envs(common_uv_envs)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                // uv installs the whole set atomically, so a failure here means an empty venv:
                // returning success would leave the caller with a bare ModuleNotFoundError and
                // no way to see the registry/TLS/package-name error that caused it.
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                tracing::warn!("uv pip install failed: {}", stderr);
                return PrepareResponse {
                    node_modules_path: None,
                    venv_path: None,
                    job_dir: job_dir.clone(),
                    success: false,
                    error: Some(format!("uv pip install failed: {}", stderr)),
                    install_stderr: Some(stderr),
                };
            }
        }
        Err(e) => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: false,
                error: Some(format!("Failed to run uv pip install: {}", e)),
                install_stderr: None,
            };
        }
    }

    // Find the site-packages directory
    let site_packages = format!("{}/lib", venv_dir);
    let site_packages_path = if let Ok(entries) = std::fs::read_dir(&site_packages) {
        // Find python3.X directory
        let python_dir = entries
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("python"));

        if let Some(py_dir) = python_dir {
            let sp = format!("{}/site-packages", py_dir.path().display());
            if std::path::Path::new(&sp).exists() {
                Some(sp)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    PrepareResponse {
        node_modules_path: None,
        venv_path: site_packages_path.or(Some(venv_dir)),
        job_dir,
        success: true,
        error: None,
        install_stderr: None,
    }
}

/// Get common environment variables for Bun processes
pub fn get_simple_bun_proc_envs() -> HashMap<String, String> {
    let mut envs = get_proc_envs(Some(("BUN_INSTALL_CACHE_DIR", &BUN_CACHE_DIR)));
    // Bun reads none of the spellings uv does, so a custom CA reaches `bun install` only here.
    if let Some(cert_path) = non_empty_env("NODE_EXTRA_CA_CERTS").or_else(index_ca_bundle) {
        envs.insert("NODE_EXTRA_CA_CERTS".to_string(), cert_path);
    }
    envs
}

/// Prepare dependencies for a script without requiring database access.
/// This is meant to be called from the CLI.
pub async fn prepare_deps_standalone(
    code: &str,
    language: &str,
    python_path: Option<&str>,
    registry: &RegistryConfig,
) -> PrepareResponse {
    // Route to the appropriate handler based on language
    match language {
        "python3" | "python" => {
            return prepare_python_deps_standalone(code, python_path, registry).await;
        }
        "bun" | "typescript" | "deno" => {
            // Continue with JS/TS handling below
        }
        _ => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: String::new(),
                success: false,
                error: Some(format!(
                    "Unsupported language for dependency preparation: {}",
                    language
                )),
                install_stderr: None,
            };
        }
    }

    // Create a temporary directory for the job
    let job_id = uuid::Uuid::new_v4();
    let job_dir = format!("/tmp/windmill-deps/{}", job_id);

    if let Err(e) = std::fs::create_dir_all(&job_dir) {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("Failed to create job directory: {}", e)),
            install_stderr: None,
        };
    }

    // Write the script code
    if let Err(e) = write_file(&job_dir, "main.ts", code) {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("Failed to write main.ts: {}", e)),
            install_stderr: None,
        };
    }

    // Write the build.js script that scans imports and generates package.json
    let build_script = format!(
        r#"{}

{}
"#,
        SIMPLE_LOADER, LOADER_BUILDER_CONTENT
    );

    if let Err(e) = write_file(&job_dir, "build.js", &build_script) {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("Failed to write build.js: {}", e)),
            install_stderr: None,
        };
    }

    let mut common_bun_proc_envs = get_simple_bun_proc_envs();

    // Step 1: Run build.js to generate package.json
    let output = Command::new(&*BUN_PATH)
        .current_dir(&job_dir)
        .env_clear()
        .envs(common_bun_proc_envs.clone())
        .args(vec!["run", "build.js"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                // If build fails, it might be because there are no external imports
                // Check if package.json was created anyway
                if !std::path::Path::new(&format!("{}/package.json", job_dir)).exists() {
                    // Create an empty package.json
                    let empty_pkg = r#"{"dependencies": {}}"#;
                    if let Err(e) = write_file(&job_dir, "package.json", empty_pkg) {
                        return PrepareResponse {
                            node_modules_path: None,
                            venv_path: None,
                            job_dir: job_dir.clone(),
                            success: false,
                            error: Some(format!("Failed to write empty package.json: {}", e)),
                            install_stderr: None,
                        };
                    }
                }
                tracing::debug!("Build script stderr (may be non-fatal): {}", stderr);
            }
        }
        Err(e) => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: false,
                error: Some(format!("Failed to run build.js: {}", e)),
                install_stderr: None,
            };
        }
    }

    // Check if package.json has any dependencies
    let package_json_path = format!("{}/package.json", job_dir);
    let package_json_content = match std::fs::read_to_string(&package_json_path) {
        Ok(content) => content,
        Err(_) => {
            // No package.json means no dependencies needed
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: true,
                error: None,
                install_stderr: None,
            };
        }
    };

    // Parse to check if dependencies is empty
    let package_json: serde_json::Value = match serde_json::from_str(&package_json_content) {
        Ok(v) => v,
        Err(e) => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: false,
                error: Some(format!("Failed to parse package.json: {}", e)),
                install_stderr: None,
            };
        }
    };

    let deps = package_json.get("dependencies").and_then(|d| d.as_object());
    if deps.map(|d| d.is_empty()).unwrap_or(true) {
        // No dependencies to install
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: true,
            error: None,
            install_stderr: None,
        };
    }

    // Step 2: Run bun install, from the same registry configuration a job installs with.
    let mut args = vec!["install".to_string()];
    let registry_config_dir = match write_registry_config_dir(&job_id, &job_dir, registry) {
        Ok(dir) => dir,
        Err(e) => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: false,
                error: Some(format!("Failed to write registry configuration: {}", e)),
                install_stderr: None,
            };
        }
    };
    if let Some(dir) = registry_config_dir.0.as_ref() {
        // Only one of the two files exists: `.npmrc` is read from the installer's home,
        // `bunfig.toml` only from the path named here.
        common_bun_proc_envs.insert("HOME".to_string(), dir.to_string());
        let bunfig = format!("{}/{}", dir, BUN_CONFIG_FILE);
        if std::path::Path::new(&bunfig).exists() {
            args.push(format!("--config={}", bunfig));
        }
    }

    let output = Command::new(&*BUN_PATH)
        .current_dir(&job_dir)
        .env_clear()
        .envs(common_bun_proc_envs)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    drop(registry_config_dir);

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                return PrepareResponse {
                    node_modules_path: None,
                    venv_path: None,
                    job_dir: job_dir.clone(),
                    success: false,
                    error: Some(format!("bun install failed: {}", stderr)),
                    install_stderr: Some(stderr),
                };
            }
        }
        Err(e) => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: false,
                error: Some(format!("Failed to run bun install: {}", e)),
                install_stderr: None,
            };
        }
    }

    let node_modules_path = format!("{}/node_modules", job_dir);
    if std::path::Path::new(&node_modules_path).exists() {
        PrepareResponse {
            node_modules_path: Some(node_modules_path),
            venv_path: None,
            job_dir,
            success: true,
            error: None,
            install_stderr: None,
        }
    } else {
        PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir,
            success: true,
            error: None,
            install_stderr: None,
        }
    }
}

/// CLI entry point for prepare-deps command
pub async fn run_prepare_deps_cli() -> anyhow::Result<()> {
    // Read JSON from stdin
    let stdin = io::stdin();
    let mut input = String::new();

    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                input.push_str(&l);
                input.push('\n');
            }
            Err(e) => {
                let response = PrepareResponse {
                    node_modules_path: None,
                    venv_path: None,
                    job_dir: String::new(),
                    success: false,
                    error: Some(format!("Failed to read stdin: {}", e)),
                    install_stderr: None,
                };
                println!("{}", serde_json::to_string(&response)?);
                return Ok(());
            }
        }
    }

    let request: PrepareRequest = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let response = PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: String::new(),
                success: false,
                error: Some(format!(
                    "Failed to parse JSON input: {}. Expected {{\"code\": \"...\", \"language\": \"bun\" or \"python3\"}}",
                    e
                )),
                install_stderr: None,
            };
            println!("{}", serde_json::to_string(&response)?);
            return Ok(());
        }
    };

    let response = prepare_deps_standalone(
        &request.code,
        &request.language,
        request.python_path.as_deref(),
        &request.registry,
    )
    .await;
    println!("{}", serde_json::to_string(&response)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PrepareRequest, PrepareResponse};

    /// The debug service and this CLI are deployed as separate images, and the service
    /// forwards `GET /api/debug/registry_config` verbatim, `message` field included. So a
    /// request from an older service carries no `registry` at all, and one from a newer
    /// service carries fields this binary does not know.
    #[test]
    fn test_registry_is_optional_and_tolerates_unknown_fields() {
        let without: PrepareRequest =
            serde_json::from_str(r#"{"code": "import lodash", "language": "bun"}"#).unwrap();
        assert!(without.registry.npm_config_registry.is_none());

        let with: PrepareRequest = serde_json::from_str(
            r#"{"code": "", "language": "bun", "registry": {"npm_config_registry": "https://npm.example", "message": "for the user"}}"#,
        )
        .unwrap();
        assert_eq!(
            with.registry.npm_config_registry.as_deref(),
            Some("https://npm.example")
        );
    }

    /// The debugger (`debugger/dap_websocket_server.py`) parses this JSON out of the CLI's
    /// stdout, so `install_stderr` has to stay additive: a response without an install failure
    /// must serialize to the shape callers already know.
    #[test]
    fn test_install_stderr_is_additive() {
        let ok = PrepareResponse {
            node_modules_path: None,
            venv_path: Some("/tmp/windmill-deps/x/venv".to_string()),
            job_dir: "/tmp/windmill-deps/x".to_string(),
            success: true,
            error: None,
            install_stderr: None,
        };
        assert_eq!(
            serde_json::to_value(&ok).unwrap(),
            serde_json::json!({
                "node_modules_path": null,
                "venv_path": "/tmp/windmill-deps/x/venv",
                "job_dir": "/tmp/windmill-deps/x",
                "success": true,
                "error": null,
            })
        );

        let failed = PrepareResponse {
            install_stderr: Some("error: no such package".to_string()),
            success: false,
            error: Some("uv pip install failed: error: no such package".to_string()),
            ..ok
        };
        let failed = serde_json::to_value(&failed).unwrap();
        assert_eq!(failed["install_stderr"], "error: no such package");
        assert_eq!(failed["success"], false);
    }
}
