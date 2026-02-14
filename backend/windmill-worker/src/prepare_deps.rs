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

use crate::{BUN_CACHE_DIR, BUN_PATH, HOME_ENV, PATH_ENV, PROXY_ENVS, UV_CACHE_DIR};
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

#[derive(Deserialize)]
pub struct PrepareRequest {
    pub code: String,
    pub language: String,
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

/// Prepare Python dependencies using uv
async fn prepare_python_deps_standalone(code: &str) -> PrepareResponse {
    // Parse imports from the code
    let packages = parse_python_imports(code);

    if packages.is_empty() {
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: String::new(),
            success: true,
            error: None,
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
        };
    }

    let common_uv_envs = get_proc_envs(Some(("UV_CACHE_DIR", &UV_CACHE_DIR)));

    // Step 1: Create virtual environment using uv
    let output = Command::new(UV_PATH.as_str())
        .current_dir(&job_dir)
        .env_clear()
        .envs(common_uv_envs.clone())
        .args(["venv", &venv_dir, "--seed"])
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
        };
    }

    let out = output.unwrap();
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir: job_dir.clone(),
            success: false,
            error: Some(format!("uv venv failed: {}", stderr)),
        };
    }

    // Step 2: Install packages using uv pip install
    let python_path = format!("{}/bin/python", venv_dir);
    let mut args = vec!["pip", "install", "--python", &python_path];
    let package_refs: Vec<&str> = packages.iter().map(|s| s.as_str()).collect();
    args.extend(package_refs.iter());

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
                let stderr = String::from_utf8_lossy(&out.stderr);
                // Installation might fail for some packages (e.g., wrong package name)
                // Log the error but continue - the script might still work if the
                // package is actually installed elsewhere or the import is optional
                tracing::warn!("uv pip install warning: {}", stderr);
            }
        }
        Err(e) => {
            return PrepareResponse {
                node_modules_path: None,
                venv_path: None,
                job_dir: job_dir.clone(),
                success: false,
                error: Some(format!("Failed to run uv pip install: {}", e)),
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
    }
}

/// Get common environment variables for Bun processes
pub fn get_simple_bun_proc_envs() -> HashMap<String, String> {
    get_proc_envs(Some(("BUN_INSTALL_CACHE_DIR", &BUN_CACHE_DIR)))
}

/// Prepare dependencies for a script without requiring database access.
/// This is meant to be called from the CLI.
pub async fn prepare_deps_standalone(code: &str, language: &str) -> PrepareResponse {
    // Route to the appropriate handler based on language
    match language {
        "python3" | "python" => {
            return prepare_python_deps_standalone(code).await;
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
        };
    }

    let common_bun_proc_envs = get_simple_bun_proc_envs();

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
        };
    }

    // Step 2: Run bun install
    let output = Command::new(&*BUN_PATH)
        .current_dir(&job_dir)
        .env_clear()
        .envs(common_bun_proc_envs)
        .args(vec!["install", "--linker", "hoisted"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return PrepareResponse {
                    node_modules_path: None,
                    venv_path: None,
                    job_dir: job_dir.clone(),
                    success: false,
                    error: Some(format!("bun install failed: {}", stderr)),
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
        }
    } else {
        PrepareResponse {
            node_modules_path: None,
            venv_path: None,
            job_dir,
            success: true,
            error: None,
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
            };
            println!("{}", serde_json::to_string(&response)?);
            return Ok(());
        }
    };

    let response = prepare_deps_standalone(&request.code, &request.language).await;
    println!("{}", serde_json::to_string(&response)?);

    Ok(())
}
