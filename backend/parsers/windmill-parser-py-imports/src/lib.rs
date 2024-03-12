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

use rustpython_parser::{
    ast::{Stmt, StmtImport, StmtImportFrom, Suite},
    Parse,
};
use sqlx::{Pool, Postgres};
use windmill_common::error;

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
    "dateutil" => "python-dateutil",
    "mailparser" => "mail-parser",
    "mailparser-reply" => "mail-parser-reply",
    "gitlab" => "python-gitlab",
    "smbclient" => "smbprotocol",
    "playhouse" => "peewee",
    "dns" => "dnspython",
    "msoffcrypto" => "msoffcrypto-tool",
    "tabula" => "tabula-py",
    "shapefile" => "pyshp",
    "sklearn" => "scikit-learn",
    "umap" => "umap-learn",
    "cv2" => "opencv-python",
    "atlassian" => "atlassian-python-api",
    "mysql" => "mysql-connector-python",
    "tenable" => "pytenable",
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
        let mut imports = vec![];
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
            vec![format!("relative:{}", module.replace(".", "/"))]
        } else {
            vec![imprt]
        }
    } else {
        vec![]
    }
}

pub fn parse_relative_imports(code: &str, path: &str) -> error::Result<Vec<String>> {
    let nimports = parse_code_for_imports(code, path)?;
    return Ok(nimports
        .into_iter()
        .filter_map(|x| {
            if x.starts_with("relative:") {
                Some(x.replace("relative:", ""))
            } else {
                None
            }
        })
        .collect());
}

fn parse_code_for_imports(code: &str, path: &str) -> error::Result<Vec<String>> {
    let code = code.split(DEF_MAIN).next().unwrap_or("");
    let ast = Suite::parse(code, "main.py").map_err(|e| {
        error::Error::ExecutionErr(format!("Error parsing code: {}", e.to_string()))
    })?;
    let nimports: Vec<String> = ast
        .into_iter()
        .filter_map(|x| match x {
            Stmt::Import(StmtImport { names, .. }) => Some(
                names
                    .into_iter()
                    .map(|x| {
                        let name = x.name.to_string();
                        process_import(Some(name), path, 0)
                    })
                    .flatten()
                    .collect::<Vec<String>>(),
            ),
            Stmt::ImportFrom(StmtImportFrom { level: Some(i), module, .. }) if i.to_u32() > 0 => {
                Some(process_import(
                    module.map(|x| x.to_string()),
                    path,
                    i.to_usize(),
                ))
            }
            Stmt::ImportFrom(StmtImportFrom { level: _, module, .. }) => {
                Some(process_import(module.map(|x| x.to_string()), path, 0))
            }
            _ => None,
        })
        .flatten()
        .filter(|x| !STDIMPORTS.contains(&x.as_str()))
        .unique()
        .collect();
    return Ok(nimports);
}

#[async_recursion]
pub async fn parse_python_imports(
    code: &str,
    w_id: &str,
    path: &str,
    db: &Pool<Postgres>,
    already_visited: &mut Vec<String>,
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

        let nimports = parse_code_for_imports(code, path)?;
        for n in nimports.iter() {
            let nested = if n.starts_with("relative:") {
                let rpath = n.replace("relative:", "");
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
                    parse_python_imports(&code, w_id, &rpath, db, already_visited).await?
                }
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

const STDIMPORTS: [&str; 302] = [
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
    "",
];
