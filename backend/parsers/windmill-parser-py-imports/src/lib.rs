/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

mod mapping;

use anyhow::anyhow;
use async_recursion::async_recursion;
use itertools::Itertools;
use lazy_static::lazy_static;
use pep440_rs::{Version, VersionSpecifier};
use std::{collections::HashMap, str::FromStr};

use mapping::{FULL_IMPORTS_MAP, SHORT_IMPORTS_MAP};
#[cfg(not(target_arch = "wasm32"))]
use regex::Regex;
#[cfg(target_arch = "wasm32")]
use regex_lite::Regex;

use rustpython_parser::{
    ast::{Stmt, StmtImport, StmtImportFrom, Suite},
    text_size::TextRange,
    Parse,
};
use sqlx::{Pool, Postgres};
use windmill_common::{
    error::{self, to_anyhow},
    worker::PythonAnnotations,
};

const DEF_MAIN: &str = "def main(";

fn replace_import(x: String) -> String {
    SHORT_IMPORTS_MAP
        .get(&x)
        .map(|x| x.to_owned())
        .unwrap_or(&x)
        .to_string()
}

fn replace_full_import(x: &str) -> Option<String> {
    FULL_IMPORTS_MAP.get(x).map(|x| (*x).to_owned())
}

lazy_static! {
    static ref RE: Regex = Regex::new(r"^\#\s?(\S+)\s*$").unwrap();
    static ref PIN_RE: Regex = Regex::new(r"(?:\s*#\s*(pin|repin):\s*)(\S*)").unwrap();
    static ref PKG_RE: Regex = Regex::new(r"^([^!=<>]+)(?:[!=<>]|$)").unwrap();
}

fn process_import(module: Option<String>, path: &str, level: usize) -> Vec<NImport> {
    if level > 0 {
        let mut imports = vec![];
        let splitted_path = path.split("/");
        let base = splitted_path
            .clone()
            .take(splitted_path.count() - level)
            .join("/");
        if let Some(m) = module {
            imports.push(NImport::Relative(format!("{base}/{}", m.replace(".", "/"))));
        } else {
            imports.push(NImport::Relative(format!("{base}")));
        }
        imports
    } else if let Some(module) = module {
        let imprt = module.split('.').next().unwrap_or("").replace("_", "-");
        if imprt == "u" || imprt == "f" {
            vec![NImport::Relative(module.replace(".", "/"))]
        } else {
            let pkg = replace_full_import(&module).unwrap_or(replace_import(imprt));
            vec![NImport::Auto { key: if module == pkg { None } else { Some(module) }, pkg }]
        }
    } else {
        vec![]
    }
}

pub fn parse_relative_imports(code: &str, path: &str) -> error::Result<Vec<String>> {
    let nimports = parse_code_for_imports(code, path)?;
    return Ok(nimports
        .into_iter()
        .filter_map(|x| match x {
            NImport::Relative(path) => Some(path),
            _ => None,
        })
        .collect());
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum NImport {
    // Order matters! First we want to resolve all repins

    // manually repinned requirement
    // e.g.:
    // import pandas # repin: pandas==x.y.z
    Repin {
        pin: ImportPin,
        key: String,
    },
    // manually pinned requirements
    // e.g.:
    // import pandas # pin: pandas>=x.y.z
    // import pandas # pin: pandas<=x.y.z
    //
    // NOTE: It is possible for multiple pins exist on same import
    // That's why we store vector of pins
    Pin {
        pins: Vec<ImportPin>,
        key: String,
    },
    // Automatically inferred requirement
    // e.g.:
    // import pandas
    Auto {
        // Take `x.y.z` for example
        // x is going to be the `root`
        // and x.y.z is `full`
        //
        // `full` will be None if it is equal to root
        //
        // We will use `root` as a requirement name and pass to `uv pip compile` if it was not replaced with any pin
        pkg: String,

        // However we still need full, since all pins pin against full import names
        key: Option<String>,
    },
    // Relative imports
    Relative(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum NImportResolved {
    Repin { pin: ImportPin, key: String },
    Pin { pins: Vec<ImportPin>, key: String },
    Auto { pkg: String, key: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ImportPin {
    pkg: String,
    path: String,
}

fn parse_code_for_imports(code: &str, path: &str) -> error::Result<Vec<NImport>> {
    let mut code = code.split(DEF_MAIN).next().unwrap_or("").to_string();

    // remove main function decorator from end of file if it exists
    if code
        .lines()
        .last()
        .map(|x| x.starts_with("@"))
        .unwrap_or(false)
    {
        code = code
            .lines()
            .take(code.lines().count() - 1)
            .collect::<Vec<&str>>()
            .join("\n")
            + "\n";
    }

    let ast = Suite::parse(&code, "main.py").map_err(|e| {
        error::Error::ExecutionErr(format!("Error parsing code for imports: {}", e.to_string()))
    })?;

    let find_pin = |range: TextRange, key: String| {
        let hs = code
            .chars()
            .skip(range.end().to_usize())
            .take_while(|e| *e != '\n')
            .collect::<String>();

        if hs.trim_start().is_empty() {
            return None;
        }

        PIN_RE.captures(&hs).and_then(|x| {
            x.get(1).zip(x.get(2)).and_then(|(ty_m, pkg_m)| {
                let pkg = pkg_m.as_str().to_owned();
                if ty_m.as_str() == "pin" {
                    Some(vec![NImport::Pin {
                        pins: vec![ImportPin { pkg, path: path.to_owned() }],
                        key,
                    }])
                } else if ty_m.as_str() == "repin" {
                    Some(vec![NImport::Repin {
                        pin: ImportPin { pkg, path: path.to_owned() },
                        key,
                    }])
                } else {
                    None
                }
            })
        })
    };

    let mut nimports: Vec<NImport> = ast
        .into_iter()
        .filter_map(|x| match x {
            Stmt::Import(StmtImport { names, range }) => names
                .get(0)
                .and_then(|al| find_pin(range, al.name.to_string()))
                .or(Some(
                    names
                        .into_iter()
                        .map(|x| {
                            let name = x.name.to_string();
                            process_import(Some(name), path, 0)
                        })
                        .flatten()
                        .collect::<Vec<NImport>>(),
                )),
            Stmt::ImportFrom(StmtImportFrom { level: Some(i), module, .. }) if i.to_u32() > 0 => {
                Some(process_import(
                    module.map(|x| x.to_string()),
                    path,
                    i.to_usize(),
                ))
            }
            Stmt::ImportFrom(StmtImportFrom { level: _, module, range, .. }) => find_pin(
                range,
                module.clone().map(|x| x.to_string()).unwrap_or_default(),
            )
            .or(Some(process_import(module.map(|x| x.to_string()), path, 0))),
            _ => None,
        })
        .flatten()
        .filter(|x| {
            if let NImport::Auto { ref pkg, .. } = x {
                !STDIMPORTS.contains(&(*pkg).as_str())
            } else {
                true
            }
        })
        .unique()
        .collect();

    nimports.sort();
    return Ok(nimports);
}

pub async fn parse_python_imports(
    code: &str,
    w_id: &str,
    path: &str,
    db: &Pool<Postgres>,
    version_specifiers: &mut Vec<VersionSpecifier>,
) -> error::Result<(Vec<String>, Option<String>)> {
    let mut compile_error_hint: Option<String> = None;
    let mut imports = parse_python_imports_inner(
        code,
        w_id,
        path,
        db,
        &mut vec![],
        version_specifiers,
        // &mut version_specifier.and_then(|_| Some(path.to_owned())),
        &mut None
    )
    .await?
    .into_values()
    .map(|nimport| match nimport {
        NImportResolved::Pin { pins, .. } => pins.into_iter().map(|p| {
            if let Some(hint) = &mut compile_error_hint{
                    hint.push_str(&format!("\n - pin to {} in {}", p.pkg, p.path));
            } else {
                compile_error_hint = Some("\n\nMultiple pins can cause problems during lockfile resolution.\nMake sure you checked every pin for conflicts:\n".into())
            };
            Ok(p.pkg)
        }).collect_vec(),
        NImportResolved::Repin { pin: ImportPin { pkg, .. }, .. } => vec![Ok(pkg)],
        NImportResolved::Auto { pkg, key } => vec![

            if let Some(key) = key {
               Ok(format!("{pkg} # (mapped from {key})"))
            } else {
               Ok(pkg)
            }
        ],
    })
    .flatten()
    .collect::<error::Result<Vec<String>>>()?
    .into_iter()
    .filter(|x| !x.trim_start().starts_with("--") && !x.trim().is_empty())
    .unique()
    .collect_vec();

    imports.sort();

    compile_error_hint
        .as_mut()
        .map(|e| e.push_str("\n\nNOTE: You can also `repin` to override all pins"));
    Ok((imports, compile_error_hint))
}

fn extract_pkg_name(requirement: &str) -> String {
    PKG_RE
        .captures(requirement)
        .map(|x| x.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())
        .unwrap_or_default()
}

#[async_recursion]
async fn parse_python_imports_inner(
    code: &str,
    w_id: &str,
    path: &str,
    db: &Pool<Postgres>,
    already_visited: &mut Vec<String>,
    version_specifiers: &mut Vec<VersionSpecifier>,
    path_where_annotated_pyv: &mut Option<String>,
) -> error::Result<HashMap<String, NImportResolved>> {
    let PythonAnnotations { py310, py311, py312, py313, .. } = PythonAnnotations::parse(&code);

    let mut push = |perform, unparsed: String| -> error::Result<()> {
        if perform {
            pep440_rs::VersionSpecifiers::from_str(unparsed.as_str())
                .ok()
                .map(|vs| version_specifiers.extend(vs.to_vec()));
        }
        Ok(())
    };
    push(py310, "==3.10.*".to_owned())?;
    push(py311, "==3.11.*".to_owned())?;
    push(py312, "==3.12.*".to_owned())?;
    push(py313, "==3.13.*".to_owned())?;

    for x in code.lines() {
        if x.starts_with("# py:") || x.starts_with("#py:") {
            push(
                true,
                x.replace('#', "").replace("py:", "").trim().to_owned(),
            )?;
        } else if !x.starts_with('#') {
            break;
        }
    }
    // we pass only if there is none or only one annotation

    // Naive:
    // 1. Check if there are multiple annotated version
    // 2. If no, take one and compare with annotated version
    // 3. We continue if same or replace none with new one

    // Optimized:
    // 1. Iterate over all annotations compare each with annotated_pyv and replace on flight
    // 2. If annotated_pyv is different version, throw and error

    // This way we make sure there is no multiple annotations for same script
    // and we get detailed span on conflicting versions

    #[derive(serde::Serialize, serde::Deserialize)]
    struct InlineMetadata {
        requires_python: String,
        dependencies: Vec<String>,
    }

    let find_requirements = code.lines().find_position(|x| {
        x.starts_with("#requirements:")
            || x.starts_with("# requirements:")
            || x.starts_with("# /// script")
    });
    if let Some((pos, item)) = find_requirements {
        let mut requirements = HashMap::new();
        if item == "# /// script" {
            let mut incorrect = false;
            let metadata = dbg!(code
                .lines()
                .skip(pos + 1)
                .map_while(|x| {
                    incorrect = !x.starts_with('#');
                    if incorrect || x.starts_with("# ///") {
                        None
                    } else {
                        x.get(1..)
                    }
                })
                .join("\n"))
            .parse::<toml::Table>()
            .map_err(to_anyhow)?;

            {
                if let Some(v) = metadata.get("requires-python").and_then(|v| v.as_str()) {
                    push(true, v.to_owned())?;
                }
            };

            metadata
                .get("dependencies")
                .and_then(|dependencies| dependencies.as_array())
                .inspect(|list| {
                    for dependency_v in list.into_iter() {
                        let requirement = dependency_v.as_str().unwrap_or("ERROR").to_owned();
                        requirements.insert(
                            key.clone(),
                            NImportResolved::Pin {
                                pins: vec![ImportPin {
                                    pkg: requirement.clone(),
                                    path: Default::default(),
                                }],
                                key,
                            },
                        );
                    }
                });
        } else {
            code.lines()
                .skip(pos + 1)
                .map_while(|x| {
                    RE.captures(x).and_then(|x| {
                        x.get(1).map(|m| {
                            let requirement = m.as_str().to_string();
                            requirements.insert(
                                requirement.clone(),
                                NImportResolved::Repin {
                                    pin: ImportPin { pkg: requirement, path: Default::default() },
                                    key: Default::default(),
                                },
                            );
                        })
                    })
                })
                .collect_vec();
        }
        Ok(requirements)
    } else {
        let find_extra_requirements = code.lines().find_position(|x| {
            x.starts_with("#extra_requirements:") || x.starts_with("# extra_requirements:")
        });
        let mut imports: HashMap<String, NImportResolved> = HashMap::new();
        if let Some((pos, _)) = find_extra_requirements {
            code.lines()
                .skip(pos + 1)
                .map_while(|x| {
                    RE.captures(x).and_then(|x| {
                        x.get(1).map(|m| {
                            let requirement = m.as_str().to_string();
                            let key = extract_pkg_name(&requirement);
                            imports.insert(
                                key.clone(),
                                NImportResolved::Pin {
                                    pins: vec![ImportPin {
                                        pkg: requirement,
                                        path: Default::default(),
                                    }],
                                    key,
                                },
                            );
                        })
                    })
                })
                .collect_vec();
        }

        // Will get unsorted vector of imports found in current script
        let mut nimports = parse_code_for_imports(code, path)?;

        // It is important to note, that sorting is important and will always result in this pattern:
        // 1. All Repins go first
        // 2. All Pins go second
        // 3. All Auto go third
        // 4. All relative imports go the last
        //
        // This way we make sure all repins are resolved before (re)pins inside imported relative scripts.
        nimports.sort();

        for n in nimports.into_iter() {
            let mut nested = match n {
                NImport::Relative(rpath) => {
                    let code = sqlx::query_scalar!(
                        r#"
                SELECT content FROM script WHERE path = $1 AND workspace_id = $2
                AND created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND
                workspace_id = $2)
                "#,
                        &rpath,
                        w_id
                    )
                    .fetch_optional(db)
                    .await?
                    .unwrap_or_else(|| "".to_string());

                    if already_visited.contains(&rpath) {
                        vec![]
                    } else {
                        already_visited.push(rpath.clone());
                        // Because the algo goes depth first, this function will never return relative import
                        // This why we can safely assume later, that there is no relative imports
                        parse_python_imports_inner(
                            &code,
                            w_id,
                            &rpath,
                            db,
                            already_visited,
                            version_specifiers,
                            path_where_annotated_pyv,
                        )
                        .await?
                        .into_values()
                        .collect_vec()
                    }
                }
                NImport::Repin { pin, key } => vec![NImportResolved::Repin { pin, key }],
                NImport::Pin { pins, key } => vec![NImportResolved::Pin { pins, key }],
                NImport::Auto { pkg, key } => vec![NImportResolved::Auto { pkg, key }],
            };

            // Nested should also be sorted for the same reason
            nested.sort();

            // At this point there should be no NImport::Relative in `nested`
            for imp in nested {
                let key = match imp.clone() {
                    NImportResolved::Pin { key, .. } => key,
                    NImportResolved::Repin { key, .. } => key,
                    NImportResolved::Auto { key, pkg } => key.unwrap_or(pkg),
                };
                // Handled cases:
                //
                //  1.
                //  Error: Imported windmill scripts have different pins
                //
                //  auto
                //  ├── pin:2
                //  └── pin:1
                //
                //  Fix 1:
                //
                //  auto
                //  ├── pin:1
                //  └── pin:1
                //
                //  Fix 2:
                //
                //  repin:1
                //  ├── pin:2
                //  └── pin:1
                //
                //  2.
                //  Error: Imported windmill scripts have different pins
                //
                //  pin:2
                //  └── pin:1
                //
                //  Fix 1:
                //
                //  auto
                //  └── pin:1
                //
                //  Fix 2:
                //
                //  repin:2
                //  └── pin:1
                //
                //  3. repins allowed to be repinned again
                //
                //  repin:2
                //  └── repin:1
                //
                match imp.clone() {
                    NImportResolved::Repin { .. } => {
                        if let Some(existing_import) = imports.get(&key) {
                            match existing_import {
                                // replace
                                p if matches!(
                                    p,
                                    NImportResolved::Pin { .. } | NImportResolved::Auto { .. }
                                ) =>
                                {
                                    imports.insert(key, imp);
                                }
                                // do nothing (older repins have greater precedence)
                                NImportResolved::Repin { .. } => {}
                                // Should not be possible
                                _ => {
                                    return Err(anyhow::anyhow!(
                                        "Internal error: cannot resolve requirement pins",
                                    )
                                    .into());
                                }
                            }
                        } else {
                            imports.insert(key, imp.clone());
                        }
                    }
                    NImportResolved::Pin { pins: new_pins, .. } => {
                        if let Some(existing_import) = imports.get_mut(&key) {
                            match existing_import {
                                // Check if pin is the same version, if same, do nothing, if not error
                                NImportResolved::Pin { pins: existing_pins, .. } => {
                                    existing_pins.extend(new_pins)
                                }
                                // do nothing
                                NImportResolved::Repin { .. } => {}
                                // Replace with new pin
                                NImportResolved::Auto { .. } => {
                                    imports.insert(key, imp);
                                }
                            }
                        } else {
                            imports.insert(key, imp.clone());
                        }
                    }
                    NImportResolved::Auto { .. } => {
                        if !imports.contains_key(&key) {
                            imports.insert(key, imp);
                        }
                    }
                }
            }
        }
        Ok(imports)
    }
}

const STDIMPORTS: [&str; 303] = [
    "--future--",
    "-abc",
    "-aix-support",
    "-ast",
    "-asyncio",
    "-bisect",
    "-blake2",
    "-bootsubprocess",
    "-bz2",
    "-codecs",
    "-codecs-cn",
    "-codecs-hk",
    "-codecs-iso2022",
    "-codecs-jp",
    "-codecs-kr",
    "-codecs-tw",
    "-collections",
    "-collections-abc",
    "-compat-pickle",
    "-compression",
    "-contextvars",
    "-crypt",
    "-csv",
    "-ctypes",
    "-curses",
    "-curses-panel",
    "-datetime",
    "-dbm",
    "-decimal",
    "-elementtree",
    "-frozen-importlib",
    "-frozen-importlib-external",
    "-functools",
    "-gdbm",
    "-hashlib",
    "-heapq",
    "-imp",
    "-io",
    "-json",
    "-locale",
    "-lsprof",
    "-lzma",
    "-markupbase",
    "-md5",
    "-msi",
    "-multibytecodec",
    "-multiprocessing",
    "-opcode",
    "-operator",
    "-osx-support",
    "-overlapped",
    "-pickle",
    "-posixshmem",
    "-posixsubprocess",
    "-py-abc",
    "-pydecimal",
    "-pyio",
    "-queue",
    "-random",
    "-sha1",
    "-sha256",
    "-sha3",
    "-sha512",
    "-signal",
    "-sitebuiltins",
    "-socket",
    "-sqlite3",
    "-sre",
    "-ssl",
    "-stat",
    "-statistics",
    "-string",
    "-strptime",
    "-struct",
    "-symtable",
    "-thread",
    "-threading-local",
    "-tkinter",
    "-tracemalloc",
    "-uuid",
    "-warnings",
    "-weakref",
    "-weakrefset",
    "-winapi",
    "-zoneinfo",
    "zoneinfo",
    "abc",
    "aifc",
    "antigravity",
    "argparse",
    "array",
    "ast",
    "asynchat",
    "asyncio",
    "asyncore",
    "atexit",
    "audioop",
    "base64",
    "bdb",
    "binascii",
    "binhex",
    "bisect",
    "builtins",
    "bz2",
    "cProfile",
    "calendar",
    "cgi",
    "cgitb",
    "chunk",
    "cmath",
    "cmd",
    "code",
    "codecs",
    "codeop",
    "collections",
    "colorsys",
    "compileall",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "copyreg",
    "crypt",
    "csv",
    "ctypes",
    "curses",
    "dataclasses",
    "datetime",
    "dbm",
    "decimal",
    "difflib",
    "dis",
    "distutils",
    "doctest",
    "email",
    "encodings",
    "ensurepip",
    "enum",
    "errno",
    "faulthandler",
    "fcntl",
    "filecmp",
    "fileinput",
    "fnmatch",
    "fractions",
    "ftplib",
    "functools",
    "gc",
    "genericpath",
    "getopt",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "grp",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "idlelib",
    "imaplib",
    "imghdr",
    "imp",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "lib2to3",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "mailbox",
    "mailcap",
    "marshal",
    "math",
    "mimetypes",
    "mmap",
    "modulefinder",
    "msilib",
    "msvcrt",
    "multiprocessing",
    "netrc",
    "nis",
    "nntplib",
    "nt",
    "ntpath",
    "nturl2path",
    "numbers",
    "opcode",
    "operator",
    "optparse",
    "os",
    "ossaudiodev",
    "pathlib",
    "pdb",
    "pickle",
    "pickletools",
    "pipes",
    "pkgutil",
    "platform",
    "plistlib",
    "poplib",
    "posix",
    "posixpath",
    "pprint",
    "profile",
    "pstats",
    "pty",
    "pwd",
    "py-compile",
    "pyclbr",
    "pydoc",
    "pydoc-data",
    "pyexpat",
    "queue",
    "quopri",
    "random",
    "re",
    "readline",
    "reprlib",
    "resource",
    "rlcompleter",
    "runpy",
    "sched",
    "secrets",
    "select",
    "selectors",
    "shelve",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtpd",
    "smtplib",
    "sndhdr",
    "socket",
    "socketserver",
    "spwd",
    "sqlite3",
    "sre-compile",
    "sre-constants",
    "sre-parse",
    "ssl",
    "stat",
    "statistics",
    "string",
    "stringprep",
    "struct",
    "subprocess",
    "sunau",
    "symtable",
    "sys",
    "sysconfig",
    "syslog",
    "tabnanny",
    "tarfile",
    "telnetlib",
    "tempfile",
    "termios",
    "textwrap",
    "this",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "token",
    "tokenize",
    "trace",
    "traceback",
    "tracemalloc",
    "tty",
    "turtle",
    "turtledemo",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uu",
    "uuid",
    "venv",
    "warnings",
    "wave",
    "weakref",
    "webbrowser",
    "winreg",
    "winsound",
    "wsgiref",
    "xdrlib",
    "xml",
    "xmlrpc",
    "zipapp",
    "zipfile",
    "zipimport",
    "zlib",
    "",
];
