//! The `min_job_isolation_by_tag` instance setting: a tag can demand a minimum isolation, and a
//! worker below it pulls none of that tag's jobs, which stay queued. It guards against a worker
//! group configured with less isolation than its tags need; it is not a defense against a worker
//! that lies about its isolation.
//!
//! The `agent_only_tags` instance setting rides on the same filter: a worker that is not an agent
//! worker (so one holding database credentials) pulls none of those tags' jobs. Like the minimum,
//! it guards against a worker group listing a tag it should not; a modified binary holding
//! database credentials can already read every job and secret, which only deployment prevents.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;

use crate::utils::{Mode, MODE_AND_ADDONS};
use crate::worker::{WorkerConfig, DEV_WORKSPACE_TAG_SUFFIX, NATIVE_MODE_RESOLVED};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum IsolationLevel {
    None = 0,
    Unshare = 1,
    Nsjail = 2,
}

impl IsolationLevel {
    /// Reads both the setting's values and what a worker reports in `worker_ping.job_isolation`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Self::None),
            "unshare" => Some(Self::Unshare),
            "nsjail" => Some(Self::Nsjail),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Unshare => "unshare",
            Self::Nsjail => "nsjail",
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            2 => Self::Nsjail,
            1 => Self::Unshare,
            _ => Self::None,
        }
    }

    /// The isolation a worker's jobs actually get from the one it reports. Native-mode and
    /// dedicated workers run jobs outside the sandbox whatever they report, so they count as
    /// `None`, as does anything unreported or unrecognized: it can only cost a worker jobs.
    pub fn effective(reported: Option<&str>, outside_sandbox: bool) -> Self {
        if outside_sandbox {
            return Self::None;
        }
        reported.and_then(Self::parse).unwrap_or(Self::None)
    }
}

pub type MinJobIsolationByTag = HashMap<String, IsolationLevel>;

lazy_static::lazy_static! {
    pub static ref MIN_JOB_ISOLATION_BY_TAG: arc_swap::ArcSwap<MinJobIsolationByTag> =
        arc_swap::ArcSwap::from_pointee(HashMap::new());
    pub static ref AGENT_ONLY_TAGS: arc_swap::ArcSwap<Vec<String>> =
        arc_swap::ArcSwap::from_pointee(vec![]);
    static ref LAST_REFUSED: Mutex<Option<BTreeMap<String, Refusal>>> = Mutex::new(None);
}

/// Why a worker refuses a tag. An agent-only tag is reported as such even when its minimum
/// isolation is also unmet, since more isolation would not lift the refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    AgentOnly,
    Isolation(IsolationLevel),
}

impl Refusal {
    fn message(&self, tag: &str, own: IsolationLevel) -> String {
        match self {
            Self::AgentOnly => format!("Tag {tag} requires an agent worker"),
            Self::Isolation(min) => format!(
                "Tag {tag} requires job isolation {} but this worker's isolation is {}",
                min.as_str(),
                own.as_str()
            ),
        }
    }
}

/// Agent workers reach the database only through the API: agent mode never opens a database
/// connection, so a process in any other mode is one that holds database credentials.
fn is_agent_worker() -> bool {
    MODE_AND_ADDONS.mode == Mode::Agent
}

/// The sandbox this worker applies, set by the worker whenever its isolation settings change.
/// It starts at `None`, so a worker that never learned its level refuses rather than over-pulls.
static WORKER_SANDBOX_ISOLATION: AtomicU8 = AtomicU8::new(IsolationLevel::None as u8);

pub fn set_worker_sandbox_isolation(level: IsolationLevel) {
    WORKER_SANDBOX_ISOLATION.store(level as u8, Ordering::Relaxed);
}

/// See [`IsolationLevel::effective`].
pub fn worker_job_isolation(wc: &WorkerConfig) -> IsolationLevel {
    if NATIVE_MODE_RESOLVED.load(Ordering::Relaxed)
        || wc.native_mode
        || wc.dedicated_worker.is_some()
        || wc.dedicated_workers.as_ref().is_some_and(|d| !d.is_empty())
    {
        return IsolationLevel::None;
    }
    IsolationLevel::from_u8(WORKER_SANDBOX_ISOLATION.load(Ordering::Relaxed))
}

/// The settings API and config sync both reject a malformed value at write time: the loader can
/// only ignore a value that is not a map, which turns the guard off.
pub fn validate_min_job_isolation_by_tag(value: Option<&serde_json::Value>) -> Result<(), String> {
    let map = match value {
        None | Some(serde_json::Value::Null) => return Ok(()),
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => return Ok(()),
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => return Err("must be an object mapping tags to unshare or nsjail".to_string()),
    };
    for (tag, level) in map {
        if tag.trim().is_empty() {
            return Err("tags must not be empty".to_string());
        }
        if !matches!(level.as_str(), Some("unshare" | "nsjail")) {
            return Err(format!(
                "the minimum for tag {tag} must be unshare or nsjail, got {level}"
            ));
        }
    }
    Ok(())
}

/// An unrecognized level is read as the strictest one: a typo in the setting must not let a
/// tag's jobs run with less isolation than intended.
pub fn parse_min_job_isolation_by_tag(value: Option<serde_json::Value>) -> MinJobIsolationByTag {
    let map = match value {
        Some(serde_json::Value::Object(map)) => map,
        None | Some(serde_json::Value::Null) => return HashMap::new(),
        Some(other) => {
            tracing::error!(
                "min_job_isolation_by_tag must map tags to levels, ignoring the value {other}"
            );
            return HashMap::new();
        }
    };
    map.into_iter()
        .map(|(tag, level)| {
            let parsed = level.as_str().and_then(IsolationLevel::parse);
            if parsed.is_none() {
                tracing::error!(
                    "min_job_isolation_by_tag: unrecognized level {level} for tag {tag}, requiring nsjail"
                );
            }
            (tag, parsed.unwrap_or(IsolationLevel::Nsjail))
        })
        .collect()
}

/// Same write-time check as [`validate_min_job_isolation_by_tag`], for `agent_only_tags`.
pub fn validate_agent_only_tags(value: Option<&serde_json::Value>) -> Result<(), String> {
    let tags = match value {
        None | Some(serde_json::Value::Null) => return Ok(()),
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => return Ok(()),
        Some(serde_json::Value::Array(tags)) => tags,
        Some(_) => return Err("must be a list of tags".to_string()),
    };
    if tags
        .iter()
        .any(|t| !t.as_str().is_some_and(|t| !t.trim().is_empty()))
    {
        return Err("tags must be non-empty strings".to_string());
    }
    Ok(())
}

pub fn parse_agent_only_tags(value: Option<serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(tags)) => tags
            .into_iter()
            .filter_map(|t| match t {
                serde_json::Value::String(s) if !s.trim().is_empty() => Some(s),
                other => {
                    tracing::error!("agent_only_tags: ignoring the entry {other}, not a tag");
                    None
                }
            })
            .collect(),
        None | Some(serde_json::Value::Null) => vec![],
        Some(other) => {
            tracing::error!("agent_only_tags must be a list of tags, ignoring the value {other}");
            vec![]
        }
    }
}

/// Per-workspace (`<tag>-<workspace>`), fork and dev-workspace tags extend the configured tag
/// with `-` or the dev workspace suffix, so any tag of that shape is covered. That also catches
/// a custom tag like `<tag>-gpu`, which can only refuse more.
fn derives_from(tag: &str, base: &str, dev_suffix: &str) -> bool {
    tag.strip_prefix(base).is_some_and(|rest| {
        rest.is_empty() || rest.starts_with('-') || rest.starts_with(dev_suffix)
    })
}

/// The minimum `tag` is held to: the strictest among the configured tags it is or derives from.
pub fn min_for_tag(tag: &str, mins: &MinJobIsolationByTag) -> Option<IsolationLevel> {
    if mins.is_empty() {
        return None;
    }
    let dev_suffix = DEV_WORKSPACE_TAG_SUFFIX.load();
    mins.iter()
        .filter(|(base, _)| derives_from(tag, base, &dev_suffix))
        .map(|(_, min)| *min)
        .max()
}

/// Whether `tag` is or derives from one of `agent_only`, by the rule of [`min_for_tag`].
pub fn is_agent_only(tag: &str, agent_only: &[String]) -> bool {
    if agent_only.is_empty() {
        return false;
    }
    let dev_suffix = DEV_WORKSPACE_TAG_SUFFIX.load();
    agent_only
        .iter()
        .any(|base| derives_from(tag, base, &dev_suffix))
}

/// The tags among `tags` whose minimum exceeds `own`, with the minimum each requires.
pub fn refused_tags(
    tags: &[String],
    own: IsolationLevel,
    mins: &MinJobIsolationByTag,
) -> BTreeMap<String, IsolationLevel> {
    tags.iter()
        .filter_map(|t| {
            min_for_tag(t, mins)
                .filter(|min| *min > own)
                .map(|min| (t.clone(), min))
        })
        .collect()
}

pub fn allowed_tags(
    tags: &[String],
    own: IsolationLevel,
    mins: &MinJobIsolationByTag,
) -> Vec<String> {
    tags.iter()
        .filter(|t| min_for_tag(t, mins).map_or(true, |min| min <= own))
        .cloned()
        .collect()
}

/// Why a worker refuses `tag`, if it does; `agent` is whether it is an agent worker. Returns at
/// once while neither setting names any tag.
fn refusal_for(
    tag: &str,
    own: IsolationLevel,
    mins: &MinJobIsolationByTag,
    agent_only: &[String],
    agent: bool,
) -> Option<Refusal> {
    if !agent && is_agent_only(tag, agent_only) {
        return Some(Refusal::AgentOnly);
    }
    min_for_tag(tag, mins)
        .filter(|min| *min > own)
        .map(Refusal::Isolation)
}

fn worker_refusals(wc: &WorkerConfig) -> (IsolationLevel, BTreeMap<String, Refusal>) {
    let own = worker_job_isolation(wc);
    let (mins, agent_only, agent) = (
        MIN_JOB_ISOLATION_BY_TAG.load(),
        AGENT_ONLY_TAGS.load(),
        is_agent_worker(),
    );
    let refused = wc
        .worker_tags
        .iter()
        .filter_map(|t| refusal_for(t, own, &mins, &agent_only, agent).map(|r| (t.clone(), r)))
        .collect();
    (own, refused)
}

/// The tags this worker refuses under the current settings, its mode and its own isolation.
pub fn worker_refused_tags(wc: &WorkerConfig) -> Vec<String> {
    worker_refusals(wc).1.into_keys().collect()
}

/// The subset of `tags` this worker may pull from.
pub fn worker_allowed_tags(wc: &WorkerConfig, tags: &[String]) -> Vec<String> {
    let own = worker_job_isolation(wc);
    let (mins, agent_only, agent) = (
        MIN_JOB_ISOLATION_BY_TAG.load(),
        AGENT_ONLY_TAGS.load(),
        is_agent_worker(),
    );
    tags.iter()
        .filter(|t| refusal_for(t, own, &mins, &agent_only, agent).is_none())
        .cloned()
        .collect()
}

/// Why this worker must not run a job with `tag`, if it must not. Pulls already leave refused
/// tags out; this catches the jobs handed to a worker directly, like a same-worker flow's steps,
/// which cannot be left for another worker and so fail.
pub fn worker_refusal(wc: &WorkerConfig, tag: &str) -> Option<String> {
    refusal(
        tag,
        worker_job_isolation(wc),
        &MIN_JOB_ISOLATION_BY_TAG.load(),
        &AGENT_ONLY_TAGS.load(),
        is_agent_worker(),
    )
}

fn refusal(
    tag: &str,
    own: IsolationLevel,
    mins: &MinJobIsolationByTag,
    agent_only: &[String],
    agent: bool,
) -> Option<String> {
    refusal_for(tag, own, mins, agent_only, agent).map(|r| r.message(tag, own))
}

static WORKER_REFUSES_TAGS: AtomicBool = AtomicBool::new(false);

/// Whether this worker refuses any of its tags, which can leave it with no pull query at all.
pub fn worker_refuses_tags() -> bool {
    WORKER_REFUSES_TAGS.load(Ordering::Relaxed)
}

/// Logs the tags this worker refuses, only when that set changes, since it is re-evaluated on
/// every config reload.
pub fn log_worker_refused_tags(wc: &WorkerConfig) {
    let (own, refused) = worker_refusals(wc);
    WORKER_REFUSES_TAGS.store(!refused.is_empty(), Ordering::Relaxed);
    let mut last = LAST_REFUSED.lock().unwrap_or_else(|e| e.into_inner());
    if last.as_ref() == Some(&refused) || (last.is_none() && refused.is_empty()) {
        *last = Some(refused);
        return;
    }
    for (tag, refusal) in refused.iter() {
        tracing::error!(
            "{}: this worker will not pull jobs with that tag",
            refusal.message(tag, own)
        );
    }
    if refused.is_empty() && last.as_ref().is_some_and(|l| !l.is_empty()) {
        tracing::info!("This worker no longer refuses any of its tags");
    }
    *last = Some(refused);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_ordered_and_unrecognized_minimums_fail_closed() {
        assert!(IsolationLevel::None < IsolationLevel::Unshare);
        assert!(IsolationLevel::Unshare < IsolationLevel::Nsjail);

        let mins = parse_min_job_isolation_by_tag(Some(serde_json::json!({
            "a": "unshare", "b": "nsjail", "c": "nsjail_sandboxing"
        })));
        let tags: Vec<String> = ["a", "b", "c", "d"].iter().map(|s| s.to_string()).collect();

        let refused = |own| {
            refused_tags(&tags, own, &mins)
                .into_keys()
                .collect::<Vec<_>>()
        };
        assert_eq!(refused(IsolationLevel::None), ["a", "b", "c"]);
        assert_eq!(refused(IsolationLevel::Unshare), ["b", "c"]);
        assert!(refused(IsolationLevel::Nsjail).is_empty());
        assert_eq!(
            allowed_tags(&tags, IsolationLevel::Unshare, &mins),
            ["a", "d"]
        );

        assert_eq!(
            IsolationLevel::effective(Some("nsjail"), true),
            IsolationLevel::None
        );
        assert_eq!(IsolationLevel::effective(None, false), IsolationLevel::None);
        assert_eq!(
            IsolationLevel::effective(Some("nsjail"), false),
            IsolationLevel::Nsjail
        );
    }

    #[test]
    fn derived_tags_take_the_strictest_minimum_they_derive_from() {
        let mins = parse_min_job_isolation_by_tag(Some(serde_json::json!({
            "bash": "unshare", "bash-ws1": "nsjail"
        })));
        let min = |t| min_for_tag(t, &mins);
        assert_eq!(min("bash"), Some(IsolationLevel::Unshare));
        assert_eq!(min("bash-dev"), Some(IsolationLevel::Unshare));
        assert_eq!(min("bash-ws2-fork"), Some(IsolationLevel::Unshare));
        assert_eq!(min("bash-ws1-dev"), Some(IsolationLevel::Nsjail));
        assert_eq!(min("bashx"), None);
        assert_eq!(min("python3"), None);
        assert!(parse_min_job_isolation_by_tag(Some(serde_json::json!(["bash"]))).is_empty());
        assert_eq!(
            refusal("bash-dev", IsolationLevel::Unshare, &mins, &[], false),
            None
        );
        assert_eq!(
            refusal("bash-ws1", IsolationLevel::Unshare, &mins, &[], false).as_deref(),
            Some(
                "Tag bash-ws1 requires job isolation nsjail but this worker's isolation is unshare"
            )
        );
        assert_eq!(
            refusal("python3", IsolationLevel::None, &mins, &[], false),
            None
        );
    }

    #[test]
    fn writes_accept_only_a_map_of_known_levels() {
        let ok = |v: serde_json::Value| validate_min_job_isolation_by_tag(Some(&v)).is_ok();
        assert!(ok(
            serde_json::json!({"bash": "nsjail", "untrusted": "unshare"})
        ));
        assert!(ok(serde_json::Value::Null));
        assert!(!ok(serde_json::json!(["bash"])));
        assert!(!ok(serde_json::json!("bash")));
        assert!(!ok(serde_json::json!({"bash": "nsjail_sandboxing"})));
        assert!(!ok(serde_json::json!({"bash": "none"})));
        assert!(!ok(serde_json::json!({"": "nsjail"})));

        let ok = |v: serde_json::Value| validate_agent_only_tags(Some(&v)).is_ok();
        assert!(ok(serde_json::json!(["untrusted"])));
        assert!(!ok(serde_json::json!({"untrusted": true})));
        assert!(!ok(serde_json::json!(["untrusted", ""])));
        assert!(!ok(serde_json::json!([1])));
    }

    #[test]
    fn agent_only_tags_are_refused_to_regular_workers_only() {
        let agent_only = parse_agent_only_tags(Some(serde_json::json!(["untrusted"])));
        let mins = parse_min_job_isolation_by_tag(Some(serde_json::json!({"untrusted": "nsjail"})));
        let r = |tag, own, agent| refusal_for(tag, own, &mins, &agent_only, agent);

        assert_eq!(
            r("untrusted-ws1", IsolationLevel::Nsjail, false),
            Some(Refusal::AgentOnly)
        );
        assert_eq!(
            r("untrusted", IsolationLevel::None, false),
            Some(Refusal::AgentOnly)
        );
        assert_eq!(r("python3", IsolationLevel::None, false), None);
        // An agent is still held to the minimum, and only to it.
        assert_eq!(
            r("untrusted", IsolationLevel::Unshare, true),
            Some(Refusal::Isolation(IsolationLevel::Nsjail))
        );
        assert_eq!(r("untrusted", IsolationLevel::Nsjail, true), None);
        assert_eq!(
            refusal(
                "untrusted",
                IsolationLevel::Nsjail,
                &mins,
                &agent_only,
                false
            )
            .as_deref(),
            Some("Tag untrusted requires an agent worker")
        );
    }
}
