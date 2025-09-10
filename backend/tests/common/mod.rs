#![allow(dead_code)]

use std::{future::Future, str::FromStr, sync::Arc};

use futures::Stream;
use serde::Serialize;
use serde_json::json;
use sqlx::{postgres::PgListener, Pool, Postgres};
use tokio::sync::RwLock;
use uuid::Uuid;
use windmill_api_client::types::NewScript;
#[cfg(feature = "python")]
use windmill_common::flow_status::FlowStatusModule;
use windmill_common::{jobs::{JobKind, JobPayload, RawCode}, jwt::JWT_SECRET, scripts::{ ScriptHash, ScriptLang}, worker::WORKER_CONFIG, KillpillSender};
use windmill_queue::PushIsolationLevel;

/// it's important this is unique between tests as there is one prometheus registry and
/// run_worker shouldn't register the same metric with the same worker name more than once.
///
/// this must fit in varchar(50)
fn next_worker_name() -> String {
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

    static ID: AtomicUsize = AtomicUsize::new(0);

    // n.b.: when tests are run with RUST_TEST_THREADS or --test-threads set to 1, the name
    // will be "main"... The id provides uniqueness & thread_name gives context.
    let id = ID.fetch_add(1, SeqCst);
    let thread = std::thread::current();
    let thread_name = thread
        .name()
        .map(|s| {
            s.len()
                .checked_sub(39)
                .and_then(|start| s.get(start..))
                .unwrap_or(s)
        })
        .unwrap_or("no thread name");
    format!("{id}/worker-{thread_name}")
}

pub struct ApiServer {
    pub addr: std::net::SocketAddr,
    #[allow(unused)]
    tx: tokio::sync::broadcast::Sender<()>,
    #[allow(unused)]
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl ApiServer {
    pub async fn start(db: Pool<Postgres>) -> anyhow::Result<Self> {
        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let sock = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| anyhow::anyhow!("failed to bind TCP listener: {}", e))?;

        let addr = sock
            .local_addr()
            .map_err(|e| anyhow::anyhow!("failed to get local address: {}", e))?;
        drop(sock);
        let (port_tx, _port_rx) = tokio::sync::oneshot::channel::<String>();
        let name = next_worker_name();
        tracing::info!("starting api server for name={name}");
        let task = tokio::task::spawn(windmill_api::run_server(
            db.clone(),
            None,
            None,
            addr,
            rx,
            port_tx,
            false,
            false,
            format!("http://localhost:{}", addr.port()),
            Some(name.clone()),
        ));

        tracing::info!("waiting for server port for name={name}");
        _port_rx.await.map_err(|e| {
            tracing::error!("failed to receive port for name={name}: {e}");
            anyhow::anyhow!("failed to receive port for name={name}: {e}")
        })?;

        // clear the cache between tests
        windmill_common::cache::clear();

        Ok(Self { addr, tx, task })
    }

    #[allow(unused)]
    pub async fn close(self) -> anyhow::Result<()> {
        println!("closing api server");
        let Self { tx, task, .. } = self;
        drop(tx);
        task.await.unwrap()
    }
}


pub struct RunJob {
    pub payload: JobPayload,
    pub args: serde_json::Map<String, serde_json::Value>,
}

impl From<JobPayload> for RunJob {
    fn from(payload: JobPayload) -> Self {
        Self { payload, args: Default::default() }
    }
}

impl RunJob {
    pub fn arg<S: Into<String>>(mut self, k: S, v: serde_json::Value) -> Self {
        self.args.insert(k.into(), v);
        self
    }

    pub async fn push(self, db: &Pool<Postgres>) -> Uuid {
        let RunJob { payload, args } = self;
        let mut hm_args = std::collections::HashMap::new();
        for (k, v) in args {
            hm_args.insert(k, windmill_common::worker::to_raw_value(&v));
        }

        let tx = PushIsolationLevel::IsolatedRoot(db.clone());
        let (uuid, tx) = windmill_queue::push(
            &db,
            tx,
            "test-workspace",
            payload,
            windmill_queue::PushArgs::from(&hm_args),
            /* user */ "test-user",
            /* email  */ "test@windmill.dev",
            /* permissioned_as */ "u/test-user".to_string(),
            /* token_prefix */ None,
            /* scheduled_for_o */ None,
            /* schedule_path */ None,
            /* parent_job */ None,
            /* root job  */ None,
            /* flow_innermost_root_job */ None,
            /* job_id */ None,
            /* is_flow_step */ false,
            /* same_worker */ false,
            None,
            true,
            None,
            None,
            None,
            None,
            None,
            false,
        )
        .await
        .expect("push has to succeed");
        tx.commit().await.unwrap();

        uuid
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    pub async fn run_until_complete(self, db: &Pool<Postgres>, port: u16) -> CompletedJob {
        let uuid = self.push(db).await;
        let listener = listen_for_completed_jobs(db).await;
        in_test_worker(db, listener.find(&uuid), port).await;
        let r = completed_job(uuid, db).await;
        r
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    pub async fn run_until_complete_with<F: Future<Output = ()>>(
        self,
        db: &Pool<Postgres>,
        port: u16,
        test: impl Fn(Uuid) -> F,
    ) -> CompletedJob {
        let uuid = self.push(db).await;
        let listener = listen_for_completed_jobs(db).await;
        test(uuid).await;
        in_test_worker(db, listener.find(&uuid), port).await;
        let r = completed_job(uuid, db).await;
        r
    }
}

pub async fn run_job_in_new_worker_until_complete(
    db: &Pool<Postgres>,
    job: JobPayload,
    port: u16,
) -> CompletedJob {
    RunJob::from(job).run_until_complete(db, port).await
}

/// Start a worker with a timeout and run a future, until the worker quits or we time out.
///
/// Cleans up the worker before resolving.
pub async fn in_test_worker<Fut: std::future::Future>(
    db: &Pool<Postgres>,
    inner: Fut,
    port: u16,
) -> <Fut as std::future::Future>::Output {
    set_jwt_secret().await;
    let (quit, worker) = spawn_test_worker(db, port);
    let worker = tokio::time::timeout(std::time::Duration::from_secs(60), worker);
    tokio::pin!(worker);

    let res = tokio::select! {
        biased;
        res = inner => res,
        res = &mut worker => match
            res.expect("worker timed out")
               .expect("worker panicked") {
            _ => panic!("worker quit early"),
        },
    };

    /* ensure the worker quits before we return */
    quit.send();

    let _: () = worker
        .await
        .expect("worker timed out")
        .expect("worker panicked");
    res
}

pub fn spawn_test_worker(
    db: &Pool<Postgres>,
    port: u16,
) -> (KillpillSender, tokio::task::JoinHandle<()>) {
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(windmill_worker::GO_BIN_CACHE_DIR)
        .expect("could not create initial worker dir");

    let (tx, rx) = KillpillSender::new(1);
    let db = db.to_owned();
    let worker_instance: &str = "test worker instance";
    let worker_name: String = next_worker_name();
    let ip: &str = Default::default();

    let tx2 = tx.clone();
    let future = async move {
        let base_internal_url = format!("http://localhost:{}", port);
        {
            let mut wc = WORKER_CONFIG.write().await;
            (*wc).worker_tags = windmill_common::worker::DEFAULT_TAGS.clone();
            (*wc).priority_tags_sorted = vec![windmill_common::worker::PriorityTags {
                priority: 0,
                tags: (*wc).worker_tags.clone(),
            }];
            windmill_common::worker::store_suspended_pull_query(&wc).await;
            windmill_common::worker::store_pull_query(&wc).await;
        }
        windmill_worker::run_worker(
            &db.into(),
            worker_instance,
            worker_name,
            1,
            1,
            ip,
            rx,
            tx2,
            &base_internal_url,
        )
        .await
    };

    (tx, tokio::task::spawn(future))
}

pub async fn listen_for_completed_jobs(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "completed").await
}

#[cfg(feature = "deno_core")]
pub async fn listen_for_queue(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "queued").await
}

pub async fn listen_for_uuid_on(
    db: &Pool<Postgres>,
    channel: &'static str,
) -> impl Stream<Item = Uuid> + Unpin {
    let mut listener = PgListener::connect_with(db).await.unwrap();
    listener.listen(channel).await.unwrap();

    Box::pin(futures::stream::unfold(listener, |mut listener| async move {
        let uuid = listener
            .try_recv()
            .await
            .unwrap()
            .expect("lost database connection")
            .payload()
            .parse::<Uuid>()
            .expect("invalid uuid");
        Some((uuid, listener))
    }))
}

pub async fn completed_job(uuid: Uuid, db: &Pool<Postgres>) -> CompletedJob {
    sqlx::query_as::<_, CompletedJob>(
        "SELECT *, result->'wm_labels' as labels FROM v2_as_completed_job  WHERE id = $1",
    )
    .bind(uuid)
    .fetch_one(db)
    .await
    .unwrap()
}

#[axum::async_trait(?Send)]
pub trait StreamFind: futures::Stream + Unpin + Sized {
    async fn find(self, item: &Self::Item) -> Option<Self::Item>
    where
        for<'l> &'l Self::Item: std::cmp::PartialEq,
    {
        use futures::{future::ready, StreamExt};

        self.filter(|i| ready(i == item)).next().await
    }
}

impl<T: futures::Stream + Unpin + Sized> StreamFind for T {}


#[cfg(feature = "python")]
pub fn get_module(cjob: &CompletedJob, id: &str) -> Option<FlowStatusModule> {
    cjob.flow_status.clone().and_then(|fs| {
        use windmill_common::flow_status::FlowStatus;

        find_module_in_vec(
            serde_json::from_value::<FlowStatus>(fs).unwrap().modules,
            id,
        )
    })
}

#[cfg(feature = "python")]
fn find_module_in_vec(modules: Vec<FlowStatusModule>, id: &str) -> Option<FlowStatusModule> {
    modules.into_iter().find(|s| s.id() == id)
}

pub async fn set_jwt_secret() -> () {
    let secret = "mytestsecret".to_string();
    let mut l = JWT_SECRET.write().await;
    *l = secret;
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct CompletedJob {
    pub workspace_id: String,
    pub id: Uuid,
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: i64,
    pub success: bool,
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub logs: Option<String>,
    pub deleted: bool,
    pub raw_code: Option<String>,
    pub canceled: bool,
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    pub flow_status: Option<serde_json::Value>,
    pub raw_flow: Option<serde_json::Value>,
    pub is_flow_step: bool,
    pub is_skipped: bool,
    pub email: String,
    pub visible_to_owner: bool,
    pub mem_peak: Option<i32>,
    pub tag: String,
    pub script_hash: Option<ScriptHash>,
    pub language: Option<ScriptLang>,
    pub job_kind: JobKind,
}

impl CompletedJob {
    pub fn json_result(&self) -> Option<serde_json::Value> {
        self.result.clone()
    }
}

pub async fn initialize_tracing() {
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = windmill_common::tracing_init::initialize_tracing(
            "test",
            &windmill_common::utils::Mode::Standalone,
            "test",
        );
    });
}

pub async fn test_for_versions<F: Future<Output = ()>>(
    version_flags: impl Iterator<Item = Arc<RwLock<bool>>>,
    test: impl Fn() -> F,
) {
    for version_flag in version_flags {
        *version_flag.write().await = true;
        test().await;
    }
}

use futures::StreamExt;

// #[cfg(feature = "python")]
pub async fn assert_lockfile(
    db: &Pool<Postgres>,
    script_content: String,
    language: ScriptLang,
    expected_lines: Vec<&str>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    client
        .create_script(
            "test-workspace",
            &NewScript {
                language: windmill_api_client::types::ScriptLang::from_str(language.as_str()).unwrap(),
                content: script_content,
                path: "f/system/test_import".to_string(),
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                description: "".to_string(),
                draft_only: None,
                envs: vec![],
                is_template: None,
                kind: None,
                parent_hash: None,
                lock: None,
                summary: "".to_string(),
                tag: None,
                schema: std::collections::HashMap::new(),
                ws_error_handler_muted: Some(false),
                priority: None,
                delete_after_use: None,
                timeout: None,
                restart_unless_cancelled: None,
                deployment_message: None,
                concurrency_key: None,
                visible_to_runner_only: None,
                no_main_func: None,
                codebase: None,
                has_preprocessor: None,
                on_behalf_of_email: None,
                assets: vec![],
            },
        )
        .await
        .unwrap();

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            completed.next().await; // deployed script

            let script = sqlx::query!(
                "SELECT hash FROM script WHERE path = $1",
                "f/system/test_import".to_string()
            )
            .fetch_one(&db2)
            .await
            .unwrap();

            let job = RunJob::from(JobPayload::Dependencies {
                path: "f/system/test_import".to_string(),
                hash: ScriptHash(script.hash),
                dedicated_worker: None,
                language,
            })
            .push(&db2)
            .await;

            completed.next().await; // completed job

            let result = completed_job(job, &db2).await.json_result().unwrap();

            assert_eq!(
                result,
                json!({
                    "lock": expected_lines.join("\n"),
                    "status": "Successful lock file generation"
                })
            );
        },
        port,
    )
    .await;

    Ok(())
}



pub async fn run_deployed_relative_imports(
    db: &Pool<Postgres>,
    script_content: String,
    language: ScriptLang,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    client
        .create_script(
            "test-workspace",
            &NewScript {
                language: windmill_api_client::types::ScriptLang::from_str(language.as_str()).unwrap(),
                content: script_content,
                path: "f/system/test_import".to_string(),
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                description: "".to_string(),
                draft_only: None,
                envs: vec![],
                is_template: None,
                kind: None,
                parent_hash: None,
                lock: None,
                summary: "".to_string(),
                tag: None,
                schema: std::collections::HashMap::new(),
                ws_error_handler_muted: Some(false),
                priority: None,
                delete_after_use: None,
                timeout: None,
                restart_unless_cancelled: None,
                deployment_message: None,
                concurrency_key: None,
                visible_to_runner_only: None,
                no_main_func: None,
                codebase: None,
                has_preprocessor: None,
                on_behalf_of_email: None,
                assets: vec![],
            },
        )
        .await
        .unwrap();

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            completed.next().await; // deployed script

            let script = sqlx::query!(
                "SELECT hash FROM script WHERE path = $1",
                "f/system/test_import".to_string()
            )
            .fetch_one(&db2)
            .await
            .unwrap();

            let job = RunJob::from(JobPayload::ScriptHash {
                path: "f/system/test_import".to_string(),
                hash: ScriptHash(script.hash),
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                language,
                priority: None,
                apply_preprocessor: false,
            })
            .push(&db2)
            .await;

            completed.next().await; // completed job

            let result = completed_job(job, &db2).await.json_result().unwrap();

            assert_eq!(
                result,
                serde_json::json!([
                    "f/system/same_folder_script",
                    "f/system/same_folder_script",
                    "f/system_relative/different_folder_script",
                    "f/system_relative/different_folder_script"
                ])
            );
        },
        port,
    )
    .await;

    Ok(())
}

pub async fn run_preview_relative_imports(
    db: &Pool<Postgres>,
    script_content: String,
    language: ScriptLang,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let mut completed = listen_for_completed_jobs(&db).await;
    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            let job = RunJob::from(JobPayload::Code(RawCode {
                hash: None,
                content: script_content,
                path: Some("f/system/test_import".to_string()),
                language,
                lock: None,
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
            }))
            .push(&db2)
            .await;

            completed.next().await; // completed job

            let result = completed_job(job, &db2).await.json_result().unwrap();

            assert_eq!(
                result,
                serde_json::json!([
                    "f/system/same_folder_script",
                    "f/system/same_folder_script",
                    "f/system_relative/different_folder_script",
                    "f/system_relative/different_folder_script"
                ])
            );
        },
        port,
    )
    .await;

    Ok(())
}