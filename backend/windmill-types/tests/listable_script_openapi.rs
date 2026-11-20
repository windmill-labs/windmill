//! `openapi.yaml` describes the `/scripts/list` row by hand, so a field added to
//! `ListableScript` must also be added to its `ListableScript` schema, or every
//! generated client silently lacks it.

use std::collections::BTreeSet;

use serde_json::Value;
use windmill_types::scripts::{ListableScript, ScriptHash, ScriptKind, ScriptLang};
use windmill_types::user_drafts::DraftUserRef;

fn listable_script(populated: bool) -> ListableScript {
    let some = |s: &str| populated.then(|| s.to_string());
    ListableScript {
        hash: ScriptHash(1),
        path: "f/a/b".to_string(),
        summary: String::new(),
        created_at: chrono::Utc::now(),
        archived: false,
        extra_perms: serde_json::json!({}),
        language: ScriptLang::Bun,
        starred: false,
        tag: None,
        description: some("d"),
        draft_only: populated.then_some(true),
        has_deploy_errors: false,
        ws_error_handler_muted: None,
        auto_kind: some("lib"),
        use_codebase: populated,
        deployment_msg: some("m"),
        kind: ScriptKind::Script,
        labels: populated.then(|| vec!["l".to_string()]),
        is_draft: populated,
        draft_path: some("u/x/y"),
        draft_users: populated.then(|| sqlx::types::Json(vec![DraftUserRef { username: None }])),
        inherited_labels: populated.then(|| vec!["l".to_string()]),
    }
}

fn keys(script: &ListableScript) -> BTreeSet<String> {
    match serde_json::to_value(script).unwrap() {
        Value::Object(o) => o.into_iter().map(|(k, _)| k).collect(),
        v => panic!("ListableScript serialized to {v}"),
    }
}

#[test]
fn listable_script_schema_matches_its_serialization() {
    let spec: serde_yml::Value = serde_yml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../windmill-api/openapi.yaml"
    )))
    .unwrap();
    let schema = &spec["components"]["schemas"]["ListableScript"];
    let names = |v: &serde_yml::Value| -> BTreeSet<String> {
        match v {
            serde_yml::Value::Mapping(m) => {
                m.keys().map(|k| k.as_str().unwrap().to_string()).collect()
            }
            serde_yml::Value::Sequence(s) => {
                s.iter().map(|k| k.as_str().unwrap().to_string()).collect()
            }
            v => panic!("unexpected {v:?}"),
        }
    };
    let properties = names(&schema["properties"]);
    let required = names(&schema["required"]);

    assert_eq!(
        keys(&listable_script(true)),
        properties,
        "ListableScript's serialized fields and the properties of its openapi.yaml schema differ"
    );
    assert_eq!(
        keys(&listable_script(false)),
        required,
        "the fields ListableScript always serializes and the `required` list of its openapi.yaml schema differ"
    );
}
