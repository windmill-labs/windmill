use std::str::FromStr;
use windmill_api_client::types::{NewScript, NewScriptLanguage};

#[cfg(feature = "enterprise")]
use chrono::Timelike;
use futures::StreamExt;

use futures::{stream, Stream};
use serde::Deserialize;
use serde_json::json;
use sqlx::{postgres::PgListener, types::Uuid, Pool, Postgres};

#[cfg(feature = "enterprise")]
use tokio::time::{timeout, Duration};

use windmill_api_client::types::{CreateFlowBody, RawScript};

use sqlx::query;

#[cfg(feature = "enterprise")]
use windmill_api_client::types::{EditSchedule, NewSchedule, ScriptArgs};

use serde::Serialize;
use windmill_common::auth::JWT_SECRET;
use windmill_common::worker::WORKER_CONFIG;
use windmill_common::{
    flow_status::{FlowStatus, FlowStatusModule, RestartedFrom},
    flows::{FlowModule, FlowModuleValue, FlowValue, InputTransform},
    jobs::{JobKind, JobPayload, RawCode},
    scripts::{ScriptHash, ScriptLang},
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
    ONCE.call_once(windmill_common::tracing_init::initialize_tracing);
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
            None,
            addr,
            rx,
            port_tx,
            false,
            format!("http://localhost:{}", addr.port()),
        ));

        _port_rx.await.unwrap();

        return Self { addr, tx, task };
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

fn get_module(cjob: &CompletedJob, id: &str) -> Option<FlowStatusModule> {
    cjob.flow_status.clone().and_then(|fs| {
        find_module_in_vec(
            serde_json::from_value::<FlowStatus>(fs).unwrap().modules,
            id,
        )
    })
}

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
    use sqlx::query_scalar;

    use super::*;

    async fn wait_until_flow_suspends(
        flow: Uuid,
        mut queue: impl Stream<Item = Uuid> + Unpin,
        db: &Pool<Postgres>,
    ) {
        loop {
            queue.by_ref().find(&flow).await.unwrap();
            if query_scalar("SELECT suspend > 0 FROM queue WHERE id = $1")
                .bind(flow)
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

                let token = windmill_worker::create_token_for_owner(&db, "test-workspace", "u/test-user", "", 100, "", &Uuid::nil()).await.unwrap();
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
            query_scalar::<_, i64>("SELECT count(*) FROM resume_job")
                .fetch_one(&db)
                .await
                .unwrap()
        );
    }

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
            json!( {"error": {"name": "SuspendedDisapproved", "message": "Disapproved by Some(\"ruben\")"}}),
            result
        );
    }

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

                let token = windmill_worker::create_token_for_owner(&db, "test-workspace", "u/test-user", "", 100, "", &Uuid::nil()).await.unwrap();
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
            json!( {"error": {"name": "SuspendedDisapproved", "message": "Disapproved by None"}}),
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

        let tx = PushIsolationLevel::IsolatedRoot(db.clone(), None);
        let (uuid, tx) = windmill_queue::push::<rsmq_async::MultiplexedRsmq>(
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
            /* running */ false,
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
    quit.send(()).expect("send");

    let _: () = worker
        .await
        .expect("worker timed out")
        .expect("worker panicked");
    res
}

fn spawn_test_worker(
    db: &Pool<Postgres>,
    port: u16,
) -> (
    tokio::sync::broadcast::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    for x in [
        windmill_worker::LOCK_CACHE_DIR,
        windmill_worker::GO_BIN_CACHE_DIR,
    ] {
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(x)
            .expect("could not create initial worker dir");
    }

    let (tx, rx) = tokio::sync::broadcast::channel(1);
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
            windmill_common::worker::make_suspended_pull_query(&wc).await;
            windmill_common::worker::make_pull_query(&wc).await;
        }
        windmill_worker::run_worker::<rsmq_async::MultiplexedRsmq>(
            &db,
            worker_instance,
            worker_name,
            1,
            1,
            ip,
            rx,
            tx2,
            &base_internal_url,
            None,
            false,
        )
        .await
    };

    (tx, tokio::task::spawn(future))
}

async fn listen_for_completed_jobs(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "insert on completed_job").await
}

async fn listen_for_queue(db: &Pool<Postgres>) -> impl Stream<Item = Uuid> + Unpin {
    listen_for_uuid_on(db, "queue").await
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
        "SELECT *, result->'wm_labels' as labels FROM completed_job  WHERE id = $1",
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
                    }
                    .into(),
                    stop_after_if: Default::default(),
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
                            }
                            .into(),
                            stop_after_if: Default::default(),
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
                        }],
                    }
                    .into(),
                    stop_after_if: Default::default(),
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
                    }.into(),
                    stop_after_if: Default::default(),
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
                                }.into(),
                                stop_after_if: Default::default(),
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
                                }.into(),
                                stop_after_if: Default::default(),
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

                            },
                        ],
                    }.into(),
                    stop_after_if: Default::default(),
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
                    }.into(),
                    stop_after_if: Default::default(),
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
                        "content": "import * as wmill from \"https://deno.land/x/windmill@v1.50.0/mod.ts\"\n\nexport async function main() {\n  return \"Hello\"\n}\n",
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
        .get_flow_by_path("test-workspace", "g/all/flow_lock_all")
        .await
        .unwrap()
        .into_inner()
        .subtype_0
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
                }) if lock.len() > 0)
            );
        });
}

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

            let completed_job =
                query!("SELECT script_path FROM completed_job  WHERE id = $1", uuid)
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
                timezone: "UTC".to_string(),
                schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
                ws_error_handler_muted: None,
                retry: None,
                summary: None,
                no_flow_overlap: None,
                tag: None,
                paused_until: None,
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
                query!("SELECT script_path FROM completed_job  WHERE id = $1", uuid)
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

            let completed_job =
                query!("SELECT script_path FROM completed_job  WHERE id = $1", uuid)
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
                timezone: "UTC".to_string(),
                schedule: format!("{} {} * * * *", then.second(), then.minute()).to_string(),
                ws_error_handler_muted: None,
                retry: None,
                summary: None,
                no_flow_overlap: None,
                tag: None,
                paused_until: None,
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
                query!("SELECT script_path FROM completed_job  WHERE id = $1", uuid)
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

            let script = query!(
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
