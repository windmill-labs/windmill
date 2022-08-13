/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use itertools::Itertools;
use phf::phf_map;
use regex::Regex;
use serde_json::json;

use crate::{
    error,
    parser::{Arg, InnerTyp, MainArgSignature, Typ},
};

use rustpython_parser::{
    ast::{ExpressionType, Located, Number, StatementType, StringGroup, Varargs},
    parser,
};

pub fn parse_python_signature(code: &str) -> error::Result<MainArgSignature> {
    let ast = parser::parse_program(code)
        .map_err(|e| error::Error::ExecutionErr(format!("Error parsing code: {}", e.to_string())))?
        .statements;
    let param = ast.into_iter().find_map(|x| match x {
        Located {
            location: _,
            node:
                StatementType::FunctionDef {
                    is_async: _,
                    name,
                    args,
                    body: _,
                    decorator_list: _,
                    returns: _,
                },
        } if &name == "main" => Some(*args),
        _ => None,
    });
    if let Some(params) = param {
        //println!("{:?}", params);
        let def_arg_start = params.args.len() - params.defaults.len();
        Ok(MainArgSignature {
            star_args: params.vararg != Varargs::None,
            star_kwargs: params.vararg != Varargs::None,
            args: params
                .args
                .into_iter()
                .enumerate()
                .map(|(i, x)| {
                    let default = if i >= def_arg_start {
                        to_value(&params.defaults[i - def_arg_start].node)
                    } else {
                        None
                    };
                    Arg {
                        name: x.arg,
                        typ: x.annotation.map_or(Typ::Unknown, |e| match *e {
                            Located { location: _, node: ExpressionType::Identifier { name } } => {
                                match name.as_ref() {
                                    "str" => Typ::Str(None),
                                    "float" => Typ::Float,
                                    "int" => Typ::Int,
                                    "bool" => Typ::Bool,
                                    "dict" => Typ::Dict,
                                    "list" => Typ::List(InnerTyp::Str),
                                    "bytes" => Typ::Bytes,
                                    "datetime" => Typ::Datetime,
                                    "datetime.datetime" => Typ::Datetime,
                                    _ => Typ::Unknown,
                                }
                            }
                            _ => Typ::Unknown,
                        }),
                        has_default: default.is_some(),
                        default,
                    }
                })
                .collect(),
        })
    } else {
        Err(error::Error::ExecutionErr(
            "main function was not findable".to_string(),
        ))
    }
}

fn to_value(et: &ExpressionType) -> Option<serde_json::Value> {
    match et {
        ExpressionType::String { value: StringGroup::Constant { value } } => Some(json!(value)),
        ExpressionType::Number { value } => match value {
            Number::Integer { value } => Some(json!(value.to_string().parse::<i64>().unwrap())),
            Number::Float { value } => Some(json!(value)),
            _ => None,
        },
        ExpressionType::True => Some(json!(true)),
        ExpressionType::False => Some(json!(false)),

        ExpressionType::Dict { elements } => {
            let v = elements
                .into_iter()
                .map(|(k, v)| {
                    let key = k
                        .as_ref()
                        .and_then(|x| to_value(&x.node))
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
        ExpressionType::List { elements } => {
            let v = elements
                .into_iter()
                .map(|x| to_value(&x.node))
                .collect::<Vec<_>>();
            Some(json!(v))
        }
        ExpressionType::None => Some(json!(null)),

        ExpressionType::Call { function: _, args: _, keywords: _ } => {
            Some(json!("<function call>"))
        }

        _ => None,
    }
}

static PYTHON_IMPORTS_REPLACEMENT: phf::Map<&'static str, &'static str> = phf_map! {
    "psycopg2" => "psycopg2-binary"
};

fn replace_import(x: String) -> String {
    PYTHON_IMPORTS_REPLACEMENT
        .get(&x)
        .map(|x| x.to_owned())
        .unwrap_or(&x)
        .to_string()
}

pub fn parse_python_imports(code: &str) -> error::Result<Vec<String>> {
    let find_requirements = code
        .lines()
        .find_position(|x| x.starts_with("#requirements:"));
    let re = Regex::new(r"^\#(\S+)$").unwrap();
    if let Some((pos, _)) = find_requirements {
        let lines = code
            .lines()
            .skip(pos + 1)
            .map_while(|x| {
                re.captures(x)
                    .map(|x| x.get(1).unwrap().as_str().to_string())
            })
            .collect();
        Ok(lines)
    } else {
        let ast = parser::parse_program(code)
            .map_err(|e| {
                error::Error::ExecutionErr(format!("Error parsing code: {}", e.to_string()))
            })?
            .statements;
        let imports = ast
            .into_iter()
            .filter_map(|x| match x {
                Located { location: _, node } => match node {
                    StatementType::Import { names } => Some(
                        names
                            .into_iter()
                            .map(|x| x.symbol.split('.').next().unwrap_or("").to_string())
                            .map(replace_import)
                            .collect::<Vec<String>>(),
                    ),
                    StatementType::ImportFrom { level: _, module: Some(mod_), names: _ } => {
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

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_python_sig() -> anyhow::Result<()> {
        //let code = "print(2 + 3, fd=sys.stderr)";
        let code = "

import os

def main(test1: str, name: datetime.datetime = datetime.now(), byte: bytes = bytes(1)):

	print(f\"Hello World and a warm welcome especially to {name}\")
	print(\"The env variable at `all/pretty_secret`: \", os.environ.get(\"ALL_PRETTY_SECRET\"))
	return {\"len\": len(name), \"splitted\": name.split() }

";
        println!("{}", serde_json::to_string(&parse_python_signature(code)?)?);

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
        println!("{}", serde_json::to_string(&r)?);
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
