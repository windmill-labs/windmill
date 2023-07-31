/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use async_recursion::async_recursion;
use itertools::Itertools;
use lazy_static::lazy_static;
use phf::phf_map;
use regex::Regex;

use sqlx::{Pool, Postgres};
use windmill_common::error;

use rustpython_parser::ast::{Located, StmtKind};
use rustpython_parser::parser::parse_program;

const DEF_MAIN: &str = "def main(";

static PYTHON_IMPORTS_REPLACEMENT: phf::Map<&'static str, &'static str> = phf_map! {
    "psycopg2" => "psycopg2-binary",
    "psycopg" => "psycopg[binary, pool]",
    "yaml" => "pyyaml",
    "git" => "GitPython",
    "shopify" => "ShopifyAPI",
    "seleniumwire" => "selenium-wire",
    "openbb-terminal" => "openbb[all]",
    "riskfolio" => "riskfolio-lib",
    "smb" => "pysmb",
    "PIL" => "Pillow",
    "googleapiclient" => "google-api-python-client",
};

fn replace_import(x: String) -> String {
    PYTHON_IMPORTS_REPLACEMENT
        .get(&x)
        .map(|x| x.to_owned())
        .unwrap_or(&x)
        .to_string()
}

lazy_static! {
    static ref RE: Regex = Regex::new(r"^\#\s?(\S+)$").unwrap();
}

fn process_import(module: Option<String>, path: &str, level: usize) -> Vec<String> {
    if level > 0 {
        let mut imports = vec!["requests".to_string()];
        let splitted_path = path.split("/");
        let base = splitted_path
            .clone()
            .take(splitted_path.count() - level)
            .join("/");
        if let Some(m) = module {
            imports.push(format!("relative:{base}/{}", m.replace(".", "/")));
        } else {
            imports.push(format!("relative:{base}"));
        }
        imports
    } else if let Some(module) = module {
        let imprt = module.split('.').next().unwrap_or("").replace("_", "-");
        if imprt == "u" || imprt == "f" {
            vec![
                "requests".to_string(),
                format!("relative:{}", module.replace(".", "/")),
            ]
        } else {
            vec![imprt]
        }
    } else {
        vec![]
    }
}

#[async_recursion]
pub async fn parse_python_imports(
    code: &str,
    w_id: &str,
    path: &str,
    db: &Pool<Postgres>,
) -> error::Result<Vec<String>> {
    let find_requirements = code
        .lines()
        .find_position(|x| x.starts_with("#requirements:") || x.starts_with("# requirements:"));
    if let Some((pos, _)) = find_requirements {
        let lines = code
            .lines()
            .skip(pos + 1)
            .map_while(|x| {
                RE.captures(x)
                    .map(|x| x.get(1).unwrap().as_str().to_string())
            })
            .collect();
        Ok(lines)
    } else {
        let find_extra_requirements = code.lines().find_position(|x| {
            x.starts_with("#extra_requirements:") || x.starts_with("# extra_requirements:")
        });
        let mut imports: Vec<String> = vec![];
        if let Some((pos, _)) = find_extra_requirements {
            let lines: Vec<String> = code
                .lines()
                .skip(pos + 1)
                .map_while(|x| {
                    RE.captures(x)
                        .map(|x| x.get(1).unwrap().as_str().to_string())
                })
                .collect();
            imports.extend(lines);
        }

        let code = code.split(DEF_MAIN).next().unwrap_or("");
        let ast = parse_program(code, "main.py").map_err(|e| {
            error::Error::ExecutionErr(format!("Error parsing code: {}", e.to_string()))
        })?;
        let nimports: Vec<String> = ast
            .into_iter()
            .filter_map(|x| match x {
                Located { node, .. } => match node {
                    StmtKind::Import { names } => Some(
                        names
                            .into_iter()
                            .map(|x| {
                                let name = x.node.name;
                                process_import(Some(name), path, 0)
                            })
                            .flatten()
                            .collect::<Vec<String>>(),
                    ),
                    StmtKind::ImportFrom { level: Some(i), module, .. } if i > 0 => {
                        Some(process_import(module, path, i))
                    }
                    StmtKind::ImportFrom { level: _, module, names: _ } => {
                        Some(process_import(module, path, 0))
                    }
                    _ => None,
                },
            })
            .flatten()
            .filter(|x| !STDIMPORTS.contains(&x.as_str()))
            .unique()
            .collect();
        for n in nimports.iter() {
            let nested = if n.starts_with("relative:") {
                let code = sqlx::query_scalar!(
                    r#"
                    SELECT content FROM script WHERE path = $1 AND workspace_id = $2
                    AND created_at = (SELECT max(created_at) FROM script WHERE path = $1 AND 
                    workspace_id = $2)
                    "#,
                    n.replace("relative:", ""),
                    w_id
                )
                .fetch_optional(db)
                .await?
                .unwrap_or_else(|| "".to_string());
                parse_python_imports(&code, w_id, path, db).await?
            } else {
                vec![replace_import(n.to_string())]
            };
            for imp in nested {
                if !imports.contains(&imp) {
                    imports.push(imp);
                }
            }
        }
        imports.sort();
        Ok(imports)
    }
}

const STDIMPORTS: [&str; 301] = [
    "__future__",
    "_abc",
    "_aix_support",
    "_ast",
    "_asyncio",
    "_bisect",
    "_blake2",
    "_bootsubprocess",
    "_bz2",
    "_codecs",
    "_codecs_cn",
    "_codecs_hk",
    "_codecs_iso2022",
    "_codecs_jp",
    "_codecs_kr",
    "_codecs_tw",
    "_collections",
    "_collections_abc",
    "_compat_pickle",
    "_compression",
    "_contextvars",
    "_crypt",
    "_csv",
    "_ctypes",
    "_curses",
    "_curses_panel",
    "_datetime",
    "_dbm",
    "_decimal",
    "_elementtree",
    "_frozen_importlib",
    "_frozen_importlib_external",
    "_functools",
    "_gdbm",
    "_hashlib",
    "_heapq",
    "_imp",
    "_io",
    "_json",
    "_locale",
    "_lsprof",
    "_lzma",
    "_markupbase",
    "_md5",
    "_msi",
    "_multibytecodec",
    "_multiprocessing",
    "_opcode",
    "_operator",
    "_osx_support",
    "_overlapped",
    "_pickle",
    "_posixshmem",
    "_posixsubprocess",
    "_py_abc",
    "_pydecimal",
    "_pyio",
    "_queue",
    "_random",
    "_sha1",
    "_sha256",
    "_sha3",
    "_sha512",
    "_signal",
    "_sitebuiltins",
    "_socket",
    "_sqlite3",
    "_sre",
    "_ssl",
    "_stat",
    "_statistics",
    "_string",
    "_strptime",
    "_struct",
    "_symtable",
    "_thread",
    "_threading_local",
    "_tkinter",
    "_tracemalloc",
    "_uuid",
    "_warnings",
    "_weakref",
    "_weakrefset",
    "_winapi",
    "_zoneinfo",
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
    "py_compile",
    "pyclbr",
    "pydoc",
    "pydoc_data",
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
    "sre_compile",
    "sre_constants",
    "sre_parse",
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
    "",
];
