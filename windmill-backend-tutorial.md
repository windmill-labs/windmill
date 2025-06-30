# Building a Mini Windmill: A Deep Dive into Job Execution Systems

## Introduction

Welcome to this comprehensive tutorial where we'll build a simplified version of Windmill's job execution system from scratch. We'll start with a basic finite state machine and progressively add sophisticated features like persistence, error handling, and real-time monitoring.

By the end of this tutorial, you'll understand the core engineering decisions behind distributed job execution systems and have built your own mini-windmill!

---

## Part 1: The Foundation - A Finite State Machine

### Why Start with a State Machine?

Job execution is fundamentally about state transitions. A job starts in one state (`Queued`), moves through various states (`Running`, `Completed`, `Failed`), and these transitions must be predictable and atomic.

Let's define our core job states:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub script_content: String,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
```

### The State Machine Engine

Our state machine needs to enforce valid transitions and handle side effects:

```rust
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;

pub struct JobStateMachine {
    jobs: Arc<RwLock<HashMap<Uuid, Job>>>,
    event_sender: mpsc::UnboundedSender<JobEvent>,
}

#[derive(Debug, Clone)]
pub enum JobEvent {
    Created(Uuid),
    Started(Uuid),
    Completed(Uuid, serde_json::Value),
    Failed(Uuid, String),
    Cancelled(Uuid),
}

impl JobStateMachine {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<JobEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                jobs: Arc::new(RwLock::new(HashMap::new())),
                event_sender: tx,
            },
            rx,
        )
    }

    pub async fn create_job(&self, script_content: String) -> Result<Uuid, JobError> {
        let job_id = Uuid::new_v4();
        let job = Job {
            id: job_id,
            script_content,
            status: JobStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id, job);
        }

        self.event_sender.send(JobEvent::Created(job_id))?;
        Ok(job_id)
    }

    pub async fn transition_to_running(&self, job_id: Uuid) -> Result<(), JobError> {
        let mut jobs = self.jobs.write().await;
        let job = jobs.get_mut(&job_id).ok_or(JobError::NotFound)?;

        match job.status {
            JobStatus::Queued => {
                job.status = JobStatus::Running;
                job.started_at = Some(Utc::now());
                self.event_sender.send(JobEvent::Started(job_id))?;
                Ok(())
            }
            _ => Err(JobError::InvalidTransition {
                from: job.status.clone(),
                to: JobStatus::Running,
            }),
        }
    }

    // Similar methods for other transitions...
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("Job not found")]
    NotFound,
    #[error("Invalid transition from {from:?} to {to:?}")]
    InvalidTransition { from: JobStatus, to: JobStatus },
    #[error("Event channel error: {0}")]
    ChannelError(#[from] mpsc::error::SendError<JobEvent>),
}
```

### Why This Design?

1. **Immutable Transitions**: Once we validate a state transition, it's committed atomically
2. **Event-Driven**: Every state change emits an event, enabling observability
3. **Type Safety**: Invalid transitions are caught at compile time where possible
4. **Concurrent Access**: RwLock allows multiple readers but exclusive writers

### Testing Our State Machine

```rust
#[tokio::test]
async fn test_job_lifecycle() {
    let (state_machine, mut events) = JobStateMachine::new();
    
    // Create a job
    let job_id = state_machine.create_job("print('hello')".to_string()).await.unwrap();
    assert_eq!(events.recv().await.unwrap(), JobEvent::Created(job_id));
    
    // Start the job
    state_machine.transition_to_running(job_id).await.unwrap();
    assert_eq!(events.recv().await.unwrap(), JobEvent::Started(job_id));
    
    // Complete the job
    let result = serde_json::json!({"output": "hello"});
    state_machine.complete_job(job_id, result.clone()).await.unwrap();
    
    let job = state_machine.get_job(job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.result, Some(result));
}
```

---

## Part 2: Adding Persistence with PostgreSQL and SQLx

### Why PostgreSQL?

While our in-memory state machine is fast, it's not durable. We need persistence for:
- **Crash Recovery**: Jobs survive system restarts
- **Scalability**: Multiple workers can access the same job queue
- **Audit Trail**: Complete history of job executions
- **Complex Queries**: Analytics and monitoring

### Database Schema Design

```sql
-- Our job table with careful indexing
CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    script_content TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'queued',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    result JSONB,
    error TEXT,
    
    -- Performance indexes
    CONSTRAINT valid_status CHECK (status IN ('queued', 'running', 'completed', 'failed', 'cancelled'))
);

-- Critical indexes for performance
CREATE INDEX idx_jobs_status_created_at ON jobs (status, created_at);
CREATE INDEX idx_jobs_created_at ON jobs (created_at DESC);

-- Job logs for streaming
CREATE TABLE job_logs (
    id BIGSERIAL PRIMARY KEY,
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    level VARCHAR(10) NOT NULL DEFAULT 'info',
    message TEXT NOT NULL
);

CREATE INDEX idx_job_logs_job_id_timestamp ON job_logs (job_id, timestamp);
```

### Persistent Job Store

```rust
use sqlx::{PgPool, Row};
use sqlx::postgres::PgRow;

#[derive(Clone)]
pub struct PersistentJobStore {
    pool: PgPool,
    event_sender: mpsc::UnboundedSender<JobEvent>,
}

impl PersistentJobStore {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        let (event_sender, _) = mpsc::unbounded_channel();
        
        Ok(Self {
            pool,
            event_sender,
        })
    }

    pub async fn create_job(&self, script_content: &str) -> Result<Uuid, JobError> {
        let job_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO jobs (id, script_content, status, created_at)
            VALUES ($1, $2, 'queued', NOW())
            "#,
            job_id,
            script_content
        )
        .execute(&self.pool)
        .await?;

        self.event_sender.send(JobEvent::Created(job_id))?;
        Ok(job_id)
    }

    pub async fn transition_to_running(&self, job_id: Uuid) -> Result<(), JobError> {
        // Critical: Use a transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;
        
        // First, check current status
        let current_status = sqlx::query_scalar!(
            "SELECT status FROM jobs WHERE id = $1 FOR UPDATE",
            job_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(JobError::NotFound)?;

        if current_status != "queued" {
            return Err(JobError::InvalidTransition {
                from: current_status.parse().unwrap(),
                to: JobStatus::Running,
            });
        }

        // Update to running
        sqlx::query!(
            r#"
            UPDATE jobs 
            SET status = 'running', started_at = NOW()
            WHERE id = $1
            "#,
            job_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.event_sender.send(JobEvent::Started(job_id))?;
        Ok(())
    }

    pub async fn get_next_queued_job(&self) -> Result<Option<Job>, JobError> {
        // SKIP LOCKED prevents workers from competing for the same job
        let row = sqlx::query!(
            r#"
            UPDATE jobs 
            SET status = 'running', started_at = NOW()
            WHERE id = (
                SELECT id FROM jobs 
                WHERE status = 'queued' 
                ORDER BY created_at 
                FOR UPDATE SKIP LOCKED 
                LIMIT 1
            )
            RETURNING id, script_content, status, created_at, started_at, completed_at, result, error
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let job = Job {
                    id: row.id,
                    script_content: row.script_content,
                    status: row.status.parse().unwrap(),
                    created_at: row.created_at,
                    started_at: row.started_at,
                    completed_at: row.completed_at,
                    result: row.result,
                    error: row.error,
                };
                self.event_sender.send(JobEvent::Started(job.id))?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }
}
```

### The Power of `FOR UPDATE SKIP LOCKED`

This PostgreSQL feature is crucial for distributed systems:
- **FOR UPDATE**: Locks the row for modification
- **SKIP LOCKED**: If a row is already locked, skip it instead of waiting
- **Result**: Multiple workers can poll for jobs without blocking each other

---

## Part 3: Zombie Jobs and Error Handling

### What Are Zombie Jobs?

Zombie jobs are jobs that are marked as "running" but their worker has crashed or disappeared. Without detection and recovery, these jobs would remain in limbo forever.

### Heartbeat-Based Detection

```rust
use std::time::Duration;

#[derive(Clone)]
pub struct ZombieJobDetector {
    store: PersistentJobStore,
    heartbeat_timeout: Duration,
}

impl ZombieJobDetector {
    pub fn new(store: PersistentJobStore) -> Self {
        Self {
            store,
            heartbeat_timeout: Duration::from_secs(30), // 30 second timeout
        }
    }

    pub async fn start_heartbeat_monitoring(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            if let Err(e) = self.detect_and_recover_zombies().await {
                tracing::error!("Failed to detect zombies: {}", e);
            }
        }
    }

    async fn detect_and_recover_zombies(&self) -> Result<(), JobError> {
        let timeout_threshold = Utc::now() - chrono::Duration::seconds(self.heartbeat_timeout.as_secs() as i64);
        
        // Find jobs that have been running too long without updates
        let zombie_jobs = sqlx::query!(
            r#"
            SELECT id FROM jobs 
            WHERE status = 'running' 
            AND started_at < $1
            "#,
            timeout_threshold
        )
        .fetch_all(&self.store.pool)
        .await?;

        for job in zombie_jobs {
            tracing::warn!("Detected zombie job: {}", job.id);
            self.recover_zombie_job(job.id).await?;
        }

        Ok(())
    }

    async fn recover_zombie_job(&self, job_id: Uuid) -> Result<(), JobError> {
        // Strategy 1: Retry the job (move back to queued)
        // Strategy 2: Mark as failed
        // Strategy 3: Send to dead letter queue
        
        sqlx::query!(
            r#"
            UPDATE jobs 
            SET status = 'queued', started_at = NULL, error = 'Recovered from zombie state'
            WHERE id = $1 AND status = 'running'
            "#,
            job_id
        )
        .execute(&self.store.pool)
        .await?;

        tracing::info!("Recovered zombie job: {}", job_id);
        Ok(())
    }
}
```

### Advanced Error Recovery Strategies

```rust
pub struct ErrorRecoveryManager {
    store: PersistentJobStore,
    retry_config: RetryConfig,
}

#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub backoff_multiplier: f64,
    pub base_delay: Duration,
}

impl ErrorRecoveryManager {
    pub async fn handle_job_failure(&self, job_id: Uuid, error: String) -> Result<(), JobError> {
        let mut tx = self.store.pool.begin().await?;
        
        // Get current retry count
        let retry_count = sqlx::query_scalar!(
            "SELECT COALESCE((result->>'retry_count')::int, 0) FROM jobs WHERE id = $1",
            job_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or(0);

        if retry_count < self.retry_config.max_retries as i32 {
            // Calculate exponential backoff delay
            let delay = self.retry_config.base_delay.as_secs() as f64
                * self.retry_config.backoff_multiplier.powi(retry_count);
            
            let retry_at = Utc::now() + chrono::Duration::seconds(delay as i64);
            
            // Schedule retry
            sqlx::query!(
                r#"
                UPDATE jobs 
                SET status = 'queued', 
                    started_at = NULL,
                    result = jsonb_set(
                        COALESCE(result, '{}'), 
                        '{retry_count}', 
                        ($2)::jsonb
                    ),
                    error = $3
                WHERE id = $1
                "#,
                job_id,
                serde_json::json!(retry_count + 1),
                format!("Retry {} after error: {}", retry_count + 1, error)
            )
            .execute(&mut *tx)
            .await?;
            
            tracing::info!("Scheduled retry {} for job {}", retry_count + 1, job_id);
        } else {
            // Max retries exceeded, mark as permanently failed
            sqlx::query!(
                "UPDATE jobs SET status = 'failed', completed_at = NOW(), error = $2 WHERE id = $1",
                job_id,
                format!("Max retries exceeded. Last error: {}", error)
            )
            .execute(&mut *tx)
            .await?;
            
            tracing::error!("Job {} failed permanently after {} retries", job_id, retry_count);
        }

        tx.commit().await?;
        Ok(())
    }
}
```

### Circuit Breaker Pattern

For handling cascading failures:

```rust
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

pub struct CircuitBreaker {
    failure_count: AtomicU32,
    is_open: AtomicBool,
    failure_threshold: u32,
    reset_timeout: Duration,
    last_failure: Arc<RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Future<Output = Result<T, E>>,
    {
        if self.is_open() {
            if self.should_attempt_reset().await {
                self.half_open();
            } else {
                return Err(CircuitBreakerError::Open);
            }
        }

        match operation.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(CircuitBreakerError::Failure(error))
            }
        }
    }

    fn on_success(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
        self.is_open.store(false, Ordering::SeqCst);
    }

    async fn on_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure.write().await = Some(Instant::now());
        
        if failures >= self.failure_threshold {
            self.is_open.store(true, Ordering::SeqCst);
            tracing::warn!("Circuit breaker opened after {} failures", failures);
        }
    }
}
```

---

## Part 4: Live Log Streaming

### The Challenge

Real-time log streaming from distributed workers to clients requires:
- **Low Latency**: Logs should appear almost instantly
- **Scalability**: Support many concurrent streams
- **Reliability**: Handle network interruptions gracefully
- **Ordering**: Logs must arrive in the correct sequence

### Server-Sent Events (SSE) Implementation

```rust
use axum::{
    response::sse::{Event, KeepAlive, Sse},
    extract::{Path, State},
    response::Response,
};
use futures::Stream;
use tokio_stream::wrappers::BroadcastStream;

pub struct LogStreamer {
    store: PersistentJobStore,
    broadcasters: Arc<RwLock<HashMap<Uuid, broadcast::Sender<LogEntry>>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub job_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

impl LogStreamer {
    pub async fn stream_logs(
        &self,
        job_id: Uuid,
    ) -> impl Stream<Item = Result<Event, Infallible>> {
        // Get existing logs first
        let existing_logs = self.get_existing_logs(job_id).await.unwrap_or_default();
        
        // Create or get broadcaster for this job
        let receiver = {
            let mut broadcasters = self.broadcasters.write().await;
            let (tx, rx) = broadcasters
                .entry(job_id)
                .or_insert_with(|| broadcast::channel(1000).0)
                .subscribe();
            rx
        };

        // Combine existing logs with live stream
        let existing_stream = stream::iter(existing_logs.into_iter().map(|log| {
            Ok(Event::default()
                .json_data(&log)
                .unwrap()
                .event("log"))
        }));

        let live_stream = BroadcastStream::new(receiver)
            .filter_map(|result| async move {
                match result {
                    Ok(log) => Some(Ok(Event::default()
                        .json_data(&log)
                        .unwrap()
                        .event("log"))),
                    Err(_) => None, // Handle lagged receivers
                }
            });

        existing_stream.chain(live_stream)
    }

    pub async fn append_log(&self, job_id: Uuid, level: String, message: String) -> Result<(), JobError> {
        let log_entry = LogEntry {
            job_id,
            timestamp: Utc::now(),
            level: level.clone(),
            message: message.clone(),
        };

        // Persist to database
        sqlx::query!(
            "INSERT INTO job_logs (job_id, level, message) VALUES ($1, $2, $3)",
            job_id,
            level,
            message
        )
        .execute(&self.store.pool)
        .await?;

        // Broadcast to live listeners
        if let Some(broadcaster) = self.broadcasters.read().await.get(&job_id) {
            let _ = broadcaster.send(log_entry); // Ignore if no receivers
        }

        Ok(())
    }
}

// Axum handler for SSE endpoint
pub async fn stream_job_logs(
    Path(job_id): Path<Uuid>,
    State(streamer): State<Arc<LogStreamer>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = streamer.stream_logs(job_id).await;
    
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
}
```

### WebSocket Alternative for Bidirectional Communication

```rust
use axum::extract::ws::{WebSocket, Message};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

pub async fn websocket_log_handler(
    ws: WebSocket,
    job_id: Uuid,
    streamer: Arc<LogStreamer>,
) {
    let (mut sender, mut receiver) = ws.split();
    
    // Spawn log streaming task
    let streamer_clone = streamer.clone();
    let sender_task = tokio::spawn(async move {
        let mut log_stream = streamer_clone.stream_logs(job_id).await;
        
        while let Some(log_result) = log_stream.next().await {
            match log_result {
                Ok(log_event) => {
                    let message = Message::Text(log_event.data);
                    if sender.send(message).await.is_err() {
                        break; // Client disconnected
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Handle incoming messages (e.g., log level filtering)
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Handle client commands like changing log level
                    if let Ok(command) = serde_json::from_str::<LogCommand>(&text) {
                        // Process command...
                    }
                }
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = sender_task => {},
        _ = receiver_task => {},
    }
}

#[derive(Deserialize)]
enum LogCommand {
    SetLevel(String),
    Pause,
    Resume,
}
```

### Performance Optimizations

#### Log Batching for High-Throughput Jobs

```rust
pub struct BatchedLogStreamer {
    batch_size: usize,
    flush_interval: Duration,
    pending_logs: Arc<Mutex<HashMap<Uuid, Vec<LogEntry>>>>,
}

impl BatchedLogStreamer {
    pub async fn start_batch_flusher(&self) {
        let mut interval = tokio::time::interval(self.flush_interval);
        
        loop {
            interval.tick().await;
            self.flush_all_batches().await;
        }
    }

    pub async fn append_log(&self, job_id: Uuid, level: String, message: String) {
        let log_entry = LogEntry {
            job_id,
            timestamp: Utc::now(),
            level,
            message,
        };

        let mut pending = self.pending_logs.lock().await;
        let job_logs = pending.entry(job_id).or_insert_with(Vec::new);
        job_logs.push(log_entry);

        if job_logs.len() >= self.batch_size {
            let logs_to_flush = std::mem::take(job_logs);
            drop(pending); // Release lock early
            
            self.flush_batch(job_id, logs_to_flush).await;
        }
    }

    async fn flush_batch(&self, job_id: Uuid, logs: Vec<LogEntry>) {
        // Batch insert to database
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO job_logs (job_id, timestamp, level, message) "
        );
        
        query_builder.push_values(logs.iter(), |mut b, log| {
            b.push_bind(log.job_id)
             .push_bind(log.timestamp)
             .push_bind(&log.level)
             .push_bind(&log.message);
        });

        if let Err(e) = query_builder.build().execute(&self.pool).await {
            tracing::error!("Failed to flush log batch: {}", e);
        }

        // Broadcast to listeners
        if let Some(broadcaster) = self.broadcasters.read().await.get(&job_id) {
            for log in logs {
                let _ = broadcaster.send(log);
            }
        }
    }
}
```

---

## Part 5: Putting It All Together

### The Complete Job Executor

```rust
pub struct MiniWindmill {
    store: PersistentJobStore,
    log_streamer: Arc<LogStreamer>,
    zombie_detector: ZombieJobDetector,
    error_recovery: ErrorRecoveryManager,
    worker_pool: WorkerPool,
}

impl MiniWindmill {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let store = PersistentJobStore::new(database_url).await?;
        let log_streamer = Arc::new(LogStreamer::new(store.clone()));
        let zombie_detector = ZombieJobDetector::new(store.clone());
        let error_recovery = ErrorRecoveryManager::new(store.clone());
        let worker_pool = WorkerPool::new(4, store.clone(), log_streamer.clone());

        Ok(Self {
            store,
            log_streamer,
            zombie_detector,
            error_recovery,
            worker_pool,
        })
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Start background services
        let zombie_detector = self.zombie_detector.clone();
        tokio::spawn(async move {
            zombie_detector.start_heartbeat_monitoring().await;
        });

        // Start worker pool
        self.worker_pool.start().await?;

        // Start web server for APIs and SSE
        self.start_web_server().await?;

        Ok(())
    }

    async fn start_web_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/jobs", post(create_job))
            .route("/jobs/:id", get(get_job))
            .route("/jobs/:id/logs", get(stream_job_logs))
            .route("/jobs/:id/ws", get(websocket_handler))
            .with_state(AppState {
                store: self.store.clone(),
                log_streamer: self.log_streamer.clone(),
            });

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

// API handlers
async fn create_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<Json<CreateJobResponse>, AppError> {
    let job_id = state.store.create_job(&payload.script_content).await?;
    Ok(Json(CreateJobResponse { job_id }))
}

#[derive(Serialize)]
struct CreateJobResponse {
    job_id: Uuid,
}

#[derive(Deserialize)]
struct CreateJobRequest {
    script_content: String,
}
```

### Testing the Complete System

```rust
#[tokio::test]
async fn test_complete_job_execution() {
    // Start mini windmill
    let windmill = MiniWindmill::new("postgresql://localhost/test").await.unwrap();
    
    // Create a job
    let job_id = windmill.store.create_job("print('Hello, World!')").await.unwrap();
    
    // Set up log streaming
    let mut log_stream = windmill.log_streamer.stream_logs(job_id).await;
    
    // Job should be picked up by worker and executed
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Check job status
    let job = windmill.store.get_job(job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    
    // Verify logs were streamed
    let log_count = log_stream.take(10).count().await;
    assert!(log_count > 0);
}

#[tokio::test]
async fn test_zombie_job_recovery() {
    let windmill = MiniWindmill::new("postgresql://localhost/test").await.unwrap();
    
    // Create a job and manually mark it as running (simulate crashed worker)
    let job_id = windmill.store.create_job("sleep(60)").await.unwrap();
    windmill.store.transition_to_running(job_id).await.unwrap();
    
    // Simulate time passing beyond zombie threshold
    // (In real test, we'd mock the time or adjust thresholds)
    
    // Trigger zombie detection
    windmill.zombie_detector.detect_and_recover_zombies().await.unwrap();
    
    // Job should be back in queue
    let job = windmill.store.get_job(job_id).await.unwrap();
    assert_eq!(job.status, JobStatus::Queued);
}
```

---

## Key Engineering Insights

### 1. **State Management is Critical**

The finite state machine approach provides:
- **Predictability**: Valid transitions are defined upfront
- **Debuggability**: Current state is always clear
- **Consistency**: Prevents impossible states

### 2. **Database Design for Performance**

- **Indexes**: Critical for job queue performance
- **SKIP LOCKED**: Enables efficient distributed polling
- **Transactions**: Ensure state consistency
- **JSONB**: Flexible storage for job metadata

### 3. **Observability from Day One**

- **Event Streams**: Every state change emits events
- **Structured Logging**: Machine-readable log format
- **Metrics**: Performance monitoring built-in
- **Real-time Streaming**: Immediate feedback

### 4. **Failure is Normal**

- **Zombie Detection**: Workers will crash
- **Retry Logic**: Transient failures are common
- **Circuit Breakers**: Prevent cascading failures
- **Dead Letter Queues**: Some jobs will never succeed

### 5. **Performance Through Design**

- **Async Everything**: Non-blocking operations
- **Connection Pooling**: Database efficiency
- **Batch Operations**: Reduce I/O overhead
- **Smart Polling**: Avoid busy waiting

---

## Next Steps: Scaling Beyond Mini-Windmill

### Horizontal Scaling Challenges

1. **Distributed Locking**: Use PostgreSQL advisory locks or Redis
2. **Leader Election**: Coordinate zombie detection across instances
3. **Sharding**: Partition jobs across multiple databases
4. **Load Balancing**: Distribute HTTP and WebSocket connections

### Advanced Features

1. **Priority Queues**: Weighted job scheduling
2. **Resource Constraints**: CPU/Memory limits per job
3. **Dependency Management**: Job chains and DAGs
4. **Multi-tenancy**: Workspace isolation
5. **Audit Logging**: Compliance and debugging

### Production Considerations

1. **Monitoring**: Prometheus metrics and Grafana dashboards
2. **Security**: Authentication, authorization, and sandboxing
3. **Backup**: Database and log retention policies
4. **Deployment**: Container orchestration and blue-green deploys

---

## Conclusion

We've built a simplified but functional job execution system that demonstrates the core principles behind Windmill's architecture. The key takeaways:

1. **Start Simple**: Begin with a clear state machine
2. **Add Persistence**: Database design drives performance
3. **Handle Failures**: Zombie detection and recovery are essential
4. **Enable Observability**: Real-time logs and metrics are crucial
5. **Design for Scale**: Async, batched, and distributed from the start

This foundation provides the building blocks for sophisticated workflow orchestration systems. The patterns we've explored - state machines, event sourcing, circuit breakers, and real-time streaming - are fundamental to building reliable distributed systems.

Remember: **Complexity is the enemy of reliability**. Start with the simplest solution that works, then evolve based on real-world requirements.

---

*Happy building! 🚀*