/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use itertools::Itertools;
use lazy_static::lazy_static;
use phf::phf_map;
use regex::Regex;

use serde_json::json;
use windmill_common::error;
use windmill_parser::{json_to_typ, Arg, MainArgSignature, Typ};

use rustpython_parser::{
    ast::{Constant, ExprKind, Located, StmtKind},
    parser,
};

const DEF_MAIN: &str = "def main(";
const FUNCTION_CALL: &str = "<function call>";

fn filter_non_main(code: &str) -> String {
    let mut filtered_code = String::new();
    let mut code_iter = code.split("\n");
    let mut remaining: String = String::new();
    while let Some(line) = code_iter.next() {
        if line.starts_with(DEF_MAIN) {
            filtered_code += DEF_MAIN;
            remaining += line.strip_prefix(DEF_MAIN).unwrap();
            remaining += &code_iter.join("\n");
            break;
        }
    }
    if filtered_code.is_empty() {
        return String::new();
    }
    let mut chars = remaining.chars();
    let mut open_parens = 1;

    while let Some(c) = chars.next() {
        if c == '(' {
            open_parens += 1;
        } else if c == ')' {
            open_parens -= 1;
        }
        filtered_code.push(c);
        if open_parens == 0 {
            break;
        }
    }

    filtered_code.push_str(": return");
    return filtered_code;
}

pub fn parse_python_signature(code: &str) -> error::Result<MainArgSignature> {
    let filtered_code = filter_non_main(code);
    if filtered_code.is_empty() {
        return Err(error::Error::BadRequest(
            "No main function found".to_string(),
        ));
    }
    let ast = parser::parse_program(&filtered_code, "main.py").map_err(|e| {
        error::Error::ExecutionErr(format!("Error parsing code: {}", e.to_string()))
    })?;
    let param = ast.into_iter().find_map(|x| match x {
        Located { node: StmtKind::FunctionDef { name, args, .. }, .. } if &name == "main" => {
            Some(*args)
        }
        _ => None,
    });
    if let Some(params) = param {
        //println!("{:?}", params);
        let def_arg_start = params.args.len() - params.defaults.len();
        Ok(MainArgSignature {
            star_args: params.vararg.is_some(),
            star_kwargs: params.vararg.is_some(),
            args: params
                .args
                .into_iter()
                .enumerate()
                .map(|(i, x)| {
                    let x = x.node;
                    let default = if i >= def_arg_start {
                        to_value(&params.defaults[i - def_arg_start].node)
                    } else {
                        None
                    };

                    let mut typ = x.annotation.map_or(Typ::Unknown, |e| match *e {
                        Located { node: ExprKind::Name { id, .. }, .. } => match id.as_ref() {
                            "str" => Typ::Str(None),
                            "float" => Typ::Float,
                            "int" => Typ::Int,
                            "bool" => Typ::Bool,
                            "dict" => Typ::Object(vec![]),
                            "list" => Typ::List(Box::new(Typ::Str(None))),
                            "bytes" => Typ::Bytes,
                            "datetime" => Typ::Datetime,
                            "datetime.datetime" => Typ::Datetime,
                            _ => Typ::Resource(id),
                        },
                        _ => Typ::Unknown,
                    });

                    if typ == Typ::Unknown
                        && default.is_some()
                        && default != Some(json!(FUNCTION_CALL))
                    {
                        typ = json_to_typ(default.as_ref().unwrap());
                    }

                    Arg { otyp: None, name: x.arg, typ, has_default: default.is_some(), default }
                })
                .collect(),
        })
    } else {
        Err(error::Error::ExecutionErr(
            "main function was not findable".to_string(),
        ))
    }
}

fn to_value(et: &ExprKind) -> Option<serde_json::Value> {
    match et {
        ExprKind::Constant { value, .. } => Some(constant_to_value(value)),
        ExprKind::Dict { keys, values } => {
            let v = keys
                .into_iter()
                .zip(values)
                .map(|(k, v)| {
                    let key = to_value(&k.node)
                        .and_then(|x| match x {
                            serde_json::Value::String(s) => Some(s),
                            _ => None,
                        })
                        .unwrap_or_else(|| "no_key".to_string());
                    (key, to_value(&v.node))
                })
                .collect::<HashMap<String, _>>();
            Some(json!(v))
        }
        ExprKind::List { elts, .. } => {
            let v = elts
                .into_iter()
                .map(|x| to_value(&x.node))
                .collect::<Vec<_>>();
            Some(json!(v))
        }
        ExprKind::Call { .. } => Some(json!(FUNCTION_CALL)),
        _ => None,
    }
}

fn constant_to_value(c: &Constant) -> serde_json::Value {
    match c {
        Constant::None => json!(null),
        Constant::Bool(b) => json!(b),
        Constant::Str(s) => json!(s),
        Constant::Bytes(b) => json!(b),
        Constant::Int(i) => serde_json::from_str(&i.to_string()).unwrap_or(json!("invalid number")),
        Constant::Tuple(t) => json!(t.iter().map(constant_to_value).collect::<Vec<_>>()),
        Constant::Float(f) => json!(f),
        Constant::Complex { real, imag } => json!([real, imag]),
        Constant::Ellipsis => json!("..."),
    }
}

static PYTHON_IMPORTS_REPLACEMENT: phf::Map<&'static str, &'static str> = phf_map! {
    "psycopg2" => "psycopg2-binary",
    "yaml" => "pyyaml",
    "git" => "GitPython"
};

fn replace_import(x: String) -> String {
    PYTHON_IMPORTS_REPLACEMENT
        .get(&x)
        .map(|x| x.to_owned())
        .unwrap_or(&x)
        .to_string()
}

lazy_static! {
    static ref RE: Regex = Regex::new(r"^\#(\S+)$").unwrap();
}

pub fn parse_python_imports(code: &str) -> error::Result<Vec<String>> {
    let find_requirements = code
        .lines()
        .find_position(|x| x.starts_with("#requirements:"));
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
        let code = code.split(DEF_MAIN).next().unwrap_or("");
        let ast = parser::parse_program(code, "main.py").map_err(|e| {
            error::Error::ExecutionErr(format!("Error parsing code: {}", e.to_string()))
        })?;
        let imports = ast
            .into_iter()
            .filter_map(|x| match x {
                Located { node, .. } => match node {
                    StmtKind::Import { names } => Some(
                        names
                            .into_iter()
                            .map(|x| x.node.name.split('.').next().unwrap_or("").to_string())
                            .map(replace_import)
                            .collect::<Vec<String>>(),
                    ),
                    StmtKind::ImportFrom { level: _, module: Some(mod_), names: _ } => {
                        let imprt = mod_.split('.').next().unwrap_or("").replace("_", "-");

                        Some(vec![replace_import(imprt)])
                    }
                    _ => None,
                },
            })
            .flatten()
            .filter(|x| !STDIMPORTS.contains(&x.as_str()))
            .unique()
            .collect();

        Ok(imports)
    }
}

#[cfg(test)]
mod tests {

    use serde_json::json;

    use super::*;

    #[test]
    fn test_parse_python_sig() -> anyhow::Result<()> {
        let code = "

import os

def main(test1: str, name: datetime.datetime = datetime.now(), byte: bytes = bytes(1), f = \"wewe\", g = 21, h = [1,2], i = True):

	print(f\"Hello World and a warm welcome especially to {name}\")
	print(\"The env variable at `all/pretty_secret`: \", os.environ.get(\"ALL_PRETTY_SECRET\"))
	return {\"len\": len(name), \"splitted\": name.split() }

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Unknown,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "f".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("wewe")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "g".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(21)),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "h".to_string(),
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2])),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "i".to_string(),
                        typ: Typ::Bool,
                        default: Some(json!(true)),
                        has_default: true
                    },
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_2() -> anyhow::Result<()> {
        let code = "

import os

postgresql = dict
def main(test1: str,
    name: datetime.datetime = datetime.now(),
    byte: bytes = bytes(1),
    resource: postgresql = \"$res:g/all/resource\"):

	print(f\"Hello World and a warm welcome especially to {name}\")
	print(\"The env variable at `all/pretty_secret`: \", os.environ.get(\"ALL_PRETTY_SECRET\"))
	return {\"len\": len(name), \"splitted\": name.split() }

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Unknown,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "resource".to_string(),
                        typ: Typ::Resource("postgresql".to_string()),
                        default: Some(json!("$res:g/all/resource")),
                        has_default: true
                    }
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_3() -> anyhow::Result<()> {
        let code = "

import os

def main(test1: str,
    name = \"test\",
    byte: bytes = bytes(1)): return

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("test")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true
                    }
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_imports() -> anyhow::Result<()> {
        //let code = "print(2 + 3, fd=sys.stderr)";
        let code = "

import os
import wmill
from zanzibar.estonie import talin
import matplotlib.pyplot as plt

def main():
    pass

";
        let r = parse_python_imports(code)?;
        // println!("{}", serde_json::to_string(&r)?);
        assert_eq!(r, vec!["wmill", "zanzibar", "matplotlib"]);
        Ok(())
    }

    #[test]
    fn test_parse_python_imports2() -> anyhow::Result<()> {
        //let code = "print(2 + 3, fd=sys.stderr)";
        let code = "
#requirements:
#burkina=0.4
#nigeria
#
#congo

import os
import wmill
from zanzibar.estonie import talin

def main():
    pass

";
        let r = parse_python_imports(code)?;
        println!("{}", serde_json::to_string(&r)?);
        assert_eq!(r, vec!["burkina=0.4", "nigeria"]);

        Ok(())
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
