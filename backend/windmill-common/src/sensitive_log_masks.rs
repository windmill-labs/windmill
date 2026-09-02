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
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Minimum length for a secret to be registered for masking.
/// Short strings (e.g. "true", "1234") would cause too many false positives.
const MIN_SECRET_LENGTH: usize = 8;

const MASKED_NOTICE: &str =
    "[windmill] secret value was masked for security reasons, use string transformations to display full value";

/// The secrets registered for one job, plus the automaton compiled from them.
#[derive(Default)]
struct JobMasks {
    secrets: HashSet<String>,
    /// Built on the first `snapshot` after a change and shared by every later
    /// snapshot. Every job registers at least its own token, so without this
    /// cache each log batch of each job would rebuild the automaton.
    compiled: Option<Arc<CompiledMasks>>,
}

/// Aho-Corasick automaton for O(m) multi-pattern matching in a single pass,
/// regardless of the number of secrets registered, with the replacement
/// strings indexed to match the automaton's pattern order.
struct CompiledMasks {
    ac: aho_corasick::AhoCorasick,
    replacements: Vec<String>,
}

lazy_static::lazy_static! {
    /// Map of job_id -> secret values that should be masked in that job's logs.
    static ref SENSITIVE_MASKS: RwLock<HashMap<Uuid, JobMasks>> =
        RwLock::new(HashMap::new());

    /// Set of currently running job IDs on this worker process.
    static ref RUNNING_JOBS: RwLock<HashSet<Uuid>> =
        RwLock::new(HashSet::new());

}

/// A lock-free snapshot of secrets for a job, taken once per log batch.
pub struct MaskSnapshot {
    compiled: Arc<CompiledMasks>,
    /// Whether the security notice has already been appended for this snapshot.
    /// Tracked locally to avoid a global write lock on every masked line.
    notice_shown: std::cell::Cell<bool>,
}

impl MaskSnapshot {
    /// Mask all secrets in `text`. Returns `Cow::Borrowed` when no match (zero allocation).
    /// The Aho-Corasick scan is O(text_len) regardless of how many secrets are registered.
    pub fn mask<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if text.is_empty() {
            return Cow::Borrowed(text);
        }

        // Single-pass check + replace using the pre-built automaton
        if !self.compiled.ac.is_match(text) {
            return Cow::Borrowed(text);
        }

        let mut result = self
            .compiled
            .ac
            .replace_all(text, &self.compiled.replacements);

        // Append the notice only once per snapshot (i.e. per batch)
        if !self.notice_shown.get() {
            self.notice_shown.set(true);
            result.push('\n');
            result.push_str(MASKED_NOTICE);
        }

        Cow::Owned(result)
    }
}

/// A masker for log sinks that persist asynchronously and can still be flushing
/// after the job is unregistered: nativets hands `console.log` output to a task
/// that drains a channel, so a tail of lines can be written past the end of the
/// run. It keeps the last masks it saw, so that tail is masked like the rest of
/// the log, and picks up secrets registered mid-run on the way there.
///
/// Masks by job id alone — the caller is the one that knows the text it passes
/// belongs to that job.
pub struct JobMasker {
    job_id: Uuid,
    snapshot: Option<MaskSnapshot>,
}

impl JobMasker {
    pub fn new(job_id: Uuid) -> Self {
        JobMasker { job_id, snapshot: None }
    }

    /// Mask every secret registered for the job. Returns `Cow::Borrowed` when no match.
    pub fn mask<'a>(&mut self, text: &'a str) -> Cow<'a, str> {
        if let Some(fresh) = snapshot(&self.job_id) {
            // Replacing an equivalent snapshot would re-arm the notice, so only take
            // one built from a secret set we have not seen.
            let unchanged = self
                .snapshot
                .as_ref()
                .is_some_and(|cur| Arc::ptr_eq(&cur.compiled, &fresh.compiled));
            if !unchanged {
                self.snapshot = Some(fresh);
            }
        }
        match self.snapshot.as_ref() {
            Some(snapshot) => snapshot.mask(text),
            None => Cow::Borrowed(text),
        }
    }
}

/// Take a snapshot of the current secrets for a job. Returns `None` if no secrets
/// are registered (the caller can then skip masking entirely for the whole batch).
///
/// Call this once per log batch in `write_lines`, not per line.
pub fn snapshot(job_id: &Uuid) -> Option<MaskSnapshot> {
    {
        let masks = SENSITIVE_MASKS.read().unwrap_or_else(|e| e.into_inner());
        let job = masks.get(job_id)?;
        if job.secrets.is_empty() {
            return None;
        }
        if let Some(compiled) = job.compiled.as_ref() {
            return Some(MaskSnapshot {
                compiled: compiled.clone(),
                notice_shown: std::cell::Cell::new(false),
            });
        }
    }

    let mut masks = SENSITIVE_MASKS.write().unwrap_or_else(|e| e.into_inner());
    let job = masks.get_mut(job_id)?;
    if job.secrets.is_empty() {
        return None;
    }
    let compiled = job
        .compiled
        .get_or_insert_with(|| Arc::new(compile(&job.secrets)));
    Some(MaskSnapshot { compiled: compiled.clone(), notice_shown: std::cell::Cell::new(false) })
}

fn compile(secrets: &HashSet<String>) -> CompiledMasks {
    // Sort longest-first so longer secrets are matched before shorter substrings
    let mut sorted: Vec<&String> = secrets.iter().collect();
    sorted.sort_by(|a, b| b.len().cmp(&a.len()));

    let replacements: Vec<String> = sorted
        .iter()
        .map(|s| {
            let char_count = s.chars().count();
            if char_count > 20 {
                let prefix: String = s.chars().take(3).collect();
                let suffix: String = s.chars().skip(char_count - 3).collect();
                format!("{}*****{}", prefix, suffix)
            } else {
                let first: String = s.chars().take(1).collect();
                let last: String = s.chars().skip(char_count - 1).collect();
                format!("{}*****{}", first, last)
            }
        })
        .collect();

    let ac = aho_corasick::AhoCorasickBuilder::new()
        .match_kind(aho_corasick::MatchKind::LeftmostLongest)
        .build(sorted.iter().map(|s| s.as_str()))
        .expect("failed to build aho-corasick automaton");

    CompiledMasks { ac, replacements }
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
        if let Some(job) = masks.get_mut(&job_id) {
            if job.secrets.insert(secret.to_string()) {
                job.compiled = None;
            }
        }
    }
}

/// Register a secret value for a specific job.
/// Used for the job's own token and for `$encrypted:` args, where we know the job ID.
pub fn register_secret_for_job(job_id: Uuid, secret: &str) {
    if secret.len() < MIN_SECRET_LENGTH {
        return;
    }
    let mut masks = SENSITIVE_MASKS.write().unwrap_or_else(|e| e.into_inner());
    if let Some(job) = masks.get_mut(&job_id) {
        if job.secrets.insert(secret.to_string()) {
            job.compiled = None;
        }
    }
}
