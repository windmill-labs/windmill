use futures::{stream, Stream};
use serde_json::json;
use sqlx::{postgres::PgListener, query_scalar, types::Uuid, Pool, Postgres, Transaction};
use windmill_api::jobs::{CompletedJob, Job};
use windmill_common::{flows::FlowValue, DEFAULT_SLEEP_QUEUE};
use windmill_queue::{get_queued_job, JobPayload};
use windmill_worker::WorkerConfig;

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

pub async fn get_job_by_id<'c>(
    mut tx: Transaction<'c, Postgres>,
    w_id: &str,
    id: Uuid,
) -> windmill_common::error::Result<(Option<Job>, Transaction<'c, Postgres>)> {
    let cjob_option = sqlx::query_as::<_, CompletedJob>(
        "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
    )
    .bind(id)
    .bind(w_id)
    .fetch_optional(&mut tx)
    .await?;
    let job_option = match cjob_option {
        Some(job) => Some(Job::CompletedJob(job)),
        None => get_queued_job(id, w_id, &mut tx).await?.map(Job::QueuedJob),
    };
    if job_option.is_some() {
        Ok((job_option, tx))
    } else {
        // check if a job had been moved in-between queries
        let cjob_option = sqlx::query_as::<_, CompletedJob>(
            "SELECT * FROM completed_job WHERE id = $1 AND workspace_id = $2",
        )
        .bind(id)
        .bind(w_id)
        .fetch_optional(&mut tx)
        .await?;
        Ok((cjob_option.map(Job::CompletedJob), tx))
    }
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

        let task = tokio::task::spawn({
            windmill_api::run_server(
                db.clone(),
                addr,
                format!("http://localhost:{}", addr.port()),
                rx,
            )
        });

        return Self { addr, tx, task };
    }

    async fn close(self) -> anyhow::Result<()> {
        let Self { tx, task, .. } = self;
        drop(tx);
        task.await.unwrap()
    }
}

mod suspend_resume {

    use futures::{Stream, StreamExt};
    use serde_json::json;
    use sqlx::{query_scalar, types::Uuid};
    use windmill_common::flows::FlowValue;
    use windmill_queue::JobPayload;

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

    async fn _print_job(id: Uuid, db: &Pool<Postgres>) -> Result<(), anyhow::Error> {
        tracing::info!(
            "{:#?}",
            get_job_by_id(db.begin().await?, "test-workspace", id)
                .await?
                .0
        );
        Ok(())
    }

    fn flow() -> FlowValue {
        serde_json::from_value(serde_json::json!({
                "modules": [{
                    "input_transform": {
                        "n": { "type": "javascript", "expr": "flow_input.n", },
                        "port": { "type": "javascript", "expr": "flow_input.port", },
                        "op": { "type": "javascript", "expr": "flow_input.op ?? 'resume'", },
                    },
                    "value": {
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
                                    `http://localhost:${port}/api/w/test-workspace/jobs/${op}/${job}/0/${secret}?approver=ruben`,\
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
                    "input_transform": {
                        "n": { "type": "javascript", "expr": "previous_result", },
                        "resume": { "type": "javascript", "expr": "resume", },
                        "resumes": { "type": "javascript", "expr": "resumes", },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "deno",
                        "content": "export function main(n, resume, resumes) { return { n: n + 1, resume, resumes } }"
                    },
                    "suspend": {
                        "required_events": 1
                    },
                }, {
                    "input_transform": {
                        "last": { "type": "javascript", "expr": "previous_result", },
                        "resume": { "type": "javascript", "expr": "resume", },
                        "resumes": { "type": "javascript", "expr": "resumes", },
                    },
                    "value": {
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

        let flow = RunJob::from(JobPayload::RawFlow { value: flow(), path: None })
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

                let tx = db.begin().await.unwrap();
                let (tx, token) = windmill_worker::create_token_for_owner(tx, "test-workspace", "u/test-user", "", 100, "").await.unwrap();
                tx.commit().await.unwrap();
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
                    "http://localhost:{port}/api/w/test-workspace/jobs/resume/{second}/0/{secret}?payload=ImZyb20gdGVzdCIK&approver=ruben"
                ))
                .await
                .unwrap()
                .error_for_status()
                .unwrap();

                completed.find(&flow).await.unwrap();
            }, port)
            .await;

        server.close().await.unwrap();

        let result = completed_job_result(flow, &db).await;

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

        let result = RunJob::from(JobPayload::RawFlow { value: flow(), path: None })
            .arg("n", json!(1))
            .arg("op", json!("cancel"))
            .arg("port", json!(port))
            .run_until_complete(&db, port)
            .await;

        server.close().await.unwrap();

        assert_eq!(
            json!({"error": "Job canceled: approval request disapproved by ruben" }),
            result
        );
    }

    #[sqlx::test(fixtures("base"))]
    async fn cancel_after_suspend(db: Pool<Postgres>) {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await;
        let port = server.addr.port();

        let flow = RunJob::from(JobPayload::RawFlow { value: flow(), path: None })
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

                let tx =db.begin().await.unwrap();
                let (tx, token) = windmill_worker::create_token_for_owner(tx, "test-workspace", "u/test-user", "", 100, "").await.unwrap();
                tx.commit().await.unwrap();
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
                    "http://localhost:{port}/api/w/test-workspace/jobs/cancel/{second}/0/{secret}?payload=ImZyb20gdGVzdCIK"
                ))
                .await
                .unwrap()
                .error_for_status()
                .unwrap();

                completed.find(&flow).await.unwrap();
            }, port)
            .await;

        server.close().await.unwrap();

        let result = completed_job_result(flow, &db).await;

        assert_eq!(
            json!({"error": "Job canceled: approval request disapproved by unknown" }),
            result
        );
    }
}

mod retry {
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use windmill_common::flows::FlowValue;
    use windmill_queue::JobPayload;

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
    if (await sock.read(buf) != 1) throw "read";
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
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "result.items" },
                    "skip_failures": false,
                    "modules": [{
                        "input_transform": {
                            "index": { "type": "javascript", "expr": "previous_result.iter.index" },
                            "port": { "type": "javascript", "expr": "flow_input.port" },
                        },
                        "value": {
                            "type": "rawscript",
                            "language": "deno",
                            "content": inner_step(),
                        },
                    }],
                },
                "retry": { "constant": { "attempts": 2, "seconds": 0 } },
            }, {
                "input_transform": {
                    "last": { "type": "javascript", "expr": "previous_result" },
                    "port": { "type": "javascript", "expr": "flow_input.port" },
                },
                "value": {
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

        let server = ApiServer::start(db.clone()).await;

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
        let result = RunJob::from(JobPayload::RawFlow { value: flow_forloop_retry(), path: None })
            .arg("items", json!(["unused", "unused", "unused"]))
            .arg("port", json!(server.addr.port()))
            .run_until_complete(&db, server.addr.port())
            .await;

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
        let result = RunJob::from(JobPayload::RawFlow { value: flow_forloop_retry(), path: None })
            .arg("items", json!(["unused", "unused", "unused"]))
            .arg("port", json!(server.addr.port()))
            .run_until_complete(&db, server.addr.port())
            .await;

        assert_eq!(server.close().await, attempts);
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains(r#"Uncaught (in promise) "read""#));
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
        let result = RunJob::from(JobPayload::RawFlow { value: flow_forloop_retry(), path: None })
            .arg("items", json!(["unused", "unused", "unused"]))
            .arg("port", json!(server.addr.port()))
            .run_until_complete(&db, server.addr.port())
            .await;

        assert_eq!(server.close().await, attempts);
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("index out of range"));
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_with_failure_module(db: Pool<Postgres>) {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await;

        let value = serde_json::from_value(json!({
            "modules": [{
                "input_transform": { "port": { "type": "javascript", "expr": "flow_input.port" } },
                "value": {
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
                "input_transform": { "error": { "type": "javascript", "expr": "previous_result", },
                                     "port": { "type": "javascript", "expr": "flow_input.port" } },
                "value": {
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
        let (attempts, responses) = [
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
        let result = RunJob::from(JobPayload::RawFlow { value, path: None })
            .arg("port", json!(server.addr.port()))
            .run_until_complete(&db, server.addr.port())
            .await;

        assert_eq!(server.close().await, attempts);
        assert_eq!(
            result,
            json!({
                "recv": 42,
                "from failure module": {
                    "error": "Error during execution of the script:\n\nTraceback (most recent call last):\n  File \"/tmp/main.py\", line 14, in <module>\n    res = inner_script.main(**kwargs)\n  File \"/tmp/inner.py\", line 5, in main\n    return sock.recv(1)[0]\nIndexError: index out of range",
                }
            })
        );
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
                    "input_transform": {
                        "n": {
                            "type": "javascript",
                            "expr": "previous_result.iter.value",
                        },
                    },
                    "value": {
                        "type": "rawscript",
                        "language": "python3",
                        "content": "def main(n):\n    if 1 < n:\n        raise StopIteration(n)",
                    },
                }],
            },
        }],
    }))
    .unwrap();

    let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
        .arg("items", json!([]))
        .run_until_complete(&db, server.addr.port())
        .await;
    assert_eq!(result, serde_json::json!([]));

    /* Don't actually test that this does 257 jobs or that will take forever. */
    let result = RunJob::from(JobPayload::RawFlow { value: flow.clone(), path: None })
        .arg("items", json!((0..257).collect::<Vec<_>>()))
        .run_until_complete(&db, server.addr.port())
        .await;
    assert!(matches!(result, serde_json::Value::Object(_)));
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("StopIteration: 2"));
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
        let tx = db.begin().await.unwrap();
        let (uuid, tx) = windmill_queue::push(
            tx,
            "test-workspace",
            payload,
            Some(args),
            /* user */ "test-user",
            /* permissioned_as */ "u/admin".to_string(),
            /* scheduled_for_o */ None,
            /* schedule_path */ None,
            /* parent_job */ None,
            /* is_flow_step */ false,
            /* running */ false,
        )
        .await
        .unwrap();

        tx.commit().await.unwrap();

        uuid
    }

    /// push the job, spawn a worker, wait until the job is in completed_job
    async fn run_until_complete(self, db: &Pool<Postgres>, port: u16) -> serde_json::Value {
        let uuid = self.push(db).await;
        let listener = listen_for_completed_jobs(db).await;
        in_test_worker(db, listener.find(&uuid), port).await;
        completed_job_result(uuid, db).await
    }
}

async fn run_job_in_new_worker_until_complete(
    db: &Pool<Postgres>,
    job: JobPayload,
    port: u16,
) -> serde_json::Value {
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
    let (quit, worker) = spawn_test_worker(db, port);
    let worker = tokio::time::timeout(std::time::Duration::from_secs(19), worker);
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
    drop(quit);

    let _: () = worker
        .await
        .expect("worker timed out")
        .expect("worker panicked");

    res
}

async fn get_token(db: &sqlx::Pool<sqlx::Postgres>, worker_name: &str) -> String {
    use rand::prelude::*;
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let mut tx = db.begin().await.unwrap();
    sqlx::query!(
        "INSERT INTO token
            (token, email, label, super_admin)
            VALUES ($1, $2, $3, $4)",
        token,
        "worker@windmill.dev",
        format!("worker token for {worker_name}"),
        false
    )
    .execute(&mut tx)
    .await
    .unwrap();

    // TODO: This should be audit logged
    tx.commit().await.unwrap();
    token
}

fn spawn_test_worker(
    db: &Pool<Postgres>,
    port: u16,
) -> (
    tokio::sync::broadcast::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = tokio::sync::broadcast::channel(1);
    let db = db.to_owned();
    let timeout = 4_000;
    let worker_instance: &str = "test worker instance";
    let worker_name: String = next_worker_name();
    let i_worker: u64 = Default::default();
    let num_workers: u64 = 2;
    let ip: &str = Default::default();
    let sleep_queue: u64 = DEFAULT_SLEEP_QUEUE / num_workers;
    let port = port;
    let worker_config = WorkerConfig {
        base_internal_url: format!("http://localhost:{port}"),
        base_url: format!("http://localhost:{port}"),
        disable_nuser: std::env::var("DISABLE_NUSER")
            .ok()
            .and_then(|x| x.parse::<bool>().ok())
            .unwrap_or(false),
        disable_nsjail: std::env::var("DISABLE_NSJAIL")
            .ok()
            .and_then(|x| x.parse::<bool>().ok())
            .unwrap_or(false),
        keep_job_dir: std::env::var("KEEP_JOB_DIR")
            .ok()
            .and_then(|x| x.parse::<bool>().ok())
            .unwrap_or(false),
    };
    let future = async move {
        windmill_worker::run_worker(
            &db,
            &windmill_api_client::create_client(
                &worker_config.base_url,
                get_token(&db, &worker_name).await,
            ),
            timeout,
            worker_instance,
            worker_name,
            i_worker,
            num_workers,
            ip,
            sleep_queue,
            worker_config,
            rx,
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

async fn completed_job_result(uuid: Uuid, db: &Pool<Postgres>) -> serde_json::Value {
    query_scalar("SELECT result FROM completed_job WHERE id = $1")
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
