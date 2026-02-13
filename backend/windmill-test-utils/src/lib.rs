#![allow(dead_code)]

use std::{future::Future, str::FromStr};

use futures::Stream;
use serde::Serialize;
use serde_json::json;
use sqlx::{postgres::PgListener, Pool, Postgres};
use uuid::Uuid;
use windmill_api_client::types::NewScript;
use windmill_common::flow_status::FlowStatusModule;
use windmill_common::{
    jobs::{JobKind, JobPayload, RawCode},
    jwt::JWT_SECRET,
    scripts::{ScriptHash, ScriptLang},
    worker::{Connection, WORKER_CONFIG},
    KillpillSender,
};
use windmill_queue::PushIsolationLevel;

pub async fn init_client(db: Pool<Postgres>) -> (windmill_api_client::Client, u16, ApiServer) {
    initialize_tracing().await;
    let server = ApiServer::start(db).await.unwrap();
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    (client, port, server)
}

pub async fn init_client_agent_mode(
    db: Pool<Postgres>,
) -> (windmill_api_client::Client, u16, ApiServer) {
    initialize_tracing().await;
    set_jwt_secret().await;

    let server = ApiServer::start_agent_mode(db).await.unwrap();
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );
    (client, port, server)
}

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
        Self::start_inner(db, false).await
    }

    pub async fn start_agent_mode(db: Pool<Postgres>) -> anyhow::Result<Self> {
        Self::start_inner(db, true).await
    }

    /// Start the API server with server_mode=true so trigger listeners are active.
    /// Alias for `start_agent_mode` with a clearer name for trigger e2e tests.
    pub async fn start_with_listeners(db: Pool<Postgres>) -> anyhow::Result<Self> {
        Self::start_inner(db, true).await
    }

    async fn start_inner(db: Pool<Postgres>, agent_mode: bool) -> anyhow::Result<Self> {
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
            agent_mode,
            false,
            format!("http://localhost:{}", addr.port()),
            Some(name.clone()),
        ));

        tracing::info!("waiting for server port for name={name}");
        if let Err(e) = _port_rx.await {
            tracing::error!("failed to receive port for name={name}: {e}");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            return Err(anyhow::anyhow!(
                "failed to receive port for name={name}: {e}"
            ));
        }

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

#[derive(Debug, Clone)]
pub struct RunJob {
    pub payload: JobPayload,
    pub args: serde_json::Map<String, serde_json::Value>,
    pub scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    pub email: String,
}

impl From<JobPayload> for RunJob {
    fn from(payload: JobPayload) -> Self {
        Self {
            payload,
            args: Default::default(),
            scheduled_for_o: None,
            email: "test@windmill.dev".to_string(),
        }
    }
}

impl RunJob {
    pub fn arg<S: Into<String>>(mut self, k: S, v: serde_json::Value) -> Self {
        self.args.insert(k.into(), v);
        self
    }

    pub fn push_arg_scheduled_for_o(
        mut self,
        scheduled_for_o: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.scheduled_for_o = scheduled_for_o;
        self
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    pub async fn push(self, db: &Pool<Postgres>) -> Uuid {
        let RunJob { payload, args, scheduled_for_o, email } = self;
        let mut hm_args = std::collections::HashMap::new();
        for (k, v) in args {
            hm_args.insert(k, windmill_common::worker::to_raw_value(&v));
        }

        let tx = PushIsolationLevel::IsolatedRoot(db.clone());
        let (uuid, tx) = windmill_queue::push(
            db,
            tx,
            "test-workspace",
            payload,
            windmill_queue::PushArgs::from(&hm_args),
            /* user */ "test-user",
            /* email  */ &email,
            /* permissioned_as */ "u/test-user".to_string(),
            /* token_prefix */ None,
            scheduled_for_o,
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
            None,
            None,
            None,
        )
        .await
        .expect("push has to succeed");
        tx.commit().await.unwrap();

        uuid
    }

    /// Push the job as a specific user (for testing permissions)
    pub async fn push_as(self, db: &Pool<Postgres>, username: &str, email: &str) -> Uuid {
        let RunJob { payload, args, scheduled_for_o, .. } = self;
        let mut hm_args = std::collections::HashMap::new();
        for (k, v) in args {
            hm_args.insert(k, windmill_common::worker::to_raw_value(&v));
        }

        let tx = PushIsolationLevel::IsolatedRoot(db.clone());
        let (uuid, tx) = windmill_queue::push(
            db,
            tx,
            "test-workspace",
            payload,
            windmill_queue::PushArgs::from(&hm_args),
            username,
            email,
            format!("u/{}", username),
            /* token_prefix */ None,
            scheduled_for_o,
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
            None,
            None,
            None,
        )
        .await
        .expect("push has to succeed");
        tx.commit().await.unwrap();

        uuid
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    pub async fn run_until_complete(
        self,
        db: &Pool<Postgres>,
        agent_mode: bool,
        port: u16,
    ) -> CompletedJob {
        let uuid = self.push(db).await;
        let listener = listen_for_completed_jobs(db).await;

        let conn = match agent_mode {
            false => Connection::Sql(db.clone()),
            #[cfg(all(feature = "private", feature = "agent_worker_server"))]
            true => testing_http_connection(port).await,
            #[cfg(not(all(feature = "private", feature = "agent_worker_server")))]
            true => {
                panic!("to use agent worker test, you need to enable 'agent_worker_server' feature")
            }
        };

        in_test_worker(conn, listener.find(&uuid), port).await;

        completed_job(uuid, db).await
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    pub async fn run_until_complete_with<F: Future<Output = ()>>(
        self,
        db: &Pool<Postgres>,
        agent_mode: bool,
        port: u16,
        test: impl Fn(Uuid) -> F,
    ) -> CompletedJob {
        let uuid = self.push(db).await;
        let listener = listen_for_completed_jobs(db).await;
        test(uuid).await;

        let conn = match agent_mode {
            false => Connection::Sql(db.clone()),
            #[cfg(all(feature = "private", feature = "agent_worker_server"))]
            true => testing_http_connection(port).await,
            #[cfg(not(all(feature = "private", feature = "agent_worker_server")))]
            true => {
                panic!("to use agent worker test, you need to enable 'agent_worker_server' feature")
            }
        };

        in_test_worker(conn, listener.find(&uuid), port).await;

        completed_job(uuid, db).await
    }
}

pub async fn run_job_in_new_worker_until_complete(
    db: &Pool<Postgres>,
    agent_mode: bool,
    job: JobPayload,
    port: u16,
) -> CompletedJob {
    RunJob::from(job)
        .run_until_complete(db, agent_mode, port)
        .await
}

/// Start a worker with a timeout and run a future, until the worker quits or we time out.
///
/// Cleans up the worker before resolving.
pub async fn in_test_worker<Fut: std::future::Future>(
    // db: &Pool<Postgres>,
    // If set to http, worker will be started in agent mode.
    conn: impl Into<Connection>,
    inner: Fut,
    port: u16,
) -> <Fut as std::future::Future>::Output {
    set_jwt_secret().await;
    let (quit, worker) = spawn_test_worker(&conn.into(), port);
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
    conn: &Connection,
    port: u16,
) -> (KillpillSender, tokio::task::JoinHandle<()>) {
    #[cfg(feature = "deno_core")]
    windmill_runtime_nativets::setup_deno_runtime().expect("V8 init failed");

    std::fs::DirBuilder::new()
        .recursive(true)
        .create(windmill_worker::GO_BIN_CACHE_DIR)
        .expect("could not create initial worker dir");

    let (tx, rx) = KillpillSender::new(1);
    let worker_instance: &str = "test worker instance";
    let worker_name: String = next_worker_name();
    let ip: &str = Default::default();
    let conn = conn.to_owned();

    let tx2 = tx.clone();
    let future = async move {
        let base_internal_url = format!("http://localhost:{}", port);
        {
            let mut wc = WORKER_CONFIG.write().await;
            wc.worker_tags = windmill_common::worker::DEFAULT_TAGS.clone();
            wc.priority_tags_sorted = vec![windmill_common::worker::PriorityTags {
                priority: 0,
                tags: wc.worker_tags.clone(),
            }];
            windmill_common::worker::store_suspended_pull_query(&wc).await;
            windmill_common::worker::store_pull_query(&wc).await;
        }
        windmill_worker::run_worker(
            &conn,
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

pub async fn listen_for_queue(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "queued").await
}

pub async fn listen_for_uuid_on(
    db: &Pool<Postgres>,
    channel: &'static str,
) -> impl Stream<Item = Uuid> + Unpin {
    let mut listener = PgListener::connect_with(db).await.unwrap();
    listener.listen(channel).await.unwrap();

    Box::pin(futures::stream::unfold(
        listener,
        |mut listener| async move {
            let uuid = listener
                .try_recv()
                .await
                .unwrap()
                .expect("lost database connection")
                .payload()
                .parse::<Uuid>()
                .expect("invalid uuid");
            Some((uuid, listener))
        },
    ))
}

pub async fn completed_job(uuid: Uuid, db: &Pool<Postgres>) -> CompletedJob {
    sqlx::query_as::<_, CompletedJob>(
        "SELECT j.id, j.workspace_id, j.parent_job, j.created_by, j.created_at, c.duration_ms,
         c.status = 'success' OR c.status = 'skipped' AS success, j.runnable_id AS script_hash, j.runnable_path AS script_path,
         j.args, c.result, FALSE AS deleted, j.raw_code, c.status = 'canceled' AS canceled,
         c.canceled_by, c.canceled_reason, j.kind AS job_kind,
         CASE WHEN j.trigger_kind = 'schedule'::job_trigger_kind THEN j.trigger END AS schedule_path,
         j.permissioned_as, COALESCE(c.flow_status, c.workflow_as_code_status) AS flow_status, j.raw_flow,
         j.flow_step_id IS NOT NULL AS is_flow_step, j.script_lang AS language, c.started_at,
         c.status = 'skipped' AS is_skipped, j.raw_lock, j.permissioned_as_email AS email, j.visible_to_owner,
         c.memory_peak AS mem_peak, j.tag, j.priority, NULL::TEXT AS logs, c.result_columns,
         j.script_entrypoint_override, j.preprocessed, c.result->'wm_labels' as labels
         FROM v2_job_completed c JOIN v2_job j USING (id) WHERE j.id = $1",
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

pub fn get_module(cjob: &CompletedJob, id: &str) -> Option<FlowStatusModule> {
    cjob.flow_status.clone().and_then(|fs| {
        use windmill_common::flow_status::FlowStatus;

        find_module_in_vec(
            serde_json::from_value::<FlowStatus>(fs).unwrap().modules,
            id,
        )
    })
}

fn find_module_in_vec(modules: Vec<FlowStatusModule>, id: &str) -> Option<FlowStatusModule> {
    modules.into_iter().find(|s| s.id() == id)
}

pub async fn set_jwt_secret() {
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
    constraints: impl Iterator<Item = &'static windmill_common::min_version::VersionConstraint>,
    test: impl Fn() -> F,
) {
    for constraint in constraints {
        *windmill_common::min_version::MIN_VERSION.write().await = constraint.version().clone();
        test().await;
    }
}

use futures::StreamExt;

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
                language: windmill_api_client::types::ScriptLang::from_str(language.as_str())
                    .unwrap(),
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

    let mut completed = listen_for_completed_jobs(db).await;
    let db2 = db.clone();
    in_test_worker(
        db,
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
                debouncing_settings: Default::default(),
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
                language: windmill_api_client::types::ScriptLang::from_str(language.as_str())
                    .unwrap(),
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

    let mut completed = listen_for_completed_jobs(db).await;
    let db2 = db.clone();
    in_test_worker(
        db,
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
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language,
                priority: None,
                apply_preprocessor: false,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
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

    let mut completed = listen_for_completed_jobs(db).await;
    let db2 = db.clone();
    in_test_worker(
        db.clone(),
        async move {
            let job = RunJob::from(JobPayload::Code(RawCode {
                hash: None,
                content: script_content,
                path: Some("f/system/test_import".to_string()),
                language,
                lock: None,
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
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

#[cfg(all(feature = "private", feature = "agent_worker_server"))]
pub async fn testing_http_connection(port: u16) -> Connection {
    let suffix = windmill_common::utils::create_default_worker_suffix("test-agent-worker");
    let agent_token = format!(
        "{}{}",
        windmill_common::agent_workers::AGENT_JWT_PREFIX,
        windmill_common::jwt::encode_with_internal_secret(windmill_api_agent_workers::AgentAuth {
            worker_group: "testing-agent".to_owned(),
            suffix: Some(suffix.clone()),
            tags: vec!["flow".into(), "python3".into(), "dependency".into()],
            exp: Some(usize::MAX),
        })
        .await
        .expect("JWT token to be created")
    );
    let base_internal_url = format!("http://localhost:{port}");
    Connection::Http(windmill_common::agent_workers::build_agent_http_client(
        &suffix,
        &agent_token,
        &base_internal_url,
    ))
}

/// IMPORTANT!:
/// Do not run parallel in tests!
///
/// No tests can run this at the same time, will result into conflicts!!!
pub async fn rebuild_dmap(client: &windmill_api_client::Client) -> bool {
    client
        .client()
        .post(format!(
            "{}/w/test-workspace/workspaces/rebuild_dependency_map",
            client.baseurl()
        ))
        .send()
        .await
        .unwrap()
        .status()
        .is_success()
}

// ============================================================================
// Dedicated Worker Protocol Helpers
// ============================================================================

/// Result from parsing a dedicated worker stdout line
#[derive(Debug, Clone, PartialEq)]
pub enum DedicatedWorkerResult {
    /// Worker printed "start" indicating it's ready
    Start,
    /// Worker returned a successful result
    Success(serde_json::Value),
    /// Worker returned an error result
    Error(serde_json::Value),
    /// Line is not a protocol message (e.g., logs)
    Other(String),
}

/// Parse a line from dedicated worker stdout according to the protocol:
/// - "start" -> Ready signal
/// - "wm_res[success]:JSON" -> Success with result
/// - "wm_res[error]:JSON" -> Error with details
/// - anything else -> Other (logs)
pub fn parse_dedicated_worker_line(line: &str) -> DedicatedWorkerResult {
    if line == "start" {
        return DedicatedWorkerResult::Start;
    }

    if let Some(json_str) = line.strip_prefix("wm_res[success]:") {
        match serde_json::from_str(json_str) {
            Ok(value) => return DedicatedWorkerResult::Success(value),
            Err(_) => return DedicatedWorkerResult::Other(line.to_string()),
        }
    }

    if let Some(json_str) = line.strip_prefix("wm_res[error]:") {
        match serde_json::from_str(json_str) {
            Ok(value) => return DedicatedWorkerResult::Error(value),
            Err(_) => return DedicatedWorkerResult::Other(line.to_string()),
        }
    }

    DedicatedWorkerResult::Other(line.to_string())
}
