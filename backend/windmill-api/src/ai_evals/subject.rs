use super::*;

/// What a run is executed against. Kept as `(kind, path, version)` rather than a bare agent
/// path so flow-scoped evaluation is a later superset instead of a rewrite.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalSubject {
    #[serde(default = "default_subject_kind")]
    pub kind: EvalSubjectKind,
    /// The agent resource under test.
    pub path: String,
    /// Which version of the agent, counted per path: how many times it had been saved. The
    /// request's to choose for a pinned run, and otherwise the version the run was enqueued
    /// against.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// The agent's unsaved edits, as the editor holds them. Present exactly when `kind` is
    /// `agent_draft`, since the edits exist nowhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<AgentDraft>,
    /// Hash of that configuration. A draft moves without the version moving, so this is the only
    /// thing that can say a run describes an agent that has since been edited. Stamped
    /// server-side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_hash: Option<String>,
}

/// Key order is not meaningful and `serde_json` preserves insertion order here, so it is sorted
/// away before hashing: the same configuration must hash the same however it was assembled.
fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let sorted = map
                .iter()
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_default(),
                        canonical_json(v)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{}}}", sorted)
        }
        serde_json::Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => other.to_string(),
    }
}

pub(crate) fn draft_hash(draft: &AgentDraft) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(canonical_json(&draft.input_transforms).as_bytes());
    hasher.update(b"|");
    hasher.update(canonical_json(&serde_json::Value::Array(draft.tools.clone())).as_bytes());
    hex::encode(hasher.finalize())[..32].to_string()
}

fn default_subject_kind() -> EvalSubjectKind {
    EvalSubjectKind::Agent
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvalSubjectKind {
    Agent,
    /// A saved agent's unsaved edits, carried by the request and inlined: a linked step resolves
    /// the resource live and so would run what the edits replace.
    AgentDraft,
    /// One past version of a saved agent, inlined for the same reason. `version` says which, and
    /// it is the request's to choose rather than the server's.
    AgentVersion,
}

/// The brain and tools of an agent, as the flow editor holds them.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentDraft {
    /// The agent's input transforms: provider, system prompt, output type and the rest. The
    /// message and attachments are supplied by the case and override anything named here.
    #[serde(default)]
    pub input_transforms: serde_json::Value,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
}

impl EvalSubject {
    /// What is recorded of a subject: enough to say what ran, without the configuration itself.
    pub(crate) fn stamp(&self) -> EvalSubject {
        EvalSubject {
            kind: self.kind.clone(),
            path: self.path.clone(),
            version: self.version,
            draft: None,
            // Only ever derived from the draft this request carries: a hash the client supplies on
            // its own could relabel a run as the deployed version.
            draft_hash: self.draft.as_ref().map(draft_hash),
        }
    }
}

#[derive(Deserialize)]
pub struct SubjectStateQuery {
    pub path: String,
}

#[derive(Serialize)]
pub struct SubjectState {
    /// The version the agent is on now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
}

/// The version the agent is deployed at. Small on purpose: the results endpoint reports the same
/// thing, but it harvests scores and reads every job to do it.
pub async fn subject_state(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<SubjectStateQuery>,
) -> JsonResult<SubjectState> {
    let Some((_, version)) = readable_agent_state(&authed, &user_db, &w_id, &query.path).await?
    else {
        return Err(Error::NotFound(format!("Agent {} not found", query.path)));
    };
    Ok(Json(SubjectState { version: Some(version) }))
}
