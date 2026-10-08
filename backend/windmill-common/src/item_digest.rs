//! Content digests of deployed scripts and flows, reproducible with `wmill digest`
//! (`cli/src/commands/digest/digest.ts`) from a `wmill sync` checkout pulled after the
//! deployment's dependency jobs: they write the locks, and store loops and branches with
//! their defaults filled in. The two implementations must hash the same bytes: change one
//! only together with the other. Numbers hash as the doubles they parse to, which needs
//! serde_json's `float_roundtrip` to parse them as exactly as JavaScript does.
//!
//! A digest is the lowercase hex SHA-256 of the canonical JSON (RFC 8785) of the item, with
//! every object member whose value is null removed:
//! - a script: `{"content", "lock", "modules", "codebase"}`, where `modules` is the
//!   multi-file script's `{<relative path>: {"content", "lock"}}` and is left out when empty.
//!   The language is not covered: a checkout only records it in the file extension;
//! - a flow step's inline script: `{"content", "lock"}`;
//! - a flow: its `value`. A flow whose value references code in `flow_node` has none, see
//!   [`references_flow_nodes`].

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::flows::{Branch, FlowModule, FlowModuleValue, FlowValue, ToolValue};

pub fn digest(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    format!("{:x}", Sha256::digest(out.as_bytes()))
}

pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        // RFC 8785 serializes every number as an IEEE double, the way ECMAScript prints it.
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            out.push_str(ryu_js::Buffer::new().format(f))
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap_or_default()),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            // RFC 8785 orders members by the UTF-16 code units of their names.
            let mut members: Vec<_> = members.iter().filter(|(_, v)| !v.is_null()).collect();
            members.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, (k, v)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_canonical(v, out);
            }
            out.push('}');
        }
    }
}

/// Whether a flow value runs code stored outside it, in `flow_node` rows: an inline script
/// step `{"type": "flowscript", "id"}`, or a loop's or branch's steps in a `*_node`. Its digest
/// would not cover that code, and a checkout cannot reproduce it either, since the export
/// carries the references. A value or step that does not parse runs nothing, so it counts as
/// none.
pub fn references_flow_nodes(value: &Value) -> bool {
    let Ok(flow) = serde_json::from_value::<FlowValue>(value.clone()) else {
        return false;
    };
    flow.modules
        .iter()
        .chain(flow.preprocessor_module.as_deref())
        .chain(flow.failure_module.as_deref())
        .any(module_references_flow_nodes)
}

fn module_references_flow_nodes(module: &FlowModule) -> bool {
    module
        .get_value()
        .is_ok_and(|v| module_value_references_flow_nodes(&v))
}

fn module_value_references_flow_nodes(value: &FlowModuleValue) -> bool {
    let any = |modules: &[FlowModule]| modules.iter().any(module_references_flow_nodes);
    let branches = |branches: &[Branch]| {
        branches
            .iter()
            .any(|b| b.modules_node.is_some() || any(&b.modules))
    };
    match value {
        FlowModuleValue::FlowScript { .. } => true,
        FlowModuleValue::ForloopFlow { modules, modules_node, .. }
        | FlowModuleValue::WhileloopFlow { modules, modules_node, .. } => {
            modules_node.is_some() || any(modules)
        }
        FlowModuleValue::BranchOne { branches: b, default, default_node, .. } => {
            default_node.is_some() || any(default) || branches(b)
        }
        FlowModuleValue::BranchAll { branches: b, .. } => branches(b),
        FlowModuleValue::AIAgent { tools, .. } => tools.iter().any(|tool| match &tool.value {
            ToolValue::FlowModule(v) => module_value_references_flow_nodes(v),
            _ => false,
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_form_follows_rfc_8785_without_nulls() {
        let v = json!({"b": [1.0, 1e21, 0.1, -0.0, null], "a": null, "é": "\u{1f}\"\n", "\u{e000}": 1, "\u{1f600}": 2});
        assert_eq!(
            canonical_json(&v),
            "{\"b\":[1,1e+21,0.1,0,null],\"é\":\"\\u001f\\\"\\n\",\"\u{1f600}\":2,\"\u{e000}\":1}"
        );
    }
}
