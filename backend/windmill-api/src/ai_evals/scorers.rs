use super::*;

/// A scorer is a column of the results table.
///
/// `id` is assigned when the scorer is added to a dataset and never reused: it is what makes a
/// column the same column across experiments when the scorer is renamed or its definition is
/// edited, and a delta is only ever computed between two scores carrying the same id.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Scorer {
    /// Assigned on write when a new scorer arrives without one, so a client cannot collide two
    /// columns onto one id.
    #[serde(default)]
    pub id: String,
    /// The column header. Defaults to the kind, or the last segment of the path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A score at or above this counts as a pass. Deliberately outside `definition`: where the
    /// line sits interprets a score rather than produces it, so moving it re-reads every score
    /// already recorded instead of invalidating them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_if: Option<f64>,
    #[serde(flatten)]
    pub def: ScorerDef,
}

/// A judge is an `ai_agent` resource sent the run to grade; a script receives the run as an
/// argument. Both are runnables, so every column has a path, a version and code you can open.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScorerDef {
    Script { path: String },
    Agent { path: String },
}

impl ScorerDef {
    pub fn path(&self) -> &str {
        match self {
            ScorerDef::Script { path } | ScorerDef::Agent { path } => path,
        }
    }

    /// The wire name of the kind, as the client sends it.
    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            ScorerDef::Script { .. } => "script",
            ScorerDef::Agent { .. } => "agent",
        }
    }

    fn kind_label(&self) -> &'static str {
        match self {
            ScorerDef::Script { .. } => "Script",
            ScorerDef::Agent { .. } => "Judge agent",
        }
    }
}

impl Scorer {
    /// Whether a score counts as a pass. `None` when the column has no threshold, which keeps a
    /// column of plain numbers from being rendered as if it had one.
    pub fn passed(&self, score: Option<f64>) -> Option<bool> {
        match (self.pass_if, score) {
            (Some(threshold), Some(score)) => Some(score >= threshold),
            _ => None,
        }
    }

    /// What produced a score, recorded with it so a comparison can say the scorer changed instead
    /// of letting that read as a difference between two agents. `resolved` is the script hash or
    /// resource version that actually ran, which the path alone does not pin.
    pub fn definition(&self, resolved: Option<&str>) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.def.kind_label().as_bytes());
        hasher.update(b":");
        hasher.update(self.def.path().as_bytes());
        if let Some(resolved) = resolved {
            hasher.update(b"@");
            hasher.update(resolved.as_bytes());
        }
        hex::encode(hasher.finalize())[..32].to_string()
    }
}

const MAX_SCORER_NAME_CHARS: usize = 120;

/// Ids are assigned here rather than trusted from the client: an id is kept only when it names a
/// column the dataset already has, so a removed column cannot come back and inherit the scores
/// recorded against it. Anything else is minted as a valid flow module identifier, which the
/// scoring flows it is baked into require (see `scorer_module_id`).
pub(crate) fn assign_scorer_ids(
    scorers: &mut Vec<Scorer>,
    existing: &std::collections::HashSet<String>,
) -> Result<()> {
    if scorers.len() > MAX_SCORERS_PER_DATASET {
        return Err(Error::BadRequest(format!(
            "An eval dataset holds at most {} scorers",
            MAX_SCORERS_PER_DATASET
        )));
    }
    let mut seen = std::collections::HashSet::new();
    for scorer in scorers.iter_mut() {
        if !existing.contains(&scorer.id) || !seen.insert(scorer.id.clone()) {
            scorer.id = Uuid::new_v4().simple().to_string();
            seen.insert(scorer.id.clone());
        }
        if let Some(name) = &scorer.name {
            if name.chars().count() > MAX_SCORER_NAME_CHARS {
                return Err(Error::BadRequest(format!(
                    "Scorer name {} is too long, {} characters at most",
                    name, MAX_SCORER_NAME_CHARS
                )));
            }
        }
        // A score is 0 to 1, so a threshold outside that range would pass everything or nothing
        // regardless of what the scorer measured.
        if let Some(pass_if) = scorer.pass_if {
            if !(0.0..=1.0).contains(&pass_if) {
                return Err(Error::BadRequest(format!(
                    "Scorer pass threshold {} must be between 0 and 1",
                    pass_if
                )));
            }
        }
        check_proper_path(scorer.def.path())?;
    }
    Ok(())
}

/// What a column is called: the dataset's own name for it, or the last segment of what it points
/// at. The same fallback the column header uses.
pub(crate) fn scorer_name(scorer: &Scorer) -> String {
    scorer
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            let path = scorer.def.path();
            path.rsplit('/').next().unwrap_or(path).to_string()
        })
}

#[derive(Serialize)]
pub struct RecentScorer {
    #[serde(flatten)]
    pub scorer: Scorer,
    /// The dataset it is a column of, which is where the user last saw it.
    pub dataset: String,
}

#[derive(Deserialize)]
pub struct RecentScorersQuery {
    /// Only scorers of this kind, which is the one the add form was opened for.
    #[serde(default)]
    pub kind: Option<String>,
}

/// The scorers already in use in this workspace, most recently edited dataset first.
///
/// Filtered twice through `user_db`: a scorer appears only if its dataset does, and the runnable
/// is checked the same way, so the list is scorers the caller could actually run.
pub async fn recent_scorers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<RecentScorersQuery>,
) -> JsonResult<Vec<RecentScorer>> {
    let mut tx = user_db.begin(&authed).await?;
    let datasets = sqlx::query!(
        "SELECT path, scorers FROM eval_dataset
         WHERE workspace_id = $1 ORDER BY edited_at DESC LIMIT 100",
        w_id
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut seen = std::collections::HashSet::new();
    let mut recent: Vec<RecentScorer> = vec![];
    for row in datasets {
        for scorer in parse_scorers(row.scorers)? {
            if query
                .kind
                .as_deref()
                .is_some_and(|kind| kind != scorer.def.kind_str())
            {
                continue;
            }
            let key = (scorer.def.kind_str(), scorer.def.path().to_string());
            if seen.insert(key) {
                recent.push(RecentScorer { scorer, dataset: row.path.clone() });
            }
        }
    }
    // Readability is resolved over every candidate, then the list is cut: an unreadable scorer must
    // not take a slot a readable one further down would have filled.
    let script_paths = recent
        .iter()
        .filter(|r| matches!(r.scorer.def, ScorerDef::Script { .. }))
        .map(|r| r.scorer.def.path().to_string())
        .collect::<Vec<_>>();
    let agent_paths = recent
        .iter()
        .filter(|r| matches!(r.scorer.def, ScorerDef::Agent { .. }))
        .map(|r| r.scorer.def.path().to_string())
        .collect::<Vec<_>>();
    // Same deployed-version predicate as get_latest_script_hash: a script with no successfully
    // locked version can't be resolved at launch, so it must not offer itself as a scorer here.
    let readable_scripts = sqlx::query_scalar!(
        "SELECT DISTINCT path FROM script
         WHERE workspace_id = $1 AND path = ANY($2)
           AND deleted = false AND lock IS NOT NULL AND lock_error_logs IS NULL",
        w_id,
        &script_paths
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    let readable_agents = sqlx::query_scalar!(
        "SELECT path FROM resource WHERE workspace_id = $1 AND path = ANY($2) AND resource_type = 'ai_agent'",
        w_id,
        &agent_paths
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect::<std::collections::HashSet<_>>();
    tx.commit().await?;

    recent.retain(|r| match &r.scorer.def {
        ScorerDef::Script { path } => readable_scripts.contains(path),
        ScorerDef::Agent { path } => readable_agents.contains(path),
    });
    recent.truncate(MAX_RECENT_SCORERS);
    Ok(Json(recent))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The definition hash is what tells a comparison that the scorer changed; the path alone
    /// would miss an edit to the script itself.
    #[test]
    fn definition_moves_with_the_runnable_and_not_with_its_name() {
        let script = |path: &str, name: Option<&str>| Scorer {
            id: "s1".to_string(),
            name: name.map(|n| n.to_string()),
            pass_if: None,
            def: ScorerDef::Script { path: path.to_string() },
        };
        // Renaming a column is not a change of scorer: same runnable, same version.
        assert_eq!(
            script("f/e/s", None).definition(Some("1234")),
            script("f/e/s", Some("Tool discipline")).definition(Some("1234"))
        );
        // Same script, newly deployed: the column says the scorer changed.
        assert_ne!(
            script("f/e/s", None).definition(Some("1234")),
            script("f/e/s", None).definition(Some("5678"))
        );
        // A judge agent and a script sharing a path are not the same column.
        let agent = Scorer {
            id: "s1".to_string(),
            name: None,
            pass_if: None,
            def: ScorerDef::Agent { path: "f/e/s".to_string() },
        };
        assert_ne!(
            agent.definition(Some("1")),
            script("f/e/s", None).definition(Some("1"))
        );
        // If the pass line entered the hash, setting a threshold would mark every score already
        // recorded as coming from a different scorer.
        let mut thresholded = script("f/e/s", None);
        thresholded.pass_if = Some(0.7);
        assert_eq!(
            thresholded.definition(Some("1234")),
            script("f/e/s", None).definition(Some("1234"))
        );
        assert_eq!(thresholded.passed(Some(0.7)), Some(true));
        assert_eq!(thresholded.passed(Some(0.69)), Some(false));
        assert_eq!(script("f/e/s", None).passed(Some(0.1)), None);
    }
}
