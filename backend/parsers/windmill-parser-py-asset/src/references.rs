//! The workspace items Python code names through the Windmill SDK, read from the same AST as its
//! assets.

use std::collections::{BTreeSet, HashMap};

use ruff_python_ast::{
    visitor::{walk_expr, walk_stmt, Visitor},
    Expr, ExprCall, Stmt,
};
use windmill_parser::item_reference::{ItemKind, ItemReference, ReferenceAccess};

struct SdkCall {
    name: &'static str,
    kind: ItemKind,
    access: ReferenceAccess,
    /// The positions that can hold the path, tried in order. `set_resource` lists two: the
    /// module function takes the path first, a `Windmill` client method second.
    args: &'static [usize],
    keywords: &'static [&'static str],
}

const fn call(
    name: &'static str,
    kind: ItemKind,
    access: ReferenceAccess,
    args: &'static [usize],
    keywords: &'static [&'static str],
) -> SdkCall {
    SdkCall { name, kind, access, args, keywords }
}

use ItemKind::*;
use ReferenceAccess::*;

const PATH: &[&str] = &["path"];
const S3_RESOURCE: &[&str] = &["s3_resource_path"];

/// The SDK functions (`wmill`) that take an item path, and where.
const SDK_CALLS: &[SdkCall] = &[
    call("get_variable", Variable, Read, &[0], PATH),
    call("set_variable", Variable, Write, &[0], PATH),
    call("get_resource", Resource, Read, &[0], PATH),
    call("set_resource", Resource, Write, &[0, 1], PATH),
    call("load_s3_file", Resource, Read, &[1], S3_RESOURCE),
    call("load_s3_file_reader", Resource, Read, &[1], S3_RESOURCE),
    call("write_s3_file", Resource, Read, &[2], S3_RESOURCE),
    call(
        "duckdb_connection_settings",
        Resource,
        Read,
        &[0],
        S3_RESOURCE,
    ),
    call(
        "polars_connection_settings",
        Resource,
        Read,
        &[0],
        S3_RESOURCE,
    ),
    call(
        "boto3_connection_settings",
        Resource,
        Read,
        &[0],
        S3_RESOURCE,
    ),
    call(
        "get_duckdb_connection_settings",
        Resource,
        Read,
        &[0],
        S3_RESOURCE,
    ),
    call(
        "get_polars_connection_settings",
        Resource,
        Read,
        &[0],
        S3_RESOURCE,
    ),
    call(
        "get_boto3_connection_settings",
        Resource,
        Read,
        &[0],
        S3_RESOURCE,
    ),
    call(
        "request_interactive_slack_approval",
        Resource,
        Read,
        &[0],
        &["slack_resource_path"],
    ),
    call("run_script", Script, Run, &[0], PATH),
    call("run_script_by_path", Script, Run, &[0], PATH),
    call(
        "run_script_async",
        Script,
        Run,
        &[0],
        &["path", "hash_or_path"],
    ),
    call("run_script_by_path_async", Script, Run, &[0], PATH),
    call("run_script_by_path_sync", Script, Run, &[0], PATH),
    call("run_flow_async", Flow, Run, &[0], PATH),
    call("task_script", Script, Task, &[0], PATH),
    call("task_flow", Flow, Task, &[0], PATH),
];

fn sdk_call(name: &str) -> Option<&'static SdkCall> {
    SDK_CALLS.iter().find(|c| c.name == name)
}

pub(crate) fn find_references(suite: &[Stmt]) -> Vec<ItemReference> {
    let mut finder = ReferencesFinder { renamed: HashMap::new(), found: BTreeSet::new() };
    finder.visit_body(suite);
    finder.found.into_iter().collect()
}

struct ReferencesFinder {
    /// Local names bound by `from wmill import get_variable as gv`.
    renamed: HashMap<String, &'static SdkCall>,
    found: BTreeSet<ItemReference>,
}

impl<'a> Visitor<'a> for ReferencesFinder {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::ImportFrom(node) = stmt {
            for alias in &node.names {
                if let (Some(local), Some(call)) = (&alias.asname, sdk_call(alias.name.as_str())) {
                    self.renamed.insert(local.to_string(), call);
                }
            }
        }
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(node) => self.record_call(node),
            Expr::StringLiteral(s) => self
                .found
                .extend(ItemReference::of_prefixed(s.value.to_str())),
            _ => {}
        }
        walk_expr(self, expr);
    }
}

impl ReferencesFinder {
    fn record_call(&mut self, node: &ExprCall) {
        let name = node
            .func
            .as_name_expr()
            .map(|n| n.id.as_str())
            .or_else(|| node.func.as_attribute_expr().map(|a| a.attr.as_str()));
        let Some(call) = name.and_then(|n| self.renamed.get(n).copied().or_else(|| sdk_call(n)))
        else {
            return;
        };
        let keyword = node.arguments.keywords.iter().find(|k| {
            k.arg
                .as_ref()
                .is_some_and(|a| call.keywords.contains(&a.as_str()))
        });
        let candidates = call
            .args
            .iter()
            .filter_map(|i| node.arguments.args.get(*i))
            .chain(keyword.map(|k| &k.value));
        for candidate in candidates {
            let Expr::StringLiteral(s) = candidate else {
                continue;
            };
            if let Some(r) = ItemReference::of(call.kind, call.access, s.value.to_str()) {
                self.found.insert(r);
                return;
            }
        }
    }
}
