/*
 * Author & Copyright: Ruben Fiszel 2021
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use itertools::Itertools;
use regex::Regex;
use serde::Serialize;
use serde_json::json;

use crate::error;

use rustpython_parser::{
    ast::{ExpressionType, Located, Number, StatementType, StringGroup, Varargs},
    parser,
};
#[derive(Serialize)]
pub struct MainArgSignature {
    pub star_args: bool,
    pub star_kwargs: bool,
    pub args: Vec<Arg>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum InnerTyp {
    Str,
    Int,
    Float,
    Bytes,
    Email,
}

#[derive(Serialize, Clone)]
#[serde(rename_all(serialize = "lowercase"))]
pub enum Typ {
    Str,
    Int,
    Float,
    Bool,
    Dict,
    List(InnerTyp),
    Bytes,
    Datetime,
    Resource(String),
    Email,
    Unknown,
}

#[derive(Serialize, Clone)]
pub struct Arg {
    pub name: String,
    pub typ: Typ,
    pub default: Option<serde_json::Value>,
    pub has_default: bool,
}

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
                            Located {
                                location: _,
                                node: ExpressionType::Identifier { name },
                            } => match name.as_ref() {
                                "str" => Typ::Str,
                                "float" => Typ::Float,
                                "int" => Typ::Int,
                                "bool" => Typ::Bool,
                                "dict" => Typ::Dict,
                                "list" => Typ::List(InnerTyp::Str),
                                "bytes" => Typ::Bytes,
                                "datetime" => Typ::Datetime,
                                "datetime.datetime" => Typ::Datetime,
                                _ => Typ::Unknown,
                            },
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

use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::{
    AssignPat, BindingIdent, Decl, ExportDecl, FnDecl, Ident, ModuleDecl, ModuleItem, Pat,
    TsArrayType, TsEntityName, TsKeywordTypeKind, TsType, TsTypeRef,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

pub fn parse_deno_signature(code: &str) -> error::Result<MainArgSignature> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.ts".into()), code.into());
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Typescript(TsConfig::default()),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    let mut err_s = "".to_string();
    for e in parser.take_errors() {
        err_s += &e.into_kind().msg().to_string();
    }

    let ast = parser
        .parse_module()
        .map_err(|e| {
            error::Error::ExecutionErr(format!("impossible to parse module: {err_s}\n{e:?}"))
        })?
        .body;

    // println!("{ast:?}");
    let params = ast.into_iter().find_map(|x| match x {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl:
                Decl::Fn(FnDecl {
                    ident:
                        Ident {
                            span: _,
                            sym,
                            optional: _,
                        },
                    declare: _,
                    function,
                }),
            span: _,
        })) if &sym.to_string() == "main" => Some(function.params),
        _ => None,
    });
    if let Some(params) = params {
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: params
                .into_iter()
                .map(|x| match x.pat {
                    Pat::Ident(ident) => {
                        let (name, typ) = binding_ident_to_arg(&ident)?;
                        Ok(Arg {
                            name,
                            typ,
                            default: None,
                            has_default: ident.id.optional,
                        })
                    }
                    Pat::Assign(AssignPat {
                        span: _,
                        left,
                        right,
                        type_ann: _,
                    }) => {
                        let (name, typ) =
                            left.as_ident().map(binding_ident_to_arg).ok_or_else(|| {
                                error::Error::ExecutionErr(format!(
                                    "Arg {left:?} has unexpected syntax"
                                ))
                            })??;
                        Ok(Arg {
                            name,
                            typ,
                            default: serde_json::to_value(right)
                                .map_err(|e| error::Error::ExecutionErr(e.to_string()))?
                                .as_object()
                                .and_then(|x| x.get("value").to_owned())
                                .cloned(),

                            has_default: true,
                        })
                    }
                    _ => Err(error::Error::ExecutionErr(format!(
                        "Arg {x:?} has unexpected syntax"
                    ))),
                })
                .collect::<Result<Vec<Arg>, error::Error>>()?,
        })
    } else {
        Err(error::Error::ExecutionErr(
            "main function was not findable (expected to find 'export main function(...)'"
                .to_string(),
        ))
    }
}

fn binding_ident_to_arg(
    BindingIdent { id, type_ann }: &BindingIdent,
) -> anyhow::Result<(String, Typ)> {
    Ok((
        id.sym.to_string(),
        type_ann
            .as_ref()
            .map(|x| match &*x.type_ann {
                TsType::TsKeywordType(t) => match t.kind {
                    TsKeywordTypeKind::TsObjectKeyword => Typ::Dict,
                    TsKeywordTypeKind::TsBooleanKeyword => Typ::Bool,
                    TsKeywordTypeKind::TsBigIntKeyword => Typ::Int,
                    TsKeywordTypeKind::TsNumberKeyword => Typ::Float,
                    TsKeywordTypeKind::TsStringKeyword => Typ::Str,
                    _ => Typ::Unknown,
                },
                // TODO: we can do better here and extract the inner type of array
                TsType::TsArrayType(TsArrayType { span: _, elem_type }) => {
                    match &**elem_type {
                        TsType::TsTypeRef(TsTypeRef {
                            span: _,
                            type_name:
                                TsEntityName::Ident(Ident {
                                    span: _,
                                    sym,
                                    optional: _,
                                }),
                            type_params: _,
                        }) => match sym.to_string().as_str() {
                            "Base64" => Typ::List(InnerTyp::Bytes),
                            "Email" => Typ::List(InnerTyp::Email),
                            "bigint" => Typ::List(InnerTyp::Int),
                            "number" => Typ::List(InnerTyp::Float),
                            _ => Typ::List(InnerTyp::Str),
                        },
                        //TsType::TsKeywordType(())
                        _ => Typ::List(InnerTyp::Str),
                    }
                }
                TsType::TsTypeRef(TsTypeRef {
                    span: _,
                    type_name,
                    type_params,
                }) => {
                    let sym = match type_name {
                        TsEntityName::Ident(Ident {
                            span: _,
                            sym,
                            optional: _,
                        }) => sym,
                        TsEntityName::TsQualifiedName(p) => &*p.right.sym,
                    };
                    match sym.to_string().as_str() {
                        "Resource" => Typ::Resource(
                            type_params
                                .as_ref()
                                .and_then(|x| {
                                    x.params.get(0).and_then(|y| {
                                        y.as_ts_lit_type().and_then(|z| {
                                            z.lit.as_str().map(|a| a.to_owned().value.to_string())
                                        })
                                    })
                                })
                                .unwrap_or_else(|| "unknown".to_string()),
                        ),
                        "Base64" => Typ::Bytes,
                        "Email" => Typ::Email,
                        _ => Typ::Unknown,
                    }
                }
                _ => Typ::Unknown,
            })
            .unwrap_or(Typ::Unknown),
    ))
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

fn to_value(et: &ExpressionType) -> Option<serde_json::Value> {
    match et {
        ExpressionType::String {
            value: StringGroup::Constant { value },
        } => Some(json!(value)),
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

        ExpressionType::Call {
            function: _,
            args: _,
            keywords: _,
        } => Some(json!("<function call>")),

        _ => None,
    }
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
                            .collect::<Vec<String>>(),
                    ),
                    StatementType::ImportFrom {
                        level: _,
                        module: Some(mod_),
                        names: _,
                    } => Some(vec![mod_
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_string()
                        .replace("_", "-")]),
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

    #[test]
    fn test_parse_deno_sig() -> anyhow::Result<()> {
        let code = "

export function main(test1?: string, test2: string = \"burkina\",
    test3: wmill.Resource<'postgres'>, b64: Base64, ls: Base64[], email: Email) {
    console.log(42)
}

";
        println!("{}", serde_json::to_string(&parse_deno_signature(code)?)?);

        Ok(())
    }
}
