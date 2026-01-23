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
use crate::jobs::{complete_job, JobKind, JobStatus, QueuedJob};
use crate::queue::pull_job;

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

    /// Process a flow preview job
    ///
    /// This is a simplified flow executor that handles basic linear flows.
    /// A full implementation would need to handle branching, loops, etc.
    async fn process_flow_preview_job(&self, job: QueuedJob, started_at: chrono::DateTime<Utc>) -> Result<()> {
        let Some(flow_value) = &job.raw_flow else {
            let error_result = serde_json::json!({"error": "No flow definition provided"});
            return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
        };

        // Extract modules from flow value
        let modules = flow_value
            .get("modules")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();

        if modules.is_empty() {
            let error_result = serde_json::json!({"error": "Flow has no modules"});
            return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
        }

        // Execute modules sequentially (simplified - no branching support)
        let mut current_result = job.args.clone();
        let mut flow_status = serde_json::json!({
            "modules": [],
            "failure_module": serde_json::Value::Null
        });

        for (idx, module) in modules.iter().enumerate() {
            let default_id = format!("module_{}", idx);
            let module_id = module
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or(&default_id);

            tracing::info!("Executing flow module: {}", module_id);

            // Update flow status
            if let Some(modules_arr) = flow_status.get_mut("modules").and_then(|m| m.as_array_mut()) {
                modules_arr.push(serde_json::json!({
                    "id": module_id,
                    "type": "InProgress"
                }));
            }

            // Extract module value (the actual script/action)
            let module_value = module.get("value");

            match self.execute_flow_module(module_value, &current_result).await {
                Ok(result) => {
                    current_result = result;
                    // Update status to success
                    if let Some(modules_arr) = flow_status.get_mut("modules").and_then(|m| m.as_array_mut()) {
                        if let Some(last) = modules_arr.last_mut() {
                            last["type"] = serde_json::json!("Success");
                            last["result"] = current_result.clone();
                        }
                    }
                }
                Err(e) => {
                    // Module failed
                    flow_status["failure_module"] = serde_json::json!({
                        "id": module_id,
                        "error": e.to_string()
                    });
                    let error_result = serde_json::json!({
                        "error": e.to_string(),
                        "flow_status": flow_status
                    });
                    return complete_job(&self.db, job.id, JobStatus::Failure, error_result, started_at).await;
                }
            }
        }

        // Flow completed successfully
        let final_result = serde_json::json!({
            "result": current_result,
            "flow_status": flow_status
        });
        complete_job(&self.db, job.id, JobStatus::Success, final_result, started_at).await
    }

    /// Execute a single flow module
    async fn execute_flow_module(
        &self,
        module_value: Option<&serde_json::Value>,
        input: &serde_json::Value,
    ) -> std::result::Result<serde_json::Value, String> {
        let Some(value) = module_value else {
            return Err("Module has no value".to_string());
        };

        // Check module type
        let module_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match module_type {
            "rawscript" => {
                // Inline script
                let code = value
                    .get("content")
                    .and_then(|c| c.as_str())
                    .ok_or("rawscript module missing content")?;

                let lang_str = value
                    .get("language")
                    .and_then(|l| l.as_str())
                    .unwrap_or("deno");

                let lang = crate::jobs::ScriptLang::from_str(lang_str)
                    .ok_or_else(|| format!("Unknown language: {}", lang_str))?;

                let result = execute_script(lang, code, input)
                    .await
                    .map_err(|e| e.to_string())?;

                if result.success {
                    Ok(result.result)
                } else {
                    Err(result.result.to_string())
                }
            }
            "identity" => {
                // Pass through input
                Ok(input.clone())
            }
            _ => {
                Err(format!("Unsupported module type in local mode: {}", module_type))
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
