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

}

/// A lock-free snapshot of secrets for a job, taken once per log batch.
/// Uses Aho-Corasick for O(m) multi-pattern matching in a single pass,
/// regardless of the number of secrets registered.
pub struct MaskSnapshot {
    /// Aho-Corasick automaton for fast matching.
    ac: aho_corasick::AhoCorasick,
    /// Replacement strings, indexed to match the automaton's pattern order.
    replacements: Vec<String>,
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
        if !self.ac.is_match(text) {
            return Cow::Borrowed(text);
        }

        let mut result = self.ac.replace_all(text, &self.replacements);

        // Append the notice only once per snapshot (i.e. per batch)
        if !self.notice_shown.get() {
            self.notice_shown.set(true);
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

    Some(MaskSnapshot { ac, replacements, notice_shown: std::cell::Cell::new(false) })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    // The registries are process-global and `register_secret_for_all_running_jobs`
    // writes to every registered job, so tests running in parallel would leak
    // secrets into each other's snapshots.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn running_job_with(secrets: &[&str]) -> Uuid {
        let job_id = Uuid::new_v4();
        register_running_job(job_id);
        for s in secrets {
            register_secret_for_job(job_id, s);
        }
        job_id
    }

    #[test]
    fn short_secrets_are_not_registered() {
        let _g = lock();
        let job_id = running_job_with(&["1234567"]);
        register_secret_for_all_running_jobs("7654321");
        assert!(snapshot(&job_id).is_none());

        register_secret_for_job(job_id, "12345678");
        register_secret_for_all_running_jobs("87654321");
        let snap = snapshot(&job_id).expect("8-byte secrets are registered");
        snap.mask("warmup 12345678");
        assert_eq!(snap.mask("12345678 87654321"), "1*****8 8*****1");
        unregister_running_job(job_id);
    }

    #[test]
    fn masks_secrets_keeping_edge_characters() {
        let _g = lock();
        let job_id = running_job_with(&["password1", "abcdefghijklmnopqrstuvwxyz"]);
        let snap = snapshot(&job_id).expect("secrets registered");

        let short = snap.mask("pw=password1");
        assert!(short.starts_with("pw=p*****1\n"), "{short}");
        assert!(short.ends_with(MASKED_NOTICE));

        assert_eq!(snap.mask("k=abcdefghijklmnopqrstuvwxyz"), "k=abc*****xyz");

        let next_batch = snapshot(&job_id).unwrap();
        assert!(next_batch.mask("pw=password1").ends_with(MASKED_NOTICE));
        unregister_running_job(job_id);
    }

    #[test]
    fn longer_secret_wins_over_its_substring() {
        let _g = lock();
        let job_id = running_job_with(&["supersecret", "supersecretvalue"]);
        let snap = snapshot(&job_id).unwrap();
        snap.mask("warmup supersecret");

        assert_eq!(snap.mask("supersecretvalue"), "s*****e");
        unregister_running_job(job_id);
    }

    #[test]
    fn unmatched_text_is_borrowed_unchanged() {
        let _g = lock();
        let job_id = running_job_with(&["password1"]);
        let snap = snapshot(&job_id).unwrap();

        assert!(matches!(
            snap.mask("nothing here"),
            Cow::Borrowed("nothing here")
        ));
        assert!(matches!(snap.mask(""), Cow::Borrowed("")));
        unregister_running_job(job_id);
    }

    #[test]
    fn multibyte_secrets_are_masked_on_char_boundaries() {
        let _g = lock();
        let job_id = running_job_with(&["pässwört"]);
        let snap = snapshot(&job_id).unwrap();
        snap.mask("warmup pässwört");

        assert_eq!(snap.mask("pässwört!"), "p*****t!");
        unregister_running_job(job_id);
    }

    #[test]
    fn secret_for_all_jobs_reaches_only_running_jobs() {
        let _g = lock();
        let a = running_job_with(&[]);
        let b = running_job_with(&[]);
        let finished = running_job_with(&[]);
        unregister_running_job(finished);

        register_secret_for_all_running_jobs("shared-secret");

        for job_id in [a, b] {
            let snap = snapshot(&job_id).expect("secret registered for running job");
            assert!(snap.mask("shared-secret").starts_with("s*****t"));
        }
        assert!(snapshot(&finished).is_none());

        unregister_running_job(a);
        unregister_running_job(b);
        assert!(snapshot(&a).is_none());
    }
}
