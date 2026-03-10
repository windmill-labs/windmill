/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Python import parsing for detecting relative imports.
//! This module extracts import statements from Python code and identifies
//! which ones are relative imports (local script references).

use itertools::Itertools;

#[cfg(not(target_arch = "wasm32"))]
use regex::Regex;
#[cfg(target_arch = "wasm32")]
use regex_lite::Regex;

use rustpython_parser::{
    ast::{Stmt, StmtImport, StmtImportFrom, Suite},
    Parse,
};

use std::sync::LazyLock;

static DEF_MAIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(async\s+)?def\s+main\s*\(").unwrap());

/// Represents a parsed import statement
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PythonImport {
    /// A relative import (local script reference)
    /// e.g., `from .module import x` or `from f.folder.module import x`
    Relative(String),
    /// An external package import
    External(String),
}

/// Standard library imports that should not be treated as external dependencies
const STDIMPORTS: &[&str] = &[
    "--future--",
    "abc",
    "aifc",
    "argparse",
    "array",
    "ast",
    "asynchat",
    "asyncio",
    "asyncore",
    "atexit",
    "base64",
    "bdb",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
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
    "multiprocessing",
    "netrc",
    "nis",
    "nntplib",
    "numbers",
    "operator",
    "optparse",
    "os",
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
    "pyclbr",
    "pydoc",
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
    "zoneinfo",
    "",
];

fn is_stdlib(name: &str) -> bool {
    STDIMPORTS.contains(&name)
}

/// Process an import statement and determine if it's relative or external
fn process_import(module: Option<String>, path: &str, level: usize) -> Option<PythonImport> {
    if level > 0 {
        // Relative import with dots (e.g., from .module import x)
        let splitted_path = path.split('/');
        let base = splitted_path
            .clone()
            .take(splitted_path.count().saturating_sub(level))
            .join("/");
        if let Some(m) = module {
            Some(PythonImport::Relative(format!(
                "{}/{}",
                base,
                m.replace('.', "/")
            )))
        } else {
            Some(PythonImport::Relative(base))
        }
    } else if let Some(module) = module {
        let root = module.split('.').next().unwrap_or("").replace('_', "-");
        // Check for Windmill-style absolute imports (f.folder.module or u.user.module)
        if root == "u" || root == "f" {
            Some(PythonImport::Relative(module.replace('.', "/")))
        } else if is_stdlib(&root) {
            None // Skip stdlib imports
        } else {
            Some(PythonImport::External(root))
        }
    } else {
        None
    }
}

/// Parse Python code and extract all imports
pub fn parse_python_imports(code: &str, path: &str) -> anyhow::Result<Vec<PythonImport>> {
    // Remove everything after main function definition to avoid parsing errors
    let mut code = DEF_MAIN_RE
        .split(code)
        .next()
        .unwrap_or_default()
        .to_string();

    // Remove main function decorator from end of file if it exists
    if code
        .lines()
        .last()
        .map(|x| x.starts_with('@'))
        .unwrap_or(false)
    {
        code = code
            .lines()
            .take(code.lines().count() - 1)
            .collect::<Vec<&str>>()
            .join("\n")
            + "\n";
    }

    // Add a fake main function to ensure the parser can process the code correctly
    let code_with_fake_main = format!("{}\n\ndef main(): pass", code);

    let ast = Suite::parse(&code_with_fake_main, "main.py").map_err(|e| {
        anyhow::anyhow!("Error parsing code for imports: {}", e.to_string())
    })?;

    let imports: Vec<PythonImport> = ast
        .into_iter()
        .filter_map(|x| match x {
            Stmt::Import(StmtImport { names, .. }) => Some(
                names
                    .into_iter()
                    .filter_map(|x| {
                        let name = x.name.to_string();
                        process_import(Some(name), path, 0)
                    })
                    .collect::<Vec<_>>(),
            ),
            Stmt::ImportFrom(StmtImportFrom {
                level: Some(i),
                module,
                ..
            }) if i.to_u32() > 0 => {
                process_import(module.map(|x| x.to_string()), path, i.to_usize())
                    .map(|imp| vec![imp])
            }
            Stmt::ImportFrom(StmtImportFrom { module, .. }) => {
                process_import(module.map(|x| x.to_string()), path, 0).map(|imp| vec![imp])
            }
            _ => None,
        })
        .flatten()
        .unique()
        .collect();

    Ok(imports)
}

/// Parse Python code and return only the relative imports (local script references)
pub fn parse_relative_imports(code: &str, path: &str) -> anyhow::Result<Vec<String>> {
    let imports = parse_python_imports(code, path)?;
    Ok(imports
        .into_iter()
        .filter_map(|x| match x {
            PythonImport::Relative(path) => Some(path),
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_relative_import_dot() {
        let code = r#"
from .helper import my_func

def main():
    return my_func()
"#;
        let result = parse_relative_imports(code, "f/folder/script").unwrap();
        assert_eq!(result, vec!["f/folder/helper"]);
    }

    #[test]
    fn test_parse_relative_import_double_dot() {
        let code = r#"
from ..utils.helper import my_func

def main():
    return my_func()
"#;
        let result = parse_relative_imports(code, "f/folder/subfolder/script").unwrap();
        assert_eq!(result, vec!["f/folder/utils/helper"]);
    }

    #[test]
    fn test_parse_absolute_windmill_import() {
        let code = r#"
from f.shared.utils import helper

def main():
    return helper()
"#;
        let result = parse_relative_imports(code, "f/folder/script").unwrap();
        assert_eq!(result, vec!["f/shared/utils"]);
    }

    #[test]
    fn test_parse_user_import() {
        let code = r#"
from u.admin.helpers import util

def main():
    return util()
"#;
        let result = parse_relative_imports(code, "f/folder/script").unwrap();
        assert_eq!(result, vec!["u/admin/helpers"]);
    }

    #[test]
    fn test_parse_stdlib_ignored() {
        let code = r#"
import os
import json
from collections import defaultdict

def main():
    return {}
"#;
        let result = parse_relative_imports(code, "f/folder/script").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_mixed_imports() {
        let code = r#"
import os
from .helper import func1
from f.shared.utils import func2
import requests

def main():
    return func1() + func2()
"#;
        let result = parse_relative_imports(code, "f/folder/script").unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"f/folder/helper".to_string()));
        assert!(result.contains(&"f/shared/utils".to_string()));
    }
}
