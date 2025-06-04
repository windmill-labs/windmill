use serde::de::DeserializeOwned;
use std::future::Future;
use std::{str::FromStr, sync::Arc};
use windmill_common::KillpillSender;

#[cfg(feature = "enterprise")]
use chrono::Timelike;
use futures::StreamExt;

use futures::{stream, Stream};
use serde::Deserialize;
use serde_json::json;
use sqlx::{postgres::PgListener, types::Uuid, Pool, Postgres};

use tokio::sync::RwLock;
#[cfg(feature = "enterprise")]
use tokio::time::{timeout, Duration};

#[cfg(feature = "python")]
use windmill_api_client::types::{CreateFlowBody, RawScript};
#[cfg(feature = "enterprise")]
use windmill_api_client::types::{EditSchedule, NewSchedule, ScriptArgs};
use windmill_api_client::types::{NewScript, ScriptLang as NewScriptLanguage};

use serde::Serialize;
#[cfg(feature = "deno_core")]
use windmill_common::flows::InputTransform;
use windmill_common::worker::WORKER_CONFIG;

use windmill_common::{
    flow_status::{FlowStatus, FlowStatusModule, RestartedFrom},
    flows::{FlowModule, FlowModuleValue, FlowValue},
    jobs::{JobKind, JobPayload, RawCode},
    jwt::JWT_SECRET,
    scripts::{ScriptHash, ScriptLang},
    worker::{
        MIN_VERSION_IS_AT_LEAST_1_427, MIN_VERSION_IS_AT_LEAST_1_432, MIN_VERSION_IS_AT_LEAST_1_440,
    },
};
use windmill_queue::PushIsolationLevel;

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

async fn initialize_tracing() {
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
    format!("{id}/{thread_name}")
}

pub struct ApiServer {
    pub addr: std::net::SocketAddr,
    tx: tokio::sync::broadcast::Sender<()>,
    task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl ApiServer {
    pub async fn start(db: Pool<Postgres>) -> Self {
        let (tx, rx) = tokio::sync::broadcast::channel::<()>(1);

        let sock = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

        let addr = sock.local_addr().unwrap();
        drop(sock);
        let (port_tx, _port_rx) = tokio::sync::oneshot::channel::<String>();

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
        ));

        _port_rx.await.expect("failed to receive port");

        // clear the cache between tests
        windmill_common::cache::clear();

        Self { addr, tx, task }
    }

    async fn close(self) -> anyhow::Result<()> {
        println!("closing api server");
        let Self { tx, task, .. } = self;
        drop(tx);
        task.await.unwrap()
    }
}

// async fn _print_job(id: Uuid, db: &Pool<Postgres>) -> Result<(), anyhow::Error> {
//     tracing::info!(
//         "{:#?}",
//         get_job_by_id(db.begin().await?, "test-workspace", id)
//             .await?
//             .0
//     );
//     Ok(())
// }

#[cfg(feature = "python")]
fn get_module(cjob: &CompletedJob, id: &str) -> Option<FlowStatusModule> {
    cjob.flow_status.clone().and_then(|fs| {
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

async fn set_jwt_secret() -> () {
    let secret = "mytestsecret".to_string();
    let mut l = JWT_SECRET.write().await;
    *l = secret;
}

mod suspend_resume {

    use serde_json::json;

    use super::*;

    async fn wait_until_flow_suspends(
        flow: Uuid,
        mut queue: impl Stream<Item = Uuid> + Unpin,
        db: &Pool<Postgres>,
    ) {
        loop {
            queue.by_ref().find(&flow).await.unwrap();
            if sqlx::query_scalar!(
                "SELECT suspend > 0 AS \"r!\" FROM v2_job_queue WHERE id = $1",
                flow
            )
            .fetch_one(db)
            .await
            .unwrap()
            {
                break;
            }
        }
    }

    fn flow() -> FlowValue {
        serde_json::from_value(serde_json::json!({
                "modules": [{
                    "id": "a",
                    "value": {
                        "input_transforms": {
                            "n": { "type": "javascript", "expr": "flow_input.n", },
                            "port": { "type": "javascript", "expr": "flow_input.port", },
                            "op": { "type": "javascript", "expr": "flow_input.op ?? 'resume'", },
                        },
                        "type": "rawscript",
                        "language": "deno",
                        "content": "\
                            export async function main(n, port, op) {\
                                const job = Deno.env.get('WM_JOB_ID');
                                const token = Deno.env.get('WM_TOKEN');
                                const r = await fetch(
                                    `http://localhost:${port}/api/w/test-workspace/jobs/job_signature/${job}/0?token=${token}&approver=ruben`,\
                                    {\
                                        method: 'GET',\
                                        headers: { 'Authorization': `Bearer ${token}` }\
                                    }\
                                );\
                                console.log(r);\
                                const secret = await r.text();\
                                console.log('Secret: ' + secret + ' ' + job + ' ' + token);\
                                const r2 = await fetch(
                                    `http://localhost:${port}/api/w/test-workspace/jobs_u/${op}/${job}/0/${secret}?approver=ruben`,\
                                    {\
                                        method: 'POST',\
                                        body: JSON.stringify('from job'),\
                                        headers: { 'content-type': 'application/json' }\
                                    }\
                                );\
                                console.log(await r2.text());\
                                return n + 1;\
                            }",
                    },
                    "suspend": {
                        "required_events": 1
                    },
                }, {
                    "id": "b",
                    "value": {
                        "input_transforms": {
                            "n": { "type": "javascript", "expr": "results.a", },
                            "resume": { "type": "javascript", "expr": "resume", },
                            "resumes": { "type": "javascript", "expr": "resumes", },
                        },
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(n, resume, resumes) { return { n: n + 1, resume, resumes } }"
                    },
                    "suspend": {
                        "required_events": 1
                    },
                }, {
                    "value": {
                        "input_transforms": {
                            "last": { "type": "javascript", "expr": "results.b", },
                            "resume": { "type": "javascript", "expr": "resume", },
                            "resumes": { "type": "javascript", "expr": "resumes", },
                        },
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(last, resume, resumes) { return { last, resume, resumes } }"
                    },
                }],
            }))
            .unwrap()
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test(db: Pool<Postgres>) {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let flow =
            RunJob::from(JobPayload::RawFlow { value: flow(), path: None, restarted_from: None })
                .arg("n", json!(1))
                .arg("port", json!(port))
                .push(&db)
                .await;

        let mut completed = listen_for_completed_jobs(&db).await;
        let queue = listen_for_queue(&db).await;
        let db_ = db.clone();

        in_test_worker(&db, async move {
                let db = db_;

                wait_until_flow_suspends(flow, queue, &db).await;
                // print_job(flow, &db).await;
                /* The first job resumes itself. */
                let _first = completed.next().await.unwrap();
                // print_job(_first, &db).await;

                /* ... and send a request resume it. */
                let second = completed.next().await.unwrap();
                // print_job(second, &db).await;

                let token = windmill_common::auth::create_token_for_owner(&db, "test-workspace", "u/test-user", "", 100, "", &Uuid::nil(), None).await.unwrap();
                let secret = reqwest::get(format!(
                    "http://localhost:{port}/api/w/test-workspace/jobs/job_signature/{second}/0?token={token}&approver=ruben"
                ))
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .text().await.unwrap();
                println!("{}", secret);

                /* ImZyb20gdGVzdCIK = base64 "from test" */
                reqwest::get(format!(
                    "http://localhost:{port}/api/w/test-workspace/jobs_u/resume/{second}/0/{secret}?payload=ImZyb20gdGVzdCIK&approver=ruben"
                ))
                .await
                .unwrap()
                .error_for_status()
                .unwrap();

                completed.find(&flow).await.unwrap();
            }, port)
            .await;

        server.close().await.unwrap();

        let result = completed_job(flow, &db).await.json_result().unwrap();

        assert_eq!(
            json!({
                "last": {
                    "resume": "from job",
                    "resumes": ["from job"],
                    "n": 3,
                },
                "resume": "from test",
                "resumes": ["from test"],
            }),
            result
        );

        // ensure resumes are cleaned up through CASCADE when the flow is finished
        assert_eq!(
            0,
            sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM resume_job")
                .fetch_one(&db)
                .await
                .unwrap()
        );
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn cancel_from_job(db: Pool<Postgres>) {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let result =
            RunJob::from(JobPayload::RawFlow { value: flow(), path: None, restarted_from: None })
                .arg("n", json!(1))
                .arg("op", json!("cancel"))
                .arg("port", json!(port))
                .run_until_complete(&db, port)
                .await
                .json_result()
                .unwrap();

        server.close().await.unwrap();

        assert_eq!(
            json!( {"error": {"name": "SuspendedDisapproved", "message": "Disapproved by ruben"}}),
            result
        );
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn cancel_after_suspend(db: Pool<Postgres>) {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let flow =
            RunJob::from(JobPayload::RawFlow { value: flow(), path: None, restarted_from: None })
                .arg("n", json!(1))
                .arg("port", json!(port))
                .push(&db)
                .await;

        let mut completed = listen_for_completed_jobs(&db).await;
        let queue = listen_for_queue(&db).await;
        let db_ = db.clone();

        in_test_worker(&db, async move {
                let db = db_;

                wait_until_flow_suspends(flow, queue, &db).await;
                /* The first job resumes itself. */
                let _first = completed.next().await.unwrap();
                /* ... and send a request resume it. */
                let second = completed.next().await.unwrap();

                let token = windmill_common::auth::create_token_for_owner(&db, "test-workspace", "u/test-user", "", 100, "", &Uuid::nil(), None).await.unwrap();
                let secret = reqwest::get(format!(
                    "http://localhost:{port}/api/w/test-workspace/jobs/job_signature/{second}/0?token={token}"
                ))
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .text().await.unwrap();
                println!("{}", secret);

                /* ImZyb20gdGVzdCIK = base64 "from test" */
                reqwest::get(format!(
                    "http://localhost:{port}/api/w/test-workspace/jobs_u/cancel/{second}/0/{secret}?payload=ImZyb20gdGVzdCIK"
                ))
                .await
                .unwrap()
                .error_for_status()
                .unwrap();

                completed.find(&flow).await.unwrap();
            }, port)
            .await;

        server.close().await.unwrap();

        let result = completed_job(flow, &db).await.json_result().unwrap();

        assert_eq!(
            json!( {"error": {"name": "SuspendedDisapproved", "message": "Disapproved by unknown"}}),
            result
        );
    }
}

mod retry {
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    /// test helper provides some external state to help steps fail at specific points
    struct Server {
        addr: std::net::SocketAddr,
        tx: tokio::sync::oneshot::Sender<()>,
        task: tokio::task::JoinHandle<Vec<u8>>,
    }

    impl Server {
        async fn start(responses: Vec<Option<u8>>) -> Self {
            use tokio::net::TcpListener;

            let (tx, rx) = tokio::sync::oneshot::channel::<()>();
            let sock = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = sock.local_addr().unwrap();

            let task = tokio::task::spawn(async move {
                tokio::pin!(rx);
                let mut results = vec![];

                for next in responses {
                    let (mut peer, _) = tokio::select! {
                        _ = &mut rx => break,
                        r = sock.accept() => r,
                    }
                    .unwrap();

                    let n = peer.read_u8().await.unwrap();
                    results.push(n);

                    if let Some(next) = next {
                        peer.write_u8(next).await.unwrap();
                    }
                }

                results
            });

            return Self { addr, tx, task };
        }

        async fn close(self) -> Vec<u8> {
            let Self { task, tx, .. } = self;
            drop(tx);
            task.await.unwrap()
        }
    }

    fn inner_step() -> &'static str {
        r#"
export async function main(index, port) {
    const buf = new Uint8Array([0]);
    const sock = await Deno.connect({ port });
    await sock.write(new Uint8Array([index]));
    if (await sock.read(buf) != 1) throw Error("read");
    return buf[0];
}
            "#
    }

    fn last_step() -> &'static str {
        r#"
def main(last, port):
    with __import__("socket").create_connection((None, port)) as sock:
        sock.send(b'\xff')
        return last + [sock.recv(1)[0]]
"#
    }

    fn flow_forloop_retry() -> FlowValue {
        serde_json::from_value(serde_json::json!({
            "modules": [{
                "id": "a",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "flow_input.items" },
                    "skip_failures": false,
                    "modules": [{
                        "value": {
                            "input_transforms": {
                                "index": { "type": "javascript", "expr": "flow_input.iter.index" },
                                "port": { "type": "javascript", "expr": "flow_input.port" },
                            },
                            "type": "rawscript",
                            "language": "deno",
                            "content": inner_step(),
                        },
                    }],
                },
                "retry": { "constant": { "attempts": 2, "seconds": 0 } },
            }, {
                "value": {
                    "input_transforms": {
                        "last": { "type": "javascript", "expr": "results.a" },
                        "port": { "type": "javascript", "expr": "flow_input.port" },
                    },
                    "type": "rawscript",
                    "language": "python3",
                    "content": last_step(),
                },
                "retry": { "constant": { "attempts": 2, "seconds": 0 } },
            }],
        }))
        .unwrap()
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_pass(db: Pool<Postgres>) {
        initialize_tracing().await;

        // let server = ApiServer::start(db.clone()).await;

        /* fails twice in the loop, then once on the last step
         * retry attempts is measured per-step, so it _retries_ at most two times on each step,
         * which means it may run the step three times in total */

        let (attempts, responses) = [
            /* pass fail */
            (0, Some(99)),
            (1, None),
            /* pass pass fail */
            (0, Some(99)),
            (1, Some(99)),
            (2, None),
            /* pass pass pass */
            (0, Some(3)),
            (1, Some(5)),
            (2, Some(7)),
            /* fail the last step once */
            (0xff, None),
            (0xff, Some(9)),
        ]
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
        let server = Server::start(responses).await;
        let result = RunJob::from(JobPayload::RawFlow {
            value: flow_forloop_retry(),
            path: None,
            restarted_from: None,
        })
        .arg("items", json!(["unused", "unused", "unused"]))
        .arg("port", json!(server.addr.port()))
        .run_until_complete(&db, server.addr.port())
        .await
        .json_result()
        .unwrap();

        assert_eq!(server.close().await, attempts);
        assert_eq!(json!([3, 5, 7, 9]), result);
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_fail_step_zero(db: Pool<Postgres>) {
        initialize_tracing().await;

        /* attempt and fail the first step three times and stop */
        let (attempts, responses) = [
            /* pass fail x3 */
            (0, Some(99)),
            (1, None),
            (0, Some(99)),
            (1, None),
            (0, Some(99)),
            (1, None),
        ]
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
        let server = Server::start(responses).await;
        let result = RunJob::from(JobPayload::RawFlow {
            value: flow_forloop_retry(),
            path: None,
            restarted_from: None,
        })
        .arg("items", json!(["unused", "unused", "unused"]))
        .arg("port", json!(server.addr.port()))
        .run_until_complete(&db, server.addr.port())
        .await
        .json_result()
        .unwrap();

        assert_eq!(server.close().await, attempts);

        assert!(
            result[1]["error"]
                .as_object()
                .unwrap()
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                == "read"
        );
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_fail_step_one(db: Pool<Postgres>) {
        initialize_tracing().await;

        /* attempt and fail the first step three times and stop */
        let (attempts, responses) = [
            /* fail once, then pass */
            (0, None),
            (0, Some(1)),
            (1, Some(2)),
            (2, Some(3)),
            /* fail three times */
            (0xff, None),
            (0xff, None),
            (0xff, None),
        ]
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
        let server = Server::start(responses).await;
        let job = RunJob::from(JobPayload::RawFlow {
            value: flow_forloop_retry(),
            path: None,
            restarted_from: None,
        })
        .arg("items", json!(["unused", "unused", "unused"]))
        .arg("port", json!(server.addr.port()))
        .run_until_complete(&db, server.addr.port())
        .await;

        let result = job.json_result().unwrap();
        assert_eq!(server.close().await, attempts);
        assert!(result["error"]
            .as_object()
            .unwrap()
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("index out of range"));
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base"))]
    async fn test_with_failure_module(db: Pool<Postgres>) {
        initialize_tracing().await;

        // let server = ApiServer::start(db.clone()).await;

        let value = serde_json::from_value(json!({
            "modules": [{
                "id": "a",
                "value": {
                    "input_transforms": { "port": { "type": "javascript", "expr": "flow_input.port" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": r#"
def main(port):
    with __import__("socket").create_connection((None, port)) as sock:
        sock.send(b'\x00')
        return sock.recv(1)[0]"#,
                },
                "retry": { "constant": { "attempts": 1, "seconds": 0 } },
            }],
            "failure_module": {
                "value": {
                    "input_transforms": { "error": { "type": "javascript", "expr": "previous_result", },
                        "port": { "type": "javascript", "expr": "flow_input.port" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": r#"
def main(error, port):
    with __import__("socket").create_connection((None, port)) as sock:
        sock.send(b'\xff')
        return { "recv": sock.recv(1)[0], "from failure module": error }"#,
                },
                "retry": { "constant": { "attempts": 1, "seconds": 0 } },
            },
        }))
        .unwrap();
        let (_attempts, responses) = [
            /* fail the first step twice */
            (0x00, None),
            (0x00, None),
            /* and the failure module once */
            (0xff, None),
            (0xff, Some(42)),
        ]
        .into_iter()
        .unzip::<_, _, Vec<_>, Vec<_>>();
        let server = Server::start(responses).await;
        let cjob = RunJob::from(JobPayload::RawFlow { value, path: None, restarted_from: None })
            .arg("port", json!(server.addr.port()))
            .run_until_complete(&db, server.addr.port())
            .await;
        let result = cjob.json_result().clone().unwrap();
        let failed_module = get_module(&cjob, "a").unwrap();
        match failed_module {
            FlowStatusModule::Failure { .. } => {}
            _ => panic!("expected failure module"),
        }

        println!("result: {:#?}", result);
        assert_eq!(
            result
                .get("from failure module")
                .unwrap()
                .get("error")
                .unwrap()
                .get("name")
                .unwrap()
                .clone(),
            json!("IndexError")
        );

        assert_eq!(result.get("recv").unwrap().clone(), json!(42));
    }
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_iteration(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "javascript", "expr": "result.items" },
                "skip_failures": false,
                "modules": [{
                    "value": {
                        "input_transforms": {
                            "n": {
                                "type": "javascript",
                                "expr": "flow_input.iter.value",
                            },
                        },
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n):\n    if 1 < n:\n        raise StopIteration(n)",
                    },
                }],
            },
        }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!([]))
            .run_until_complete(&db, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!([]));

    /* Don't actually test that this does 257 jobs or that will take forever. */
    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!((0..257).collect::<Vec<_>>()))
            .run_until_complete(&db, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert!(matches!(result, serde_json::Value::Array(_)));
    assert!(result[2]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("2"));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_iteration_parallel(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "javascript", "expr": "result.items" },
                "skip_failures": false,
                "parallel": true,
                "modules": [{
                    "value": {
                        "input_transforms": {
                            "n": {
                                "type": "javascript",
                                "expr": "flow_input.iter.value",
                            },
                        },
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n):\n    if 1 < n:\n        raise StopIteration(n)",
                    },
                }],
            },
        }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!([]))
            .run_until_complete(&db, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!([]));

    /* Don't actually test that this does 257 jobs or that will take forever. */
    let job =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("items", json!((0..50).collect::<Vec<_>>()))
            .run_until_complete(&db, server.addr.port())
            .await;
    // println!("{:#?}", job);
    let result = job.json_result().unwrap();
    assert!(matches!(result, serde_json::Value::Array(_)));
    assert!(result[2]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("2"));
}

struct RunJob {
    payload: JobPayload,
    args: serde_json::Map<String, serde_json::Value>,
}

impl From<JobPayload> for RunJob {
    fn from(payload: JobPayload) -> Self {
        Self { payload, args: Default::default() }
    }
}

impl RunJob {
    fn arg<S: Into<String>>(mut self, k: S, v: serde_json::Value) -> Self {
        self.args.insert(k.into(), v);
        self
    }

    async fn push(self, db: &Pool<Postgres>) -> Uuid {
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
            /* scheduled_for_o */ None,
            /* schedule_path */ None,
            /* parent_job */ None,
            /* root job  */ None,
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
        )
        .await
        .expect("push has to succeed");
        tx.commit().await.unwrap();

        uuid
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    async fn run_until_complete(self, db: &Pool<Postgres>, port: u16) -> CompletedJob {
        let uuid = self.push(db).await;
        let listener = listen_for_completed_jobs(db).await;
        in_test_worker(db, listener.find(&uuid), port).await;
        let r = completed_job(uuid, db).await;
        r
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    async fn run_until_complete_with<F: Future<Output = ()>>(
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

async fn run_job_in_new_worker_until_complete(
    db: &Pool<Postgres>,
    job: JobPayload,
    port: u16,
) -> CompletedJob {
    RunJob::from(job).run_until_complete(db, port).await
}

/// Start a worker with a timeout and run a future, until the worker quits or we time out.
///
/// Cleans up the worker before resolving.
async fn in_test_worker<Fut: std::future::Future>(
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

fn spawn_test_worker(
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

async fn listen_for_completed_jobs(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "completed").await
}

async fn listen_for_queue(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "queued").await
}

async fn listen_for_uuid_on(
    db: &Pool<Postgres>,
    channel: &'static str,
) -> impl Stream<Item = Uuid> + Unpin {
    let mut listener = PgListener::connect_with(db).await.unwrap();
    listener.listen(channel).await.unwrap();

    Box::pin(stream::unfold(listener, |mut listener| async move {
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

async fn completed_job(uuid: Uuid, db: &Pool<Postgres>) -> CompletedJob {
    sqlx::query_as::<_, CompletedJob>(
        "SELECT *, result->'wm_labels' as labels FROM v2_as_completed_job  WHERE id = $1",
    )
    .bind(uuid)
    .fetch_one(db)
    .await
    .unwrap()
}

#[axum::async_trait(?Send)]
trait StreamFind: futures::Stream + Unpin + Sized {
    async fn find(self, item: &Self::Item) -> Option<Self::Item>
    where
        for<'l> &'l Self::Item: std::cmp::PartialEq,
    {
        use futures::{future::ready, StreamExt};

        self.filter(|i| ready(i == item)).next().await
    }
}

impl<T: futures::Stream + Unpin + Sized> StreamFind for T {}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_deno_flow(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;

    let numbers = "export function main() { return [1, 2, 3]; }";
    let doubles = "export function main(n) { return n * 2; }";

    let flow = {
        FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: Default::default(),
                        language: ScriptLang::Deno,
                        content: numbers.to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        is_trigger: None,
                    }
                    .into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Javascript { expr: "result".to_string() },
                        skip_failures: false,
                        parallel: false,
                        parallelism: None,
                        modules: vec![FlowModule {
                            id: "c".to_string(),
                            value: FlowModuleValue::RawScript {
                                input_transforms: [(
                                    "n".to_string(),
                                    InputTransform::Javascript {
                                        expr: "flow_input.iter.value".to_string(),
                                    },
                                )]
                                .into(),
                                language: ScriptLang::Deno,
                                content: doubles.to_string(),
                                path: None,
                                lock: None,
                                tag: None,
                                custom_concurrency_key: None,
                                concurrent_limit: None,
                                concurrency_time_window_s: None,
                                is_trigger: None,
                            }
                            .into(),
                            stop_after_if: Default::default(),
                            stop_after_all_iters_if: Default::default(),
                            summary: Default::default(),
                            suspend: Default::default(),
                            retry: None,
                            sleep: None,
                            cache_ttl: None,
                            mock: None,
                            timeout: None,
                            priority: None,
                            delete_after_use: None,
                            continue_on_error: None,
                            skip_if: None,
                        }],
                        modules_node: None,
                    }
                    .into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
            ],
            same_worker: false,
            ..Default::default()
        }
    };

    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let port = server.addr.port();

    for i in 0..50 {
        println!("deno flow iteration: {}", i);
        let job = run_job_in_new_worker_until_complete(&db, job.clone(), port).await;
        // println!("job: {:#?}", job.flow_status);
        let result = job.json_result().unwrap();
        assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {}", i);
    }
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_identity(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [{
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(): return 42",
                }}, {
                    "value": {
                        "type": "identity",
                    },
                }, {
                    "value": {
                        "type": "identity",
                    },
                }, {
                    "value": {
                        "type": "identity",
                    },
                }],
    }))
    .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .run_until_complete(&db, server.addr.port())
            .await
            .json_result()
            .unwrap();
    assert_eq!(result, serde_json::json!(42));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_deno_flow_same_worker(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;

    let write_file = r#"export async function main(loop: boolean, i: number, path: string) {  
            await Deno.writeTextFile(`./shared/${path}`, `${loop} ${i}`);
        }"#
    .to_string();

    let flow = FlowValue {
            modules: vec![
                FlowModule {
                    id: "a".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: [
                            (
                                "loop".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&false) },
                            ),
                            ("i".to_string(), InputTransform::Static { value: windmill_common::worker::to_raw_value(&1) }),
                            (
                                "path".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&"outer.txt") },
                            ),
                        ]
                        .into(),
                        language: ScriptLang::Deno,
                        content: write_file.clone(),
                        path: None,
                        lock: None,
                        tag: None,
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        is_trigger: None,

                    }.into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
                FlowModule {
                    id: "b".to_string(),
                    value: FlowModuleValue::ForloopFlow {
                        iterator: InputTransform::Static { value: windmill_common::worker::to_raw_value(&[1, 2, 3]) },
                        skip_failures: false,
                        parallel: false,
                        parallelism: None,
                        modules: vec![
                            FlowModule {
                                id: "d".to_string(),
                                value: FlowModuleValue::RawScript {
                                    input_transforms: [
                                        (
                                            "i".to_string(),
                                            InputTransform::Javascript {
                                                expr: "flow_input.iter.value".to_string(),
                                            },
                                        ),
                                        (
                                            "loop".to_string(),
                                            InputTransform::Static { value: windmill_common::worker::to_raw_value(&true) },
                                        ),
                                        (
                                            "path".to_string(),
                                            InputTransform::Static { value: windmill_common::worker::to_raw_value(&"inner.txt") },
                                        ),
                                    ]
                                    .into(),
                                    language: ScriptLang::Deno,
                                    content: write_file,
                                    path: None,
                                    lock: None,
                                    tag: None,
                                    custom_concurrency_key: None,
                                    concurrent_limit: None,
                                    concurrency_time_window_s: None,
                                    is_trigger: None,
                                }.into(),
                                stop_after_if: Default::default(),
                                stop_after_all_iters_if: Default::default(),
                                summary: Default::default(),
                                suspend: Default::default(),
                                retry: None,
                                sleep: None,
                                cache_ttl: None,
                                mock: None,
                                timeout: None,
                                priority: None,
                                delete_after_use: None,
                                continue_on_error: None,
                                skip_if: None,
                            },
                            FlowModule {
                                id: "e".to_string(),
                                value: FlowModuleValue::RawScript {
                                    input_transforms: [(
                                        "path".to_string(),
                                        InputTransform::Static { value: windmill_common::worker::to_raw_value(&"inner.txt") },
                                    ), (
                                        "path2".to_string(),
                                        InputTransform::Static { value: windmill_common::worker::to_raw_value(&"outer.txt") },
                                    )]
                                    .into(),
                                    language: ScriptLang::Deno,
                                    content: r#"export async function main(path: string, path2: string) {  
                                        return await Deno.readTextFile(`./shared/${path}`) + "," + await Deno.readTextFile(`./shared/${path2}`);
                                    }"#
                                    .to_string(),
                                    path: None,
                                    lock: None,
                                    tag: None,
                                    custom_concurrency_key: None,
                                    concurrent_limit: None,
                                    concurrency_time_window_s: None,
                                    is_trigger: None,

                                }.into(),
                                stop_after_if: Default::default(),
                                stop_after_all_iters_if: Default::default(),
                                summary: Default::default(),
                                suspend: Default::default(),
                                retry: None,
                                sleep: None,
                                cache_ttl: None,
                                mock: None,
                                timeout: None,
                                priority: None,
                                delete_after_use: None,
                                continue_on_error: None,
                                skip_if: None,
                            },
                        ],
                        modules_node: None,
                    }.into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
                FlowModule {
                    id: "c".to_string(),
                    value: FlowModuleValue::RawScript {
                        input_transforms: [
                            (
                                "loops".to_string(),
                                InputTransform::Javascript { expr: "results.b".to_string() },
                            ),
                            (
                                "path".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&"outer.txt") },
                            ),
                            (
                                "path2".to_string(),
                                InputTransform::Static { value: windmill_common::worker::to_raw_value(&"inner.txt") },
                            ),
                        ]
                        .into(),
                        language: ScriptLang::Deno,
                        content: r#"export async function main(path: string, loops: string[], path2: string) {
                            return await Deno.readTextFile(`./shared/${path}`) + "," + loops + "," + await Deno.readTextFile(`./shared/${path2}`);
                        }"#
                        .to_string(),
                        path: None,
                        lock: None,
                        tag: None,
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        is_trigger: None,
                    }.into(),
                    stop_after_if: Default::default(),
                    stop_after_all_iters_if: Default::default(),
                    summary: Default::default(),
                    suspend: Default::default(),
                    retry: None,
                    sleep: None,
                    cache_ttl: None,
                    mock: None,
                    timeout: None,
                    priority: None,
                    delete_after_use: None,
                    continue_on_error: None,
                    skip_if: None,
                },
            ],
            same_worker: true,
            ..Default::default()
        };

    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let result = run_job_in_new_worker_until_complete(&db, job.clone(), server.addr.port())
        .await
        .json_result()
        .unwrap();
    assert_eq!(
        result,
        serde_json::json!("false 1,true 1,false 1,true 2,false 1,true 3,false 1,true 3")
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_flow_result_by_id(db: Pool<Postgres>) {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
            "modules": [
                {
                    "id": "a",
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(){ return 42 }",
                    }
                },
                {
                    "value": {
                        "branches": [
                            {
                                "modules": [{
                                    "value": {
                                        "branches": [{"modules": [                {
                                            "id": "d",
                                            "value": {
                                                "input_transforms": {"v": {"type": "javascript", "expr": "results.a"}},
                                                "type": "rawscript",
                                                "language": "deno",
                                                "content": "export function main(v){ return v }",
                                            }

                                        },]}],
                                        "type": "branchall",
                                    }
                                }],
                            }],
                            "type": "branchall",
                        },
                    }
            ],
        }))
        .unwrap();

    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, job.clone(), port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(result, serde_json::json!([[42]]));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_if(db: Pool<Postgres>) {
    initialize_tracing().await;
    // let server = ApiServer::start(db.clone()).await;
    // let port = server.addr.port();

    let port = 123;
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return n",
                },
                "stop_after_if": {
                    "expr": "result < 0",
                    "skip_if_stopped": false,
                },
            },
            {
                "id": "b",
                "value": {
                    "input_transforms": { "n": { "type": "javascript", "expr": "results.a" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return f'last step saw {n}'",
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let result = RunJob::from(job.clone())
        .arg("n", json!(123))
        .run_until_complete(&db, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(json!("last step saw 123"), result);

    let cjob = RunJob::from(job.clone())
        .arg("n", json!(-123))
        .run_until_complete(&db, port)
        .await;

    let result = cjob.json_result().unwrap();
    assert_eq!(json!(-123), result);
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_stop_after_if_nested(db: Pool<Postgres>) {
    initialize_tracing().await;
    // let server = ApiServer::start(db.clone()).await;
    // let port = server.addr.port();

    let port = 123;
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "branches": [{"modules": [{
                    "id": "b",
                    "value": {
                        "input_transforms": { "n": { "type": "javascript", "expr": "flow_input.n" } },
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n): return n",
                    },
                    "stop_after_if": {
                        "expr": "result < 0",
                        "skip_if_stopped": false,
                    }}]}],
                    "type": "branchall"
                },
            },
            {
                "id": "c",
                "value": {
                    "input_transforms": { "n": { "type": "javascript", "expr": "results.a" } },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(n): return f'last step saw {n}'",
                },
            },
        ],
    }))
    .unwrap();
    let job = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };

    let result = RunJob::from(job.clone())
        .arg("n", json!(123))
        .run_until_complete(&db, port)
        .await
        .json_result()
        .unwrap();
    assert_eq!(json!("last step saw [123]"), result);

    let cjob = RunJob::from(job.clone())
        .arg("n", json!(-123))
        .run_until_complete(&db, port)
        .await;

    let result = cjob.json_result().unwrap();
    assert_eq!(json!([-123]), result);
}

#[cfg(all(feature = "deno_core", feature = "python"))]
#[sqlx::test(fixtures("base"))]
async fn test_python_flow(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let numbers = "def main(): return [1, 2, 3]";
    let doubles = "def main(n): return n * 2";

    let flow: FlowValue = serde_json::from_value(serde_json::json!( {
        "modules": [
            {
                "value": {
                    "type": "rawscript",
                    "language": "python3",
                    "content": numbers,
                },
            },
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "result" },
                    "skip_failures": false,
                    "modules": [{
                        "value": {
                            "input_transforms": {
                                "n": {
                                    "type": "javascript",
                                    "expr": "flow_input.iter.value",
                                },
                            },
                            "type": "rawscript",
                            "language": "python3",
                            "content": doubles,
                        },
                    }],
                },
            },
        ],
    }))
    .unwrap();

    for i in 0..10 {
        println!("python flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(
            &db,
            JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None },
            port,
        )
        .await
        .json_result()
        .unwrap();

        assert_eq!(result, serde_json::json!([2, 4, 6]), "iteration: {i}");
    }
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_flow_2(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [
                {
                    "value": {
                        "input_transforms": {},
                        "type": "rawscript",
                        "content": "import wmill\ndef main():  return \"Hello\"",
                        "language": "python3"
                    },
                }
            ]
    }))
    .unwrap();

    for i in 0..10 {
        println!("python flow iteration: {}", i);
        let result = run_job_in_new_worker_until_complete(
            &db,
            JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None },
            port,
        )
        .await
        .json_result()
        .unwrap();

        assert_eq!(result, serde_json::json!("Hello"), "iteration: {i}");
    }
}

#[sqlx::test(fixtures("base"))]
async fn test_go_job(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
package inner

import "fmt"

func main(derp string) (string, error) {
	fmt.Println("Hello, 世界")
	return fmt.Sprintf("hello %s", derp), nil
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Go,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("derp", json!("world"))
    .run_until_complete(&db, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
}

#[cfg(feature = "rust")]
#[sqlx::test(fixtures("base"))]
async fn test_rust_job(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
fn main(world: String) -> Result<String, String> {
    println!("Which world to greet today?");
    Ok(format!("Hello {}!", world))
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Rust,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("world", json!("Hyrule"))
    .run_until_complete(&db, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("Hello Hyrule!"));
}

// #[sqlx::test(fixtures("base"))]
// async fn test_csharp_job(db: Pool<Postgres>) {
//     initialize_tracing().await;
//     let server = ApiServer::start(db.clone()).await;
//     let port = server.addr.port();
//
//     let content = r#"
// using System;
//
// class Script
// {
//     public static string Main(string world, int b = 2)
//     {
//         Console.WriteLine($"Hello {world} - {b}. This is a log line");
//         return $"Hello {world} - {b}";
//     }
// }
//         "#
//     .to_owned();
//
//     let result = RunJob::from(JobPayload::Code(RawCode {
//         hash: None,
//         content,
//         path: None,
//         lock: None,
//         language: ScriptLang::CSharp,
//         custom_concurrency_key: None,
//         concurrent_limit: None,
//         concurrency_time_window_s: None,
//         cache_ttl: None,
//         dedicated_worker: None,
//     }))
//     .arg("world", json!("Arakis"))
//     .arg("b", json!(3))
//     .run_until_complete(&db, port)
//     .await
//     .json_result()
//     .unwrap();
//
//     assert_eq!(result, serde_json::json!("Hello Arakis - 3"));
// }

#[sqlx::test(fixtures("base"))]
async fn test_bash_job(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
msg="$1"
echo "hello $msg"
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bash,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
}

#[cfg(feature = "nu")]
#[sqlx::test(fixtures("base"))]
async fn test_nu_job(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
def main [ msg: string ] {
    "hello " + $msg
}
"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nu,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("msg", json!("world"))
    .run_until_complete(&db, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
}

#[cfg(feature = "nu")]
#[sqlx::test(fixtures("base"))]
async fn test_nu_job_full(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
def main [ 
  # Required
  ## Primitive
  a
  b: any
  c: bool
  d: float
  e: datetime
  f: string
  j: nothing
  ## Nesting
  g: record
  h: list<string>
  i: table
  # Optional
  m?
  n = "foo"
  o: any = "foo"
  p?: any
  # TODO: ...x
 ] {
    0
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Nu,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("a", json!("3"))
    .arg("b", json!("null"))
    .arg("c", json!(true))
    .arg("d", json!(3.0))
    .arg("e", json!("2024-09-24T10:00:00.000Z"))
    .arg("f", json!("str"))
    .arg("j", json!(null))
    .arg("g", json!({"a": 32}))
    .arg("h", json!(["foo"]))
    .arg(
        "i",
        json!([
            {"a": 1, "b": "foo", "c": true},
            {"a": 2, "b": "baz", "c": false}
        ]),
    )
    .arg("n", json!("baz"))
    .run_until_complete(&db, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!(0));
}

#[cfg(feature = "java")]
#[sqlx::test(fixtures("base"))]
async fn test_java_job(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
public class Main {
  public static Object main(
    // Primitive
    int a,
    float b,
    // Objects
    Integer age,
    Float d
    ){
    return "hello world";
  }
}

"#
    .to_owned();

    let job = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Java,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("a", json!(3))
    .arg("b", json!(3.0))
    .arg("age", json!(30))
    .arg("d", json!(3.0))
    .run_until_complete(&db, port)
    .await;
    assert_eq!(job.json_result(), Some(json!("hello world")));
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
def main():
    return "hello world"
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_heavy_dep(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
import numpy as np

def main():
    a = np.arange(15).reshape(3, 5)
    return len(a)
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(3));
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_with_imports(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
import wmill

def main():
    return wmill.get_workspace()
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("test-workspace"));
}

#[sqlx::test(fixtures("base"))]
async fn test_bun_job_datetime(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
export async function main(a: Date) {
    return typeof a;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Bun,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .run_until_complete(&db, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("object"));
}

#[sqlx::test(fixtures("base"))]
async fn test_deno_job_datetime(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
export async function main(a: Date) {
    return typeof a;
}
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Deno,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .run_until_complete(&db, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!("object"));
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_datetime_and_bytes(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let content = r#"
from datetime import datetime
def main(a: datetime, b: bytes):
    return (isinstance(a, datetime), isinstance(b, bytes))
        "#
    .to_owned();

    let result = RunJob::from(JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        lock: None,
        language: ScriptLang::Python3,
        custom_concurrency_key: None,
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
    }))
    .arg("a", json!("2024-09-24T10:00:00.000Z"))
    .arg("b", json!("dGVzdA=="))
    .run_until_complete(&db, port)
    .await
    .json_result()
    .unwrap();

    assert_eq!(result, serde_json::json!([true, true]));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_empty_loop_1(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [] },
                    "modules": [
                        {
                            "value": {
                                "input_transforms": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "flow_input.iter.value",
                                    },
                                },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            },
                        }
                    ],
                },
            },
            {
                "value": {
                    "input_transforms": {
                        "items": {
                            "type": "javascript",
                            "expr": "results.a",
                        },
                    },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(items): return sum(items)",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(0));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_invalid_first_step(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "flow_input" },
                    "modules": [
                        {
                            "value": {
                                "type": "identity",
                            },
                        }
                    ],
                },
            },
            {
                "value": {
                    "type": "identity",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let job = run_job_in_new_worker_until_complete(&db, flow, port).await;

    assert!(
        serde_json::to_string(&job.json_result().unwrap()).unwrap().contains("Expected an array value in the iterator expression, found: invalid type: map, expected a sequence at line 1 column 0")
    );
}

#[sqlx::test(fixtures("base"))]
async fn test_empty_loop_2(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [] },
                    "modules": [
                        {
                            "value": {
                                "input_transforms": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "flow_input.iter.value",
                                    },
                                },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            },
                        }
                    ],
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([]));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_step_after_loop(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();
    let flow: FlowValue = serde_json::from_value(serde_json::json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "static", "value": [2,3,4] },
                    "modules": [
                        {
                            "value": {
                                "input_transforms": {
                                    "n": {
                                        "type": "javascript",
                                        "expr": "flow_input.iter.value",
                                    },
                                },
                                "type": "rawscript",
                                "language": "python3",
                                "content": "def main(n): return n",
                            } ,
                        }
                    ],
                },
            },
            {
                "value": {
                    "input_transforms": {
                        "items": {
                            "type": "javascript",
                            "expr": "results.a",
                        },
                    },
                    "type": "rawscript",
                    "language": "python3",
                    "content": "def main(items): return sum(items)",
                },
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(9));
}

fn module_add_item_to_list(i: i32, id: &str) -> serde_json::Value {
    json!({
        "id": format!("id_{}", i.to_string().replace("-", "_")),
        "value": {
            "input_transforms": {
                "array": {
                    "type": "javascript",
                    "expr": format!("results.{id}"),
                },
                "i": {
                    "type": "static",
                    "value": json!(i),
                }
            },
            "type": "rawscript",
            "language": "deno",
            "content": "export function main(array, i){ array.push(i); return array }",
        }
    })
}

fn module_failure() -> serde_json::Value {
    json!({
        "value": {
            "input_transforms": {},
            "type": "rawscript",
            "language": "deno",
            "content": "export function main(){ throw Error('failure') }",
        }
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_branchone_simple(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [],
                    "default": [module_add_item_to_list(2, "a")],
                    "type": "branchone",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([1, 2]));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchone_with_cond(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [{"expr": "results.a[0] == 1", "modules": [module_add_item_to_list(3, "a")]}],
                    "default": [module_add_item_to_list(2, "a")],
                    "type": "branchone",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([1, 3]));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchall_sequential(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_add_item_to_list(2, "a")]},
                        {"modules": [module_add_item_to_list(3, "a")]}],
                    "type": "branchall",
                    "parallel": true,
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([[1, 2], [1, 3]]));
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchall_simple(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_add_item_to_list(2, "a")]},
                        {"modules": [module_add_item_to_list(3, "a")]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([[1, 2], [1, 3]]));
}

#[derive(Deserialize)]
struct ErrorResult {
    error: NamedError,
}

#[derive(Deserialize)]
struct NamedError {
    name: String,
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_skip_failure(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_failure()], "skip_failure": false},
                        {"modules": [module_add_item_to_list(3, "a")]}],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(
        serde_json::from_value::<ErrorResult>(result.get(0).unwrap().clone())
            .unwrap()
            .error
            .name,
        "Error"
    );

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {"modules": [module_failure()], "skip_failure": true},
                        {"modules": [module_add_item_to_list(2, "a")]}
                ],
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(
        serde_json::from_value::<ErrorResult>(result.get(0).unwrap().clone())
            .unwrap()
            .error
            .name,
        "Error"
    );
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_branchone_nested(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [] }",
                }
            },
            module_add_item_to_list(1, "a"),
            {
                "id": "b",
                "value": {
                    "branches": [
                        {
                            "expr": "false",
                            "modules": []
                        },
                        {
                            "expr": "true",
                            "modules": [                {
                                "value": {
                                    "branches": [
                                        {
                                            "expr": "false",
                                            "modules": []
                                        }],
                                    "default": [module_add_item_to_list(2, "id_1")],
                                    "type": "branchone",
                                }
                            }]
                        },
                    ],
                    "default": [module_add_item_to_list(-4, "id_1")],
                    "type": "branchone",
                }
            },
            module_add_item_to_list(3, "b"),
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!([1, 2, 3]));
}

#[sqlx::test(fixtures("base"))]
async fn test_branchall_nested(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(){ return [1] }",
                }
            },
            {
                "value": {
                    "branches": [
                        {
                            "modules": [
                                    {
                                "id": "b",
                                "value": {
                                    "branches": [
                                        {"modules": [module_add_item_to_list(2, "a")]},
                                        {"modules": [module_add_item_to_list(3, "a")]}],
                                    "type": "branchall",
                                }
                            }, {
                                "value": {
                                    "branches": [
                                        {"modules": [module_add_item_to_list(4, "b")]},
                                        {"modules": [module_add_item_to_list(5, "b")]}],
                                    "type": "branchall",
                                }
                            }
                                    ]
                        },
                        {"modules": [module_add_item_to_list(6, "a")]}],
                        // "parallel": false,
                    "type": "branchall",
                }
            },
        ],
    }))
    .unwrap();

    let flow = JobPayload::RawFlow { value: flow, path: None, restarted_from: None };
    let result = run_job_in_new_worker_until_complete(&db, flow, port)
        .await
        .json_result()
        .unwrap();

    println!("{:#?}", result);
    assert_eq!(
        result,
        serde_json::json!([[[[1, 2], [1, 3], 4], [[1, 2], [1, 3], 5]], [1, 6]])
    );
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]
async fn test_failure_module(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(serde_json::json!({
            "modules": [{
                "id": "a",
                "value": {
                    "input_transforms": {
                        "l": { "type": "javascript", "expr": "[]", },
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                    },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 0) throw Error(JSON.stringify(l)); return { l: [...l, 0] } }",
                },
            }, {
                "id": "b",
                "value": {
                    "input_transforms": {
                        "l": { "type": "javascript", "expr": "results.a.l", },
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                    },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 1) throw Error(JSON.stringify(l)); return { l: [...l, 1] } }",
                },
            }, {
                "value": {
                    "input_transforms": {
                        "l": { "type": "javascript", "expr": "results.b.l", },
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                    },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(n, l) { if (n == 2) throw Error(JSON.stringify(l)); return { l: [...l, 2] } }",
                },
            }],
            "failure_module": {
                "value": {
                    "input_transforms": { "error": { "type": "javascript", "expr": "previous_result", } },
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main(error) { return { 'from failure module': error } }",
                }
            },
        }))
        .unwrap();

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(0))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

    assert!(result["from failure module"]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("[]"));

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(1))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

    assert!(result["from failure module"]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("[0]"));

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(2))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

    assert!(result["from failure module"]["error"]
        .as_object()
        .unwrap()
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("[0,1]"));

    let result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .arg("n", json!(3))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();
    assert_eq!(json!({ "l": [0, 1, 2] }), result);
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_lock_all(db: Pool<Postgres>) {
    use futures::StreamExt;
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: windmill_api_client::types::OpenFlow = serde_json::from_value(serde_json::json!({
        "summary": "",
        "description": "",
        "value": {
            "modules": [
                {
                    "id": "a",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "import wmill\n\ndef main():\n  return \"Test\"\n",
                        "language": "python3",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                },
                {
                    "id": "b",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "import * as wmill from \"https://deno.land/x/windmill@v1.50.0/mod.ts\"\n\nexport async function main() {\n  return wmill\n}\n",
                        "language": "deno",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                },
                {
                    "id": "c",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "package inner\n\nimport (\n\t\"fmt\"\n\t\"rsc.io/quote\"\n  wmill \"github.com/windmill-labs/windmill-go-client\"\n)\n\n// the main must return (interface{}, error)\n\nfunc main() (interface{}, error) {\n\tfmt.Println(\"Hello, World\")\n  // v, _ := wmill.GetVariable(\"g/all/pretty_secret\")\n  return \"Test\"\n}\n",
                        "language": "go",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                },
                {
                    "id": "d",
                    "value": {
                        "lock": null,
                        "path": null,
                        "type": "rawscript",
                        "content": "\n# the last line of the stdout is the return value\necho \"Hello $msg\"\n",
                        "language": "bash",
                        "input_transforms": {}
                    },
                    "summary": null,
                    "stop_after_if": null,
                    "input_transforms": {}
                }
            ],
            "failure_module": null
        },
        "schema": {
            "type": "object",
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "required": [],
            "properties": {}
        }
    }))
    .unwrap();

    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_owned(),
    );
    client
        .create_flow(
            "test-workspace",
            &CreateFlowBody {
                open_flow_w_path: windmill_api_client::types::OpenFlowWPath {
                    open_flow: flow,
                    path: "g/all/flow_lock_all".to_owned(),
                    tag: None,
                    ws_error_handler_muted: None,
                    priority: None,
                    dedicated_worker: None,
                    timeout: None,
                    visible_to_runner_only: None,
                    on_behalf_of_email: None,
                },
                draft_only: None,
                deployment_message: None,
            },
        )
        .await
        .unwrap();
    let mut str = listen_for_completed_jobs(&db).await;
    let listen_first_job = str.next();
    in_test_worker(&db, listen_first_job, port).await;

    let modules = client
        .get_flow_by_path("test-workspace", "g/all/flow_lock_all", None)
        .await
        .unwrap()
        .into_inner()
        .open_flow
        .value
        .modules;
    modules.into_iter()
        .for_each(|m| {
            assert!(matches!(
                m.value,
                windmill_api_client::types::FlowModuleValue::RawScript(RawScript {
                    language: windmill_api_client::types::RawScriptLanguage::Bash,
                    lock: Some(ref lock),
                    ..
                }) if lock == "")
                || matches!(
                m.value,
                windmill_api_client::types::FlowModuleValue::RawScript(RawScript{
                    language: windmill_api_client::types::RawScriptLanguage::Go | windmill_api_client::types::RawScriptLanguage::Python3 | windmill_api_client::types::RawScriptLanguage::Deno,
                    lock: Some(ref lock),
                    ..
                }) if lock.len() > 0),
            "{:?}", m.value
            );
        });
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base"))]

async fn test_complex_flow_restart(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "go",
                    "content": "package inner\nimport (\n\t\"fmt\"\n\t\"math/rand\"\n)\nfunc main(max int) (interface{}, error) {\n\tresult := rand.Intn(max) + 1\n\tfmt.Printf(\"Number generated: '%d'\", result)\n\treturn result, nil\n}",
                    "input_transforms": {
                        "max": {
                            "type": "static",
                            "value": json!(20),
                        },
                    }
                },
                "summary": "Generate random number in [1, 20]"
            },
            {
                "id": "b",
                "value":
                {
                    "type": "branchall",
                    "branches":
                    [
                        {
                            "modules":
                            [
                                {
                                    "id": "d",
                                    "value":
                                    {
                                        "type": "branchone",
                                        "default":
                                        [
                                            {
                                                "id": "f",
                                                "value":
                                                {
                                                    "type": "rawscript",
                                                    "content": "package inner\nimport \"math/rand\"\nfunc main(x int) (interface{}, error) {\n\treturn rand.Intn(x) + 1, nil\n}",
                                                    "language": "go",
                                                    "input_transforms":
                                                    {
                                                        "x":
                                                        {
                                                            "expr": "results.a",
                                                            "type": "javascript"
                                                        }
                                                    }
                                                },
                                                "summary": "Rand N in [1; x]"
                                            }
                                        ],
                                        "branches":
                                        [
                                            {
                                                "expr": "results.a < flow_input.max / 2",
                                                "modules":
                                                [
                                                    {
                                                        "id": "e",
                                                        "value":
                                                        {
                                                            "type": "rawscript",
                                                            "content": "package inner\nimport \"math/rand\"\nfunc main(x int) (interface{}, error) {\n\treturn rand.Intn(x * 2) + 1, nil\n}\n",
                                                            "language": "go",
                                                            "input_transforms":
                                                            {
                                                                "x":
                                                                {
                                                                    "expr": "results.a",
                                                                    "type": "javascript"
                                                                }
                                                            }
                                                        },
                                                        "summary": "Rand N in [1; x*2]"
                                                    }
                                                ],
                                                "summary": "N in first half"
                                            }
                                        ]
                                    },
                                    "summary": ""
                                }
                            ],
                            "summary": "Process x",
                            "parallel": true,
                            "skip_failure": false
                        },
                        {
                            "modules":
                            [
                                {
                                    "id": "c",
                                    "value":
                                    {
                                        "type": "rawscript",
                                        "content": "package inner\nfunc main(x int) (interface{}, error) {\n\treturn x, nil\n}",
                                        "language": "go",
                                        "input_transforms":
                                        {
                                            "x":
                                            {
                                                "expr": "results.a",
                                                "type": "javascript"
                                            }
                                        }
                                    },
                                    "summary": "Identity"
                                }
                            ],
                            "summary": "Do nothing",
                            "parallel": true,
                            "skip_failure": false
                        }
                    ],
                    "parallel": false
                },
                "summary": ""
            },
            {
                "id": "g",
                "value":
                {
                    "tag": "",
                    "type": "rawscript",
                    "content": "package inner\nimport \"fmt\"\nfunc main(x []int) (interface{}, error) {\n\tfmt.Printf(\"Results: %v\", x)\n\treturn x, nil\n}\n",
                    "language": "go",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.b",
                            "type": "javascript"
                        }
                    }
                },
                "summary": "Print results - This will get the results from the prior step directly"
            },
            {
                "id": "h",
                "value":
                {
                    "tag": "",
                    "type": "rawscript",
                    "content": "package inner\nimport (\n\t\"fmt\"\n\t\"slices\"\n)\nfunc main(x []int) (interface{}, error) {\n\tresult := slices.Max(x)\n\tfmt.Printf(\"Result is %d\", result)\n\treturn result, nil\n}",
                    "language": "go",
                    "input_transforms":
                    {
                        "x":
                        {
                            "expr": "results.b",
                            "type": "javascript"
                        }
                    }
                },
                "summary": "Choose max - this will get results.b querying get_result_by_id on the backend"
            }
        ],
    }))
    .unwrap();

    let first_run_result =
        RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None, restarted_from: None })
            .run_until_complete(&db, port)
            .await;

    let restarted_flow_result = RunJob::from(JobPayload::RawFlow {
        value: flow.clone(),
        path: None,
        restarted_from: Some(RestartedFrom {
            flow_job_id: first_run_result.id,
            step_id: "h".to_owned(),
            branch_or_iteration_n: None,
        }),
    })
    .run_until_complete(&db, port)
    .await;

    let first_run_result_int =
        serde_json::from_value::<i32>(first_run_result.json_result().unwrap())
            .expect("first_run_result was not an int");
    let restarted_flow_result_int =
        serde_json::from_value::<i32>(restarted_flow_result.json_result().unwrap())
            .expect("restarted_flow_result was not an int");
    assert_eq!(first_run_result_int, restarted_flow_result_int);
}

#[sqlx::test(fixtures("base"))]
async fn test_rust_client(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    )
    .list_workspaces()
    .await
    .unwrap();
}

#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("base", "schedule"))]
async fn test_script_schedule_handlers(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(true));

    let now = chrono::Utc::now();
    // add 5 seconds to now
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();

    let schedule = NewSchedule {
        args: ScriptArgs::from(args),
        enabled: Some(true),
        is_flow: false,
        on_failure: Some("script/f/system/schedule_error_handler".to_string()),
        on_failure_times: None,
        on_failure_exact: None,
        on_failure_extra_args: None,
        on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
        on_recovery_times: None,
        on_recovery_extra_args: None,
        on_success: None,
        on_success_extra_args: None,
        path: "f/system/failing_script_schedule".to_string(),
        script_path: "f/system/failing_script".to_string(),
        timezone: "UTC".to_string(),
        schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
        ws_error_handler_muted: None,
        retry: None,
        no_flow_overlap: None,
        summary: None,
        tag: None,
        paused_until: None,
        cron_version: None,
    };

    let _ = client.create_schedule("test-workspace", &schedule).await;

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed error job

            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // error handler

            if uuid.is_err() {
                panic!("schedule error handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job = sqlx::query!(
                "SELECT script_path FROM v2_as_completed_job  WHERE id = $1",
                uuid
            )
            .fetch_one(&db2)
            .await
            .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path != Some("f/system/schedule_error_handler".to_string())
            {
                panic!(
                    "a script was run after main job execution but was not schedule error handler"
                );
            }
        },
        port,
    )
    .await;

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(false));
    let now = chrono::Utc::now();
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();
    client
        .update_schedule(
            "test-workspace",
            "f/system/failing_script_schedule",
            &EditSchedule {
                args: ScriptArgs::from(args),
                on_failure: Some("script/f/system/schedule_error_handler".to_string()),
                on_failure_times: None,
                on_failure_exact: None,
                on_failure_extra_args: None,
                on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
                on_recovery_times: None,
                on_recovery_extra_args: None,
                on_success: None,
                on_success_extra_args: None,
                timezone: "UTC".to_string(),
                schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
                ws_error_handler_muted: None,
                retry: None,
                summary: None,
                no_flow_overlap: None,
                tag: None,
                paused_until: None,
                cron_version: None,
            },
        )
        .await
        .unwrap();

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed working job
            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // recovery handler

            if uuid.is_err() {
                panic!("schedule recovery handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job =
                sqlx::query!("SELECT script_path FROM v2_as_completed_job  WHERE id = $1", uuid)
                    .fetch_one(&db2)
                    .await
                    .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path
                    != Some("f/system/schedule_recovery_handler".to_string())
            {
                panic!("a script was run after main job execution but was not schedule recovery handler");
            }
        },
        port,
    )
    .await;
}

#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("base", "schedule"))]
async fn test_flow_schedule_handlers(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(true));

    let now = chrono::Utc::now();
    // add 5 seconds to now
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();

    let schedule = NewSchedule {
        args: ScriptArgs::from(args),
        enabled: Some(true),
        is_flow: true,
        on_failure: Some("script/f/system/schedule_error_handler".to_string()),
        on_failure_times: None,
        on_failure_exact: None,
        on_failure_extra_args: None,
        on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
        on_recovery_times: None,
        on_recovery_extra_args: None,
        on_success: None,
        on_success_extra_args: None,
        path: "f/system/failing_flow_schedule".to_string(),
        script_path: "f/system/failing_flow".to_string(),
        timezone: "UTC".to_string(),
        schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
        ws_error_handler_muted: None,
        retry: None,
        no_flow_overlap: None,
        summary: None,
        tag: None,
        paused_until: None,
        cron_version: None,
    };

    let _ = client.create_schedule("test-workspace", &schedule).await;

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed error step
            str.next().await; // completed error flow

            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // error handler

            if uuid.is_err() {
                panic!("schedule error handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job = sqlx::query!(
                "SELECT script_path FROM v2_as_completed_job  WHERE id = $1",
                uuid
            )
            .fetch_one(&db2)
            .await
            .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path != Some("f/system/schedule_error_handler".to_string())
            {
                panic!(
                    "a script was run after main job execution but was not schedule error handler"
                );
            }
        },
        port,
    )
    .await;

    let mut args = std::collections::HashMap::new();
    args.insert("fail".to_string(), json!(false));
    let now = chrono::Utc::now();
    let then = now
        .checked_add_signed(chrono::Duration::try_seconds(5).unwrap())
        .unwrap();
    client
        .update_schedule(
            "test-workspace",
            "f/system/failing_flow_schedule",
            &EditSchedule {
                args: ScriptArgs::from(args),
                on_failure: Some("script/f/system/schedule_error_handler".to_string()),
                on_failure_times: None,
                on_failure_exact: None,
                on_failure_extra_args: None,
                on_recovery: Some("script/f/system/schedule_recovery_handler".to_string()),
                on_recovery_times: None,
                on_recovery_extra_args: None,
                on_success: None,
                on_success_extra_args: None,
                timezone: "UTC".to_string(),
                schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
                ws_error_handler_muted: None,
                retry: None,
                summary: None,
                no_flow_overlap: None,
                tag: None,
                paused_until: None,
                cron_version: None,
            },
        )
        .await
        .unwrap();

    let mut str = listen_for_completed_jobs(&db).await;

    let db2 = db.clone();
    in_test_worker(
        &db,
        async move {
            str.next().await; // completed working step
            str.next().await; // completed working flow
            let uuid = timeout(Duration::from_millis(5000), str.next()).await; // recovery handler

            if uuid.is_err() {
                panic!("schedule recovery handler was not run within 5 s");
            }

            let uuid = uuid.unwrap().unwrap();

            let completed_job =
                sqlx::query!("SELECT script_path FROM v2_as_completed_job  WHERE id = $1", uuid)
                    .fetch_one(&db2)
                    .await
                    .unwrap();

            if completed_job.script_path.is_none()
                || completed_job.script_path
                    != Some("f/system/schedule_recovery_handler".to_string())
            {
                panic!("a script was run after main job execution but was not schedule recovery handler");
            }
        },
        port,
    )
    .await;
}

async fn run_deployed_relative_imports(
    db: &Pool<Postgres>,
    script_content: String,
    language: ScriptLang,
) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    client
        .create_script(
            "test-workspace",
            &NewScript {
                language: NewScriptLanguage::from_str(language.as_str()).unwrap(),
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
}

async fn run_preview_relative_imports(
    db: &Pool<Postgres>,
    script_content: String,
    language: ScriptLang,
) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
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
}

#[sqlx::test(fixtures("base", "relative_bun"))]
async fn test_relative_imports_bun(db: Pool<Postgres>) {
    let content = r#"
import { main as test1 } from "/f/system/same_folder_script.ts";
import { main as test2 } from "./same_folder_script.ts";
import { main as test3 } from "/f/system_relative/different_folder_script.ts";
import { main as test4 } from "../system_relative/different_folder_script.ts";

export async function main() {
  return [test1(), test2(), test3(), test4()];
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await;
    run_preview_relative_imports(&db, content, ScriptLang::Bun).await;
}

#[cfg(feature = "deno_core")]
#[sqlx::test(fixtures("base", "relative_bun"))]
async fn test_nested_imports_bun(db: Pool<Postgres>) {
    let content = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export async function main() {
  return test();
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await;
    run_preview_relative_imports(&db, content, ScriptLang::Bun).await;
}

#[sqlx::test(fixtures("base", "relative_deno"))]
async fn test_relative_imports_deno(db: Pool<Postgres>) {
    let content = r#"
import { main as test1 } from "/f/system/same_folder_script.ts";
import { main as test2 } from "./same_folder_script.ts";
import { main as test3 } from "/f/system_relative/different_folder_script.ts";
import { main as test4 } from "../system_relative/different_folder_script.ts";

export async function main() {
  return [test1(), test2(), test3(), test4()];
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Deno).await;
    run_preview_relative_imports(&db, content, ScriptLang::Deno).await;
}

#[sqlx::test(fixtures("base", "relative_deno"))]
async fn test_nested_imports_deno(db: Pool<Postgres>) {
    let content = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export async function main() {
  return test();
}
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Deno).await;
    run_preview_relative_imports(&db, content, ScriptLang::Deno).await;
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "relative_python"))]
async fn test_relative_imports_python(db: Pool<Postgres>) {
    let content = r#"
from f.system.same_folder_script import main as test1
from .same_folder_script import main as test2
from f.system_relative.different_folder_script import main as test3
from ..system_relative.different_folder_script import main as test4
    
def main():
    return [test1(), test2(), test3(), test4()]
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Python3).await;
    run_preview_relative_imports(&db, content, ScriptLang::Python3).await;
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "relative_python"))]
async fn test_nested_imports_python(db: Pool<Postgres>) {
    let content = r#"

from f.system_relative.nested_script import main as test

def main():
    return test()
"#
    .to_string();

    run_deployed_relative_imports(&db, content.clone(), ScriptLang::Python3).await;
    run_preview_relative_imports(&db, content, ScriptLang::Python3).await;
}

#[cfg(feature = "python")]
async fn assert_lockfile(
    db: &Pool<Postgres>,
    script_content: String,
    language: ScriptLang,
    expected_lines: Vec<&str>,
) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();
    let client = windmill_api_client::create_client(
        &format!("http://localhost:{port}"),
        "SECRET_TOKEN".to_string(),
    );

    client
        .create_script(
            "test-workspace",
            &NewScript {
                language: NewScriptLanguage::from_str(language.as_str()).unwrap(),
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
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_requirements_python(db: Pool<Postgres>) {
    let content = r#"# py: ==3.11.11
# requirements:
# tiny==0.1.3

import bar
import baz # pin: foo
import baz # repin: fee
import bug # repin: free
    
def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec!["# py: 3.11.11", "tiny==0.1.3"],
    )
    .await;
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_extra_requirements_python(db: Pool<Postgres>) {
    {
        let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny

import f.system.extra_requirements
import tiny # pin: tiny==0.1.0
import tiny # pin: tiny==0.1.1
import tiny # repin: tiny==0.1.2

def main():
    pass
    "#
        .to_string();

        assert_lockfile(
            &db,
            content,
            ScriptLang::Python3,
            vec!["# py: 3.11.11", "bottle==0.13.2", "tiny==0.1.2"],
        )
        .await;
    }
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_extra_requirements_python2(db: Pool<Postgres>) {
    let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny==0.1.3

import simplejson # pin: simplejson==3.20.1
def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec!["# py: 3.11.11", "simplejson==3.20.1", "tiny==0.1.3"],
    )
    .await;
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_pins_python(db: Pool<Postgres>) {
    let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny==0.1.3
# bottle==0.13.2

import f.system.requirements
import f.system.pins
import tiny # repin: tiny==0.1.3
import simplejson

def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# py: 3.11.11",
            "bottle==0.13.2",
            "microdot==2.2.0",
            "simplejson==3.19.3",
            "tiny==0.1.3",
        ],
    )
    .await;
}
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "multipython"))]
async fn test_multipython_python(db: Pool<Postgres>) {
    let content = r#"# py: <=3.12.2, >=3.12.0
import f.multipython.script1
import f.multipython.aliases
"#
    .to_string();

    assert_lockfile(&db, content, ScriptLang::Python3, vec!["# py: 3.12.1\n"]).await;
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "multipython"))]
async fn test_inline_script_metadata_python(db: Pool<Postgres>) {
    let content = r#"# py_select_latest
# /// script
# requires-python = ">3.11,<3.12.3,!=3.12.2"
# dependencies = [
#   "tiny==0.1.3",
# ]
# ///
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec!["# py: 3.12.1", "tiny==0.1.3"],
    )
    .await;
}
#[sqlx::test(fixtures("base", "result_format"))]
async fn test_result_format(db: Pool<Postgres>) {
    let ordered_result_job_id = "1eecb96a-c8b0-4a3d-b1b6-087878c55e41";

    set_jwt_secret().await;

    let server = ApiServer::start(db.clone()).await;

    let port = server.addr.port();

    let token = windmill_common::auth::create_token_for_owner(
        &db,
        "test-workspace",
        "u/test-user",
        "",
        100,
        "",
        &Uuid::nil(),
        None,
    )
    .await
    .unwrap();

    #[derive(Debug, Deserialize)]
    struct JobResponse {
        result: Option<Box<serde_json::value::RawValue>>,
    }

    async fn get_result<T: DeserializeOwned>(url: String) -> T {
        reqwest::get(url)
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    let correct_result = r#"[{"b":"first","a":"second"}]"#;

    let job_response: JobResponse = get_result(format!("http://localhost:{port}/api/w/test-workspace/jobs_u/get/{ordered_result_job_id}?token={token}&no_logs=true")).await;
    assert_eq!(job_response.result.unwrap().get(), correct_result);

    let job_response: JobResponse = get_result(format!("http://localhost:{port}/api/w/test-workspace/jobs_u/completed/get_result_maybe/{ordered_result_job_id}?token={token}&no_logs=true")).await;
    assert_eq!(job_response.result.unwrap().get(), correct_result);

    let job_result: Box<serde_json::value::RawValue> = get_result(format!("http://localhost:{port}/api/w/test-workspace/jobs_u/completed/get_result/{ordered_result_job_id}?token={token}&no_logs=true")).await;
    assert_eq!(job_result.get(), correct_result);

    let response = windmill_api::jobs::run_wait_result(
        &db.into(),
        Uuid::parse_str(ordered_result_job_id).unwrap(),
        "test-workspace".to_string(),
        None,
        "test-user",
    )
    .await
    .unwrap();
    let result: Box<serde_json::value::RawValue> = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert_eq!(result.get(), correct_result);
}

#[sqlx::test(fixtures("base"))]
async fn test_job_labels(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    let db = &db;
    let test = |original_labels: &'static [&'static str]| async move {
        let job = RunJob::from(JobPayload::RawFlow {
            value: serde_json::from_value(json!({
                "modules": [{
                    "id": "a",
                    "value": {
                        "type": "rawscript",
                        "content": r#"export function main(world: string) {
                        const greet = `Hello ${world}!`;
                        console.log(greet)
                        return { greet, wm_labels: ["yolo", "greet", "greet", world] };
                    }"#,
                        "language": "deno",
                        "input_transforms": {
                            "world": { "type": "javascript", "expr": "flow_input.world" }
                        }
                    }
                }],
                "schema": {
                    "$schema": "https://json-schema.org/draft/2020-12/schema",
                    "properties": { "world": { "type": "string" } },
                    "type": "object",
                    "order": [ "world" ]
                }
            }))
            .unwrap(),
            path: None,
            restarted_from: None,
        })
        .arg("world", json!("you"))
        .run_until_complete_with(&db, port, |id| async move {
            sqlx::query!(
                "UPDATE v2_job SET labels = $2 WHERE id = $1 AND $2::TEXT[] IS NOT NULL",
                id,
                original_labels as &[&str],
            )
            .execute(db)
            .await
            .unwrap();
        })
        .await;

        let result = job.json_result().unwrap();
        assert_eq!(result.get("greet"), Some(&json!("Hello you!")));
        let labels = sqlx::query_scalar!("SELECT labels FROM v2_job WHERE id = $1", job.id)
            .fetch_one(db)
            .await
            .unwrap();
        let mut expected_labels = original_labels
            .iter()
            .chain(&["yolo", "greet", "you"])
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        expected_labels.sort();
        assert_eq!(labels, Some(expected_labels));
    };
    test(&[]).await;
    test(&["z", "a", "x"]).await;
}

#[cfg(feature = "python")]
const WORKFLOW_AS_CODE: &str = r#"
from wmill import task

import pandas as pd
import numpy as np

@task()
def heavy_compute(n: int):
    df = pd.DataFrame(np.random.randn(100, 4), columns=list('ABCD'))
    return df.sum().sum()

@task
def send_result(res: int, email: str):
    print(f"Sending result {res} to {email}")
    return "OK"

def main(n: int):
    l = []
    for i in range(n):
        l.append(heavy_compute(i))
    print(l)
    return [send_result(sum(l), "example@example.com"), n]
"#;

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "hello"))]
async fn test_workflow_as_code(db: Pool<Postgres>) {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await;
    let port = server.addr.port();

    // workflow as code require at least 2 workers:
    let db = &db;
    in_test_worker(
        &db,
        async move {
            let job = RunJob::from(JobPayload::Code(RawCode {
                language: ScriptLang::Python3,
                content: WORKFLOW_AS_CODE.into(),
                ..RawCode::default()
            }))
            .arg("n", json!(3))
            .run_until_complete(&db, port)
            .await;

            assert_eq!(job.json_result().unwrap(), json!(["OK", 3]));

            let workflow_as_code_status = sqlx::query_scalar!(
                "SELECT workflow_as_code_status FROM v2_job_completed WHERE id = $1",
                job.id
            )
            .fetch_one(db)
            .await
            .unwrap()
            .unwrap();

            #[derive(Deserialize)]
            #[allow(dead_code)]
            struct WorkflowJobStatus {
                name: String,
                started_at: String,
                scheduled_for: String,
                duration_ms: i64,
            }

            let workflow_as_code_status: std::collections::HashMap<String, WorkflowJobStatus> =
                serde_json::from_value(workflow_as_code_status).unwrap();

            let uuids = sqlx::query_scalar!("SELECT id FROM v2_job WHERE parent_job = $1", job.id)
                .fetch_all(db)
                .await
                .unwrap();

            assert_eq!(uuids.len(), 4);
            for uuid in uuids {
                let status = workflow_as_code_status.get(&uuid.to_string());
                assert!(status.is_some());
                assert!(
                    status.unwrap().name == "send_result"
                        || status.unwrap().name == "heavy_compute"
                );
            }
        },
        port,
    )
    .await;
}

async fn test_for_versions<F: Future<Output = ()>>(
    version_flags: impl Iterator<Item = Arc<RwLock<bool>>>,
    test: impl Fn() -> F,
) {
    for version_flag in version_flags {
        *version_flag.write().await = true;
        test().await;
    }
}

mod job_payload {
    use super::*;

    use lazy_static::lazy_static;

    use windmill_common::cache;
    use windmill_common::flows::FlowNodeId;

    lazy_static! {
        static ref VERSION_FLAGS: [Arc<RwLock<bool>>; 3] = [
            MIN_VERSION_IS_AT_LEAST_1_427.clone(),
            MIN_VERSION_IS_AT_LEAST_1_432.clone(),
            MIN_VERSION_IS_AT_LEAST_1_440.clone(),
        ];
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_script_hash_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123412),
                path: "f/system/hello".to_string(),
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                language: ScriptLang::Deno,
                priority: None,
                apply_preprocessor: false,
            })
            .arg("world", json!("foo"))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello foo!"));
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_script_hash_payload_with_preprocessor(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123413),
                path: "f/system/hello_with_preprocessor".to_string(),
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                language: ScriptLang::Deno,
                priority: None,
                apply_preprocessor: true,
            })
            .run_until_complete_with(db, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("bar")));
            assert_eq!(args.get("bar"), Some(&json!("baz")));
            assert_eq!(job.json_result().unwrap(), json!("Hello bar baz"));
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_script_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
        })
        .run_until_complete(&db, port)
        .await
        .json_result()
        .unwrap();

        let flow_data = cache::flow::fetch_version_lite(&db, 1443253234253454)
            .await
            .unwrap();
        let flow_value = flow_data.value();
        let flow_scripts = {
            async fn load(db: &Pool<Postgres>, modules: &[FlowModule]) -> Vec<FlowNodeId> {
                let mut res = vec![];
                for module in modules {
                    let value =
                        serde_json::from_str::<FlowModuleValue>(module.value.get()).unwrap();
                    match value {
                        FlowModuleValue::FlowScript { id, .. } => res.push(id),
                        FlowModuleValue::ForloopFlow { modules_node: Some(flow_node), .. } => {
                            let flow_data = cache::flow::fetch_flow(db, flow_node).await.unwrap();
                            res.extend(Box::pin(load(db, &flow_data.value().modules)).await);
                        }
                        _ => {}
                    }
                }
                res
            }

            load(&db, &flow_value.modules).await
        };
        assert_eq!(flow_scripts.len(), 2);

        let test = || async {
            let result = RunJob::from(JobPayload::FlowScript {
                id: flow_scripts[0],
                language: ScriptLang::Deno,
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                path: "f/system/hello/test-0".into(),
            })
            .arg("world", json!("foo"))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello foo!"));
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
        let test = || async {
            let result = RunJob::from(JobPayload::FlowScript {
                id: flow_scripts[1],
                language: ScriptLang::Deno,
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                path: "f/system/hello/test-0".into(),
            })
            .arg("hello", json!("You know nothing Jean Neige"))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!("Did you just say \"You know nothing Jean Neige\"??!")
            );
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_node_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
        })
        .run_until_complete(&db, port)
        .await
        .json_result()
        .unwrap();

        let flow_data = cache::flow::fetch_version_lite(&db, 1443253234253454)
            .await
            .unwrap();
        let flow_value = flow_data.value();
        let forloop_module =
            serde_json::from_str::<FlowModuleValue>(flow_value.modules[0].value.get()).unwrap();
        let FlowModuleValue::ForloopFlow { modules_node: Some(id), .. } = forloop_module else {
            panic!("Expected a forloop module with a flow node");
        };

        let test = || async {
            let result = RunJob::from(JobPayload::FlowNode {
                id,
                path: "f/system/hello_with_nodes_flow/forloop-0".into(),
            })
            .arg("iter", json!({ "value": "tests", "index": 0 }))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Did you just say \"Hello tests!\"??!"));
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dependencies_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::Dependencies {
                path: "f/system/hello".to_string(),
                hash: ScriptHash(123412),
                language: ScriptLang::Deno,
                dedicated_worker: None,
            })
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result.get("status").unwrap(),
                &json!("Successful lock file generation")
            );
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    // Just test that deploying a flow work as expected.
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_dependencies_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::FlowDependencies {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                version: 1443253234253454,
            })
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result.get("status").unwrap(),
                &json!("Successful lock file generation")
            );
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_flow_dependencies_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::RawFlowDependencies {
                path: "none".to_string(),
                flow_value: serde_json::from_value(json!({
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string) {
                                const greet = `Hello ${world}!`;
                                console.log(greet)
                                return greet
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" }
                            }
                        }
                    }],
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "properties": { "world": { "type": "string" } },
                        "type": "object",
                        "order": [  "world" ]
                    }
                }))
                .unwrap(),
            })
            .arg("skip_flow_update", json!(true))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            let result = RunJob::from(JobPayload::RawFlow {
                value: serde_json::from_value::<FlowValue>(
                    result.get("updated_flow_value").unwrap().clone(),
                )
                .unwrap(),
                path: None,
                restarted_from: None,
            })
            .arg("world", json!("Jean Neige"))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello Jean Neige!"));
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_script_dependencies_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::RawScriptDependencies {
                script_path: "none".into(),
                content: r#"export function main(world: string) {
                    const greet = `Hello ${world}!`;
                    console.log(greet)
                    return greet
                }"#
                .into(),
                language: ScriptLang::Deno,
            })
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!({ "lock": "", "status": "Successful lock file generation" })
            );
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                apply_preprocessor: false,
                version: 1443253234253454,
            })
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );
        };
        // Test the not "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
        })
        .run_until_complete(&db, port)
        .await
        .json_result()
        .unwrap();
        // Test the "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_payload_with_preprocessor(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let db = &db;
        let test = || async {
            let job = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_preprocessor".to_string(),
                dedicated_worker: None,
                apply_preprocessor: true,
                version: 1443253234253456,
            })
            .run_until_complete_with(db, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            let flow_status = job.flow_status.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("bar")));
            assert_eq!(args.get("bar"), Some(&json!("baz")));
            assert_eq!(job.json_result().unwrap(), json!("Hello bar-baz"));
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
            let flow_status = serde_json::from_value::<FlowStatus>(flow_status.clone()).unwrap();
            let FlowStatusModule::Success { job, .. } = flow_status.preprocessor_module.unwrap()
            else {
                panic!("Expected a success preprocessor module");
            };
            let pp_id = job;
            let job = sqlx::query!(
                "SELECT preprocessed, script_entrypoint_override FROM v2_job WHERE id = $1",
                pp_id
            )
            .fetch_one(db)
            .await
            .unwrap();
            assert_eq!(job.preprocessed, Some(true));
            assert_eq!(
                job.script_entrypoint_override.as_deref(),
                Some("preprocessor")
            );
        };
        // Test the not "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_preprocessor".to_string(),
            dedicated_worker: None,
            version: 1443253234253456,
        })
        .run_until_complete(db, port)
        .await
        .json_result()
        .unwrap();
        // Test the "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_restarted_flow_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let completed_job_id = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                apply_preprocessor: true,
                version: 1443253234253454,
            })
            .run_until_complete(&db, port)
            .await
            .id;

            let result = RunJob::from(JobPayload::RestartedFlow {
                completed_job_id,
                step_id: "a".into(),
                branch_or_iteration_n: None,
            })
            .arg("iter", json!({ "value": "tests", "index": 0 }))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );
        };
        // Test the not "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
        })
        .run_until_complete(&db, port)
        .await
        .json_result()
        .unwrap();
        // Test the "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_flow_payload(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::RawFlow {
                value: serde_json::from_value(json!({
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string) {
                                const greet = `Hello ${world}!`;
                                console.log(greet)
                                return greet
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" }
                            }
                        }
                    }],
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "properties": { "world": { "type": "string" } },
                        "type": "object",
                        "order": [  "world" ]
                    }
                }))
                .unwrap(),
                path: None,
                restarted_from: None,
            })
            .arg("world", json!("Jean Neige"))
            .run_until_complete(&db, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello Jean Neige!"));
        };
        test_for_versions(VERSION_FLAGS.iter().cloned(), test).await;
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_flow_payload_with_restarted_from(db: Pool<Postgres>) {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let db = &db;
        let test = |restarted_from, arg, result| async move {
            let job = RunJob::from(JobPayload::RawFlow {
                value: serde_json::from_value(json!({
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string) {
                                return `Hello ${world}!`;
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" }
                            }
                        }
                    }, {
                        "id": "b",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string, a: string) {
                                return `${a} ${world}!`;
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" },
                                "a": { "type": "javascript", "expr": "results.a" }
                            }
                        }
                    }, {
                        "id": "c",
                        "value": {
                            "type": "forloopflow",
                            "iterator": { "type": "javascript", "expr": "['a', 'b', 'c']" },
                            "modules": [{
                                "value": {
                                    "input_transforms": {
                                        "world": { "type": "javascript", "expr": "flow_input.world" },
                                        "b": { "type": "javascript", "expr": "results.b" },
                                        "x": { "type": "javascript", "expr": "flow_input.iter.value" }
                                    },
                                    "type": "rawscript",
                                    "language": "deno",
                                    "content": r#"export function main(world: string, b: string, x: string) {
                                        return `${x}: ${b} ${world}!`;
                                    }"#,
                                },
                            }],
                        }
                    }],
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "properties": { "world": { "type": "string" } },
                        "type": "object",
                        "order": [  "world" ]
                    }
                }))
                .unwrap(),
                path: None,
                restarted_from,
            })
            .arg("world", arg)
            .run_until_complete(db, port)
            .await;

            assert_eq!(job.json_result().unwrap(), result);
            job.id
        };
        let flow_job_id = test(
            None,
            json!("foo"),
            json!([
                "a: Hello foo! foo! foo!",
                "b: Hello foo! foo! foo!",
                "c: Hello foo! foo! foo!"
            ]),
        )
        .await;
        let flow_job_id = test(
            Some(RestartedFrom { flow_job_id, step_id: "a".into(), branch_or_iteration_n: None }),
            json!("foo"),
            json!([
                "a: Hello foo! foo! foo!",
                "b: Hello foo! foo! foo!",
                "c: Hello foo! foo! foo!"
            ]),
        )
        .await;
        let flow_job_id = test(
            Some(RestartedFrom { flow_job_id, step_id: "b".into(), branch_or_iteration_n: None }),
            json!("bar"),
            json!([
                "a: Hello foo! bar! bar!",
                "b: Hello foo! bar! bar!",
                "c: Hello foo! bar! bar!"
            ]),
        )
        .await;
        let _ = test(
            Some(RestartedFrom {
                flow_job_id,
                step_id: "c".into(),
                branch_or_iteration_n: Some(1),
            }),
            json!("yolo"),
            json!([
                "a: Hello foo! bar! bar!",
                "b: Hello foo! bar! yolo!",
                "c: Hello foo! bar! yolo!"
            ]),
        )
        .await;
    }
}
