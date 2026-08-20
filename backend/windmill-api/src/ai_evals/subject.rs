use super::*;


/// What a run is executed against. Kept as `(kind, path, version)` rather than a bare agent
/// path so flow-scoped evaluation is a later superset instead of a rewrite.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EvalSubject {
    #[serde(default = "default_subject_kind")]
    pub kind: EvalSubjectKind,
    /// The agent resource under test.
    pub path: String,
    /// Which version of the agent, counted per path: how many times it had been saved. For a
    /// pinned run this says what to inline and is the request's to choose; otherwise it is the
    /// version the run was enqueued against, recorded so the run stays attributable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    /// The agent's undeployed configuration, read from its draft server-side. Present exactly
    /// when `kind` is `agent_draft`, and it is the whole definition of what ran.
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
    /// A saved agent's undeployed draft. The value is read server-side and inlined, because a
    /// linked step resolves the resource live and so would run what the draft replaces. Its own
    /// subject, so its runs never mix with the deployed agent's history.
    AgentDraft,
    /// One past version of a saved agent, read out of its history and inlined the same way a draft
    /// is — a linked step resolves the resource live, so pinning is exactly what it cannot
    /// express. `version` says which, and it is the request's rather than the server's here: it is
    /// the whole content of the choice.
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
    /// What is recorded of a subject: enough to say what ran, without the configuration itself,
    /// which is large and already described by its hash.
    pub(crate) fn stamp(&self) -> EvalSubject {
        EvalSubject {
            kind: self.kind.clone(),
            path: self.path.clone(),
            version: self.version,
            draft: None,
            // Kept when there is no configuration to hash: a subject read back from a run's own
            // stamp carries the hash and not the draft, and recomputing would erase it.
            draft_hash: self
                .draft
                .as_ref()
                .map(draft_hash)
                .or_else(|| self.draft_hash.clone()),
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
    /// What its draft hashes to now, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_hash: Option<String>,
    pub has_undeployed_changes: bool,
}


/// What the agent is right now: the version it is deployed at, and what its draft hashes to.
///
/// Small on purpose. The results endpoint reports the same thing, but it harvests scores and
/// reads every job to do it, so it is not something to poll — and without polling, an agent
/// edited while the table is open goes on looking current until the pane is reopened.
pub async fn subject_state(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<SubjectStateQuery>,
) -> JsonResult<SubjectState> {
    // Reading the agent through `user_db` is what gates the rest: a draft's existence is
    // information about the resource it is a draft of.
    require_agent(&authed, &user_db, &w_id, &query.path).await?;
    let version = current_resource_version(&db, &w_id, &query.path).await?;
    let draft = agent_draft_config(&authed, &user_db, &w_id, &query.path)
        .await
        .ok();
    Ok(Json(SubjectState {
        version,
        draft_hash: draft.as_ref().map(draft_hash),
        has_undeployed_changes: draft.is_some(),
    }))
}

// -----------------------------------------------------------------------------------------------
// Scoring
// -----------------------------------------------------------------------------------------------
