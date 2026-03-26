//! In-memory store for masking sensitive values (secrets, password args) in job logs.
//!
//! Workers run an embedded server in the same process, so we use global state to track:
//! - Which jobs are currently running
//! - Which secret values each job should mask in its stdout
//!
//! When a secret is fetched via `get_value_internal` (embedded server handler), we don't know
//! which job triggered the request (auth is user-based, not job-based), so we register the
//! secret for ALL currently running jobs on this worker process.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use uuid::Uuid;

/// Minimum length for a secret to be registered for masking.
/// Short strings (e.g. "true", "1234") would cause too many false positives.
const MIN_SECRET_LENGTH: usize = 8;

const MASKED_NOTICE: &str =
    "[windmill] secret value was masked for security reasons, use string transformations to display full value";

lazy_static::lazy_static! {
    /// Map of job_id -> set of secret values that should be masked in that job's logs.
    static ref SENSITIVE_MASKS: RwLock<HashMap<Uuid, HashSet<String>>> =
        RwLock::new(HashMap::new());

    /// Set of currently running job IDs on this worker process.
    static ref RUNNING_JOBS: RwLock<HashSet<Uuid>> =
        RwLock::new(HashSet::new());

    /// Tracks which jobs have already had the masking notice appended,
    /// so we only show it once per job.
    static ref NOTICE_SHOWN: RwLock<HashSet<Uuid>> =
        RwLock::new(HashSet::new());
}

/// A lock-free snapshot of secrets for a job, taken once per log batch.
/// Uses Aho-Corasick for O(m) multi-pattern matching in a single pass,
/// regardless of the number of secrets registered.
pub struct MaskSnapshot {
    /// Aho-Corasick automaton for fast matching.
    ac: aho_corasick::AhoCorasick,
    /// Replacement strings, indexed to match the automaton's pattern order.
    replacements: Vec<String>,
    job_id: Uuid,
}

impl MaskSnapshot {
    /// Mask all secrets in `text`. Returns `Cow::Borrowed` when no match (zero allocation).
    /// The Aho-Corasick scan is O(text_len) regardless of how many secrets are registered.
    pub fn mask<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if text.is_empty() {
            return Cow::Borrowed(text);
        }

        // Single-pass check + replace using the pre-built automaton
        if !self.ac.is_match(text) {
            return Cow::Borrowed(text);
        }

        let mut result = self.ac.replace_all(text, &self.replacements);

        // Append the notice only once per job
        let mut shown = NOTICE_SHOWN.write().unwrap_or_else(|e| e.into_inner());
        if shown.insert(self.job_id) {
            result.push('\n');
            result.push_str(MASKED_NOTICE);
        }

        Cow::Owned(result)
    }
}

/// Take a snapshot of the current secrets for a job. Returns `None` if no secrets
/// are registered (the caller can then skip masking entirely for the whole batch).
///
/// Call this once per log batch in `write_lines`, not per line.
pub fn snapshot(job_id: &Uuid) -> Option<MaskSnapshot> {
    let masks = SENSITIVE_MASKS.read().unwrap_or_else(|e| e.into_inner());
    let secrets = masks.get(job_id)?;
    if secrets.is_empty() {
        return None;
    }

    // Sort longest-first so longer secrets are matched before shorter substrings
    let mut sorted: Vec<&String> = secrets.iter().collect();
    sorted.sort_by(|a, b| b.len().cmp(&a.len()));

    let replacements: Vec<String> = sorted
        .iter()
        .map(|s| {
            let prefix: String = s.chars().take(3).collect();
            format!("{}*****", prefix)
        })
        .collect();

    let ac = aho_corasick::AhoCorasickBuilder::new()
        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
        .build(sorted.iter().map(|s| s.as_str()))
        .expect("failed to build aho-corasick automaton");

    Some(MaskSnapshot { ac, replacements, job_id: *job_id })
}

/// Register a job as currently running. Call this before `handle_queued_job`.
pub fn register_running_job(job_id: Uuid) {
    {
        let mut jobs = RUNNING_JOBS.write().unwrap_or_else(|e| e.into_inner());
        jobs.insert(job_id);
    }
    {
        let mut masks = SENSITIVE_MASKS.write().unwrap_or_else(|e| e.into_inner());
        masks.entry(job_id).or_default();
    }
}

/// Unregister a job when it completes. Removes both the running job entry and its mask set.
pub fn unregister_running_job(job_id: Uuid) {
    {
        let mut jobs = RUNNING_JOBS.write().unwrap_or_else(|e| e.into_inner());
        jobs.remove(&job_id);
    }
    {
        let mut masks = SENSITIVE_MASKS.write().unwrap_or_else(|e| e.into_inner());
        masks.remove(&job_id);
    }
    {
        let mut shown = NOTICE_SHOWN.write().unwrap_or_else(|e| e.into_inner());
        shown.remove(&job_id);
    }
}

/// Register a secret value for ALL currently running jobs.
/// Used when a secret is fetched via the embedded server (we don't know which job triggered it).
pub fn register_secret_for_all_running_jobs(secret: &str) {
    if secret.len() < MIN_SECRET_LENGTH {
        return;
    }
    let jobs = RUNNING_JOBS.read().unwrap_or_else(|e| e.into_inner());
    if jobs.is_empty() {
        return;
    }
    let job_ids: Vec<Uuid> = jobs.iter().copied().collect();
    drop(jobs);

    let mut masks = SENSITIVE_MASKS.write().unwrap_or_else(|e| e.into_inner());
    for job_id in job_ids {
        if let Some(set) = masks.get_mut(&job_id) {
            set.insert(secret.to_string());
        }
    }
}

/// Register a secret value for a specific job.
/// Used for `$encrypted:` args where we know the job ID.
pub fn register_secret_for_job(job_id: Uuid, secret: &str) {
    if secret.len() < MIN_SECRET_LENGTH {
        return;
    }
    let mut masks = SENSITIVE_MASKS.write().unwrap_or_else(|e| e.into_inner());
    if let Some(set) = masks.get_mut(&job_id) {
        set.insert(secret.to_string());
    }
}
