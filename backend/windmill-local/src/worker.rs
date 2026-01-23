//! Worker for local mode
//!
//! A single embedded worker that pulls jobs from the queue and executes them.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use chrono::Utc;

use crate::db::LocalDb;
use crate::error::Result;
use crate::executor::{execute_script, ExecutionResult};
use crate::flow_executor;
use crate::jobs::{complete_job, JobKind, JobStatus, QueuedJob};
use crate::queue::pull_job;
use windmill_common::flows::FlowValue;

/// Worker that processes jobs from the queue
pub struct Worker {
    db: Arc<LocalDb>,
    /// Channel to signal shutdown
    shutdown_rx: watch::Receiver<bool>,
}

impl Worker {
    /// Create a new worker
    pub fn new(db: Arc<LocalDb>, shutdown_rx: watch::Receiver<bool>) -> Self {
        Self { db, shutdown_rx }
    }

    /// Run the worker loop
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Worker started");

        loop {
            // Check for shutdown signal
            if *self.shutdown_rx.borrow() {
                tracing::info!("Worker received shutdown signal");
                break;
            }

            // Try to pull a job
            match pull_job(&self.db).await {
                Ok(Some(job)) => {
                    tracing::info!("Processing job: {} (kind: {:?})", job.id, job.kind);
                    if let Err(e) = self.process_job(job).await {
                        tracing::error!("Error processing job: {}", e);
                    }
                }
                Ok(None) => {
                    // No jobs available, wait a bit before polling again
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                        _ = self.shutdown_rx.changed() => {}
                    }
                }
                Err(e) => {
                    tracing::error!("Error pulling job: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }

        tracing::info!("Worker stopped");
        Ok(())
    }

    /// Process a single job
    async fn process_job(&self, job: QueuedJob) -> Result<()> {
        let started_at = Utc::now();

        match job.kind {
            JobKind::Preview => {
                self.process_preview_job(job, started_at).await
            }
            JobKind::FlowPreview => {
                self.process_flow_preview_job(job, started_at).await
            }
            _ => {
                // Unsupported job kind
                let error_result = serde_json::json!({
                    "error": format!("Unsupported job kind in local mode: {:?}", job.kind)
                });
                complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await
            }
        }
    }

    /// Process a script preview job
    async fn process_preview_job(&self, job: QueuedJob, started_at: chrono::DateTime<Utc>) -> Result<()> {
        let Some(code) = &job.raw_code else {
            let error_result = serde_json::json!({"error": "No code provided for preview"});
            return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
        };

        let Some(lang) = job.script_lang else {
            let error_result = serde_json::json!({"error": "No language specified for preview"});
            return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
        };

        // Execute the script
        let exec_result = execute_script(lang, code, &job.args).await;

        match exec_result {
            Ok(ExecutionResult { success, result, logs }) => {
                tracing::debug!("Job {} logs:\n{}", job.id, logs);
                let status = if success { JobStatus::Success } else { JobStatus::Failure };
                complete_job(&self.db, job.id, status, result, started_at).await
            }
            Err(e) => {
                let error_result = serde_json::json!({"error": e.to_string()});
                complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await
            }
        }
    }

    /// Process a flow preview job using the full flow executor
    async fn process_flow_preview_job(&self, job: QueuedJob, started_at: chrono::DateTime<Utc>) -> Result<()> {
        let Some(flow_json) = &job.raw_flow else {
            let error_result = serde_json::json!({"error": "No flow definition provided"});
            return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
        };

        // Parse the flow value using windmill-common types
        let flow_value: FlowValue = match serde_json::from_value(flow_json.clone()) {
            Ok(fv) => fv,
            Err(e) => {
                let error_result = serde_json::json!({"error": format!("Failed to parse flow: {}", e)});
                return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
            }
        };

        if flow_value.modules.is_empty() {
            let error_result = serde_json::json!({"error": "Flow has no modules"});
            return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
        }

        tracing::info!("Executing flow with {} modules", flow_value.modules.len());

        // Execute the flow using the full flow executor
        match flow_executor::execute_flow(&self.db, &flow_value, job.args.clone()).await {
            Ok((result, status)) => {
                let is_failure = status.failure_module.is_some();
                let final_result = serde_json::json!({
                    "result": result,
                    "flow_status": status
                });

                let job_status = if is_failure {
                    JobStatus::Failure
                } else {
                    JobStatus::Success
                };

                complete_job(&self.db, job.id, job_status, final_result, started_at).await
            }
            Err(e) => {
                let error_result = serde_json::json!({"error": e.to_string()});
                complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::{get_completed_job, push_preview, PreviewRequest, ScriptLang};

    #[tokio::test]
    async fn test_worker_processes_bash_preview() {
        let db = Arc::new(LocalDb::in_memory().await.unwrap());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Push a bash preview job
        let req = PreviewRequest {
            content: "echo 42".to_string(),
            language: ScriptLang::Bash,
            args: serde_json::json!({}),
            lock: None,
            tag: None,
        };
        let job_id = push_preview(&db, req).await.unwrap();

        // Create and run worker for one iteration
        let mut worker = Worker::new(db.clone(), shutdown_rx);

        // Process one job then shutdown
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            shutdown_tx.send(true).unwrap();
        });

        worker.run().await.unwrap();

        // Check the job completed
        let completed = get_completed_job(&db, job_id).await.unwrap();
        assert!(completed.is_some());
        let completed = completed.unwrap();
        assert_eq!(completed.status, JobStatus::Success);
        // Output "42" is parsed as JSON number
        assert_eq!(completed.result, serde_json::json!(42));
    }
}
