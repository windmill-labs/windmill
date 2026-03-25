//! Tests for sensitive log masking: ensuring secrets and password-type args
//! are masked in job stdout before being persisted to the database.
//!
//! Run with:
//!   cargo test -p windmill-common --test sensitive_log_masks_test -- --nocapture
//!
//! These are pure in-memory tests (no database needed).

use std::borrow::Cow;
use uuid::Uuid;
use windmill_common::sensitive_log_masks::{
    mask_sensitive_values, register_running_job, register_secret_for_all_running_jobs,
    register_secret_for_job, unregister_running_job,
};

/// Helper: create a fresh job id and register it as running.
fn setup_job() -> Uuid {
    let job_id = Uuid::new_v4();
    register_running_job(job_id);
    job_id
}

#[test]
fn test_register_and_mask_secret() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "my_super_secret_password");

    let input = "connecting with password my_super_secret_password to database";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(
        masked.as_ref(),
        "connecting with password ******* to database"
    );
    assert!(matches!(masked, Cow::Owned(_)));

    unregister_running_job(job_id);
}

#[test]
fn test_secret_for_all_running_jobs() {
    let job1 = setup_job();
    let job2 = setup_job();
    let job3 = setup_job();

    register_secret_for_all_running_jobs("shared_api_key_value_12345");

    for job_id in [job1, job2, job3] {
        let masked = mask_sensitive_values(&job_id, "using key shared_api_key_value_12345 now");
        assert_eq!(masked.as_ref(), "using key ******* now");
    }

    unregister_running_job(job1);
    unregister_running_job(job2);
    unregister_running_job(job3);
}

#[test]
fn test_unregister_cleans_up() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "temporary_secret_value");

    // Before unregister: masking works
    let masked = mask_sensitive_values(&job_id, "value is temporary_secret_value");
    assert_eq!(masked.as_ref(), "value is *******");

    unregister_running_job(job_id);

    // After unregister: no masking (job not found, returns original)
    let masked = mask_sensitive_values(&job_id, "value is temporary_secret_value");
    assert_eq!(masked.as_ref(), "value is temporary_secret_value");
    assert!(matches!(masked, Cow::Borrowed(_)));
}

#[test]
fn test_short_secrets_ignored() {
    let job_id = setup_job();

    // Secrets shorter than 8 chars should be ignored
    register_secret_for_job(job_id, "short");
    register_secret_for_job(job_id, "abc");
    register_secret_for_job(job_id, "1234567"); // 7 chars, still too short

    let input = "values: short, abc, 1234567 should not be masked";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), input);
    assert!(matches!(masked, Cow::Borrowed(_)));

    // But 8-char secrets should be masked
    register_secret_for_job(job_id, "12345678");
    let masked = mask_sensitive_values(&job_id, "value 12345678 is masked");
    assert_eq!(masked.as_ref(), "value ******* is masked");

    unregister_running_job(job_id);
}

#[test]
fn test_mask_multiple_secrets() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "first_secret_value");
    register_secret_for_job(job_id, "second_secret_value");
    register_secret_for_job(job_id, "third_secret_value");

    let input = "got first_secret_value and second_secret_value and third_secret_value done";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), "got ******* and ******* and ******* done");

    unregister_running_job(job_id);
}

#[test]
fn test_no_masking_when_no_secrets() {
    let job_id = setup_job();

    // No secrets registered, text should pass through unchanged
    let input = "this is normal log output with no secrets";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), input);
    assert!(matches!(masked, Cow::Borrowed(_)));

    unregister_running_job(job_id);
}

#[test]
fn test_mask_returns_borrowed_when_no_match() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "very_specific_secret_not_in_text");

    let input = "this log line has nothing sensitive";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), input);
    assert!(matches!(masked, Cow::Borrowed(_)));

    unregister_running_job(job_id);
}

#[test]
fn test_multiple_occurrences_of_same_secret() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "repeated_secret_val");

    let input = "repeated_secret_val appears twice: repeated_secret_val";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), "******* appears twice: *******");

    unregister_running_job(job_id);
}

#[test]
fn test_secret_not_leaked_to_other_jobs() {
    let job1 = setup_job();
    let job2 = setup_job();

    register_secret_for_job(job1, "job1_only_secret_value");

    // job1 should have masking
    let masked = mask_sensitive_values(&job1, "secret is job1_only_secret_value");
    assert_eq!(masked.as_ref(), "secret is *******");

    // job2 should NOT have masking for job1's secret
    let masked = mask_sensitive_values(&job2, "secret is job1_only_secret_value");
    assert_eq!(masked.as_ref(), "secret is job1_only_secret_value");

    unregister_running_job(job1);
    unregister_running_job(job2);
}

#[test]
fn test_empty_secret_ignored() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "");

    let input = "nothing should be masked here";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), input);

    unregister_running_job(job_id);
}

#[test]
fn test_register_secret_for_all_skips_unregistered_jobs() {
    let job1 = setup_job();

    // Unregister job1 before registering a global secret
    unregister_running_job(job1);

    // This should not panic or add to removed jobs
    register_secret_for_all_running_jobs("global_secret_value_123");

    // job1 should not have masking since it's unregistered
    let masked = mask_sensitive_values(&job1, "global_secret_value_123");
    assert_eq!(masked.as_ref(), "global_secret_value_123");
}

#[test]
fn test_mask_secret_containing_special_chars() {
    let job_id = setup_job();
    // Secret with regex-special characters
    register_secret_for_job(job_id, "p@$$w0rd!#(2024)");

    let input = "auth with p@$$w0rd!#(2024) succeeded";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(masked.as_ref(), "auth with ******* succeeded");

    unregister_running_job(job_id);
}

#[test]
fn test_multiline_masking() {
    let job_id = setup_job();
    register_secret_for_job(job_id, "line_spanning_secret");

    let input = "line1: line_spanning_secret\nline2: line_spanning_secret\nline3: safe";
    let masked = mask_sensitive_values(&job_id, input);
    assert_eq!(
        masked.as_ref(),
        "line1: *******\nline2: *******\nline3: safe"
    );

    unregister_running_job(job_id);
}
