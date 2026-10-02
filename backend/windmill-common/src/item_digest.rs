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
//! - a flow: its `value`, with any step stored by reference in `flow_node` inlined back.

use std::collections::HashMap;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{db::DB, error::Result};

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

/// Inlines every step a flow value stores by reference in `flow_node` (an inline script
/// `{"type": "flowscript", "id"}`, or a `modules_node`/`default_node` holding a loop's or
/// branch's steps), so that the digest covers their code.
pub async fn inline_flow_nodes(db: &DB, w_id: &str, value: &mut Value) -> Result<()> {
    // A node's steps can reference further nodes.
    for _ in 0..100 {
        let mut ids = vec![];
        collect_node_ids(value, &mut ids);
        if ids.is_empty() {
            return Ok(());
        }
        let nodes = sqlx::query!(
            "SELECT id, code, lock, flow FROM flow_node WHERE id = ANY($1) AND workspace_id = $2",
            &ids,
            w_id
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|n| (n.id, (n.code, n.lock, n.flow)))
        .collect::<HashMap<_, _>>();
        inline_nodes(value, &nodes);
    }
    Ok(())
}

const NODE_KEYS: [(&str, &str); 2] = [("modules_node", "modules"), ("default_node", "default")];

fn collect_node_ids(value: &Value, ids: &mut Vec<i64>) {
    match value {
        Value::Object(o) => {
            if o.get("type").and_then(Value::as_str) == Some("flowscript") {
                ids.extend(o.get("id").and_then(Value::as_i64));
            }
            for (key, _) in NODE_KEYS {
                ids.extend(o.get(key).and_then(Value::as_i64));
            }
            o.values().for_each(|v| collect_node_ids(v, ids));
        }
        Value::Array(a) => a.iter().for_each(|v| collect_node_ids(v, ids)),
        _ => {}
    }
}

type Node = (Option<String>, Option<String>, Option<Value>);

fn inline_nodes(value: &mut Value, nodes: &HashMap<i64, Node>) {
    match value {
        Value::Object(o) => {
            if o.get("type").and_then(Value::as_str) == Some("flowscript") {
                let id = o.remove("id").and_then(|id| id.as_i64());
                o.insert("type".into(), "rawscript".into());
                let (code, lock) = id
                    .and_then(|id| nodes.get(&id))
                    .map(|(code, lock, _)| (code.clone(), lock.clone()))
                    .unwrap_or_default();
                o.insert("content".into(), code.unwrap_or_default().into());
                o.insert("lock".into(), lock.into());
            }
            for (key, target) in NODE_KEYS {
                if let Some(id) = o.remove(key) {
                    let modules = id
                        .as_i64()
                        .and_then(|id| nodes.get(&id))
                        .and_then(|(_, _, flow)| flow.as_ref()?.get("modules").cloned());
                    o.insert(target.into(), modules.unwrap_or(Value::Array(vec![])));
                }
            }
            o.values_mut().for_each(|v| inline_nodes(v, nodes));
        }
        Value::Array(a) => a.iter_mut().for_each(|v| inline_nodes(v, nodes)),
        _ => {}
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
