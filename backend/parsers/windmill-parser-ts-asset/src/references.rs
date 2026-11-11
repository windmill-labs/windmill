//! The workspace items TypeScript code names through the Windmill SDK, read from the same AST as
//! its assets.

use std::collections::{BTreeSet, HashMap};

use swc_ecma_ast::{
    CallExpr, Expr, ImportDecl, ImportSpecifier, Lit, MemberExpr, MemberProp, ModuleExportName,
    ModuleItem, Str, Tpl,
};
use swc_ecma_visit::{Visit, VisitWith};
use windmill_parser::item_reference::{ItemKind, ItemReference, ReferenceAccess};

use crate::object_str_prop;

enum Arg {
    Index(usize),
    /// A field of the object passed at this position.
    Field(usize, &'static str),
}

struct SdkCall {
    name: &'static str,
    kind: ItemKind,
    access: ReferenceAccess,
    arg: Arg,
}

const fn call(name: &'static str, kind: ItemKind, access: ReferenceAccess, arg: Arg) -> SdkCall {
    SdkCall { name, kind, access, arg }
}

use Arg::*;
use ItemKind::*;
use ReferenceAccess::*;

/// The SDK functions (`windmill-client`) that take an item path, and where.
const SDK_CALLS: &[SdkCall] = &[
    call("getResource", Resource, Read, Index(0)),
    call("setResource", Resource, Write, Index(1)),
    call("getVariable", Variable, Read, Index(0)),
    call("setVariable", Variable, Write, Index(0)),
    call("databaseUrlFromResource", Resource, Read, Index(0)),
    call("denoS3LightClientSettings", Resource, Read, Index(0)),
    call("duckdbConnectionSettings", Resource, Read, Index(0)),
    call("polarsConnectionSettings", Resource, Read, Index(0)),
    call("loadS3File", Resource, Read, Index(1)),
    call("loadS3FileStream", Resource, Read, Index(1)),
    call("writeS3File", Resource, Read, Index(2)),
    call(
        "requestInteractiveSlackApproval",
        Resource,
        Read,
        Field(0, "slackResourcePath"),
    ),
    call("runScript", Script, Run, Index(0)),
    call("runScriptAsync", Script, Run, Index(0)),
    call("runScriptByPath", Script, Run, Index(0)),
    call("runScriptByPathAsync", Script, Run, Index(0)),
    call("runFlow", Flow, Run, Index(0)),
    call("runFlowAsync", Flow, Run, Index(0)),
    call("taskScript", Script, Task, Index(0)),
    call("taskFlow", Flow, Task, Index(0)),
];

fn sdk_call(name: &str) -> Option<&'static SdkCall> {
    SDK_CALLS.iter().find(|c| c.name == name)
}

pub(crate) fn find_references(module: &[ModuleItem]) -> Vec<ItemReference> {
    let mut finder = ReferencesFinder { renamed: HashMap::new(), found: BTreeSet::new() };
    finder.visit_module_items(module);
    finder.found.into_iter().collect()
}

struct ReferencesFinder {
    /// Local names bound by `import { getVariable as gv }`.
    renamed: HashMap<String, &'static SdkCall>,
    found: BTreeSet<ItemReference>,
}

impl Visit for ReferencesFinder {
    fn visit_import_decl(&mut self, node: &ImportDecl) {
        for specifier in &node.specifiers {
            let ImportSpecifier::Named(named) = specifier else {
                continue;
            };
            let imported = match &named.imported {
                Some(ModuleExportName::Ident(i)) => i.sym.as_str(),
                Some(ModuleExportName::Str(s)) => s.value.as_str(),
                None => continue,
            };
            if let Some(call) = sdk_call(imported) {
                self.renamed.insert(named.local.sym.to_string(), call);
            }
        }
    }

    fn visit_call_expr(&mut self, node: &CallExpr) {
        let name = match node.callee.as_expr().map(AsRef::as_ref) {
            Some(Expr::Ident(i)) => Some(i.sym.as_str()),
            Some(Expr::Member(MemberExpr { prop: MemberProp::Ident(i), .. })) => {
                Some(i.sym.as_str())
            }
            _ => None,
        };
        let call = name.and_then(|n| self.renamed.get(n).copied().or_else(|| sdk_call(n)));
        if let Some(call) = call {
            let (index, field) = match call.arg {
                Index(i) => (i, None),
                Field(i, name) => (i, Some(name)),
            };
            let literal = node
                .args
                .get(index)
                .and_then(|a| match (a.expr.as_ref(), field) {
                    (Expr::Object(obj), Some(name)) => object_str_prop(obj, name),
                    (expr, None) => literal_value(expr),
                    _ => None,
                });
            if let Some(r) = literal.and_then(|l| ItemReference::of(call.kind, call.access, &l)) {
                self.found.insert(r);
            }
        }
        node.visit_children_with(self);
    }

    fn visit_str(&mut self, node: &Str) {
        self.found.extend(ItemReference::of_prefixed(&node.value));
    }

    fn visit_tpl(&mut self, node: &Tpl) {
        if let Some(value) = template_value(node) {
            self.found.extend(ItemReference::of_prefixed(&value));
        }
        node.visit_children_with(self);
    }
}

fn literal_value(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(Lit::Str(s)) => Some(s.value.to_string()),
        Expr::Tpl(t) => template_value(t),
        _ => None,
    }
}

/// A template literal's text when it interpolates nothing.
fn template_value(tpl: &Tpl) -> Option<String> {
    match (tpl.exprs.as_slice(), tpl.quasis.as_slice()) {
        ([], [quasi]) => Some(quasi.cooked.as_ref().unwrap_or(&quasi.raw).to_string()),
        _ => None,
    }
}
