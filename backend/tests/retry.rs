#[cfg(feature = "deno_core")]
mod retry {
    use windmill_test_utils::*;
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use windmill_common::flow_status::FlowStatusModule;
    use windmill_common::flows::FlowValue;
    use windmill_common::jobs::JobPayload;

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

    #[cfg(feature = "deno_core")]
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
    async fn test_pass(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        // let server = ApiServer::start(db.clone()).await?;

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
        .run_until_complete(&db, false, server.addr.port())
        .await
        .json_result()
        .unwrap();

        assert_eq!(server.close().await, attempts);
        assert_eq!(json!([3, 5, 7, 9]), result);
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_fail_step_zero(db: Pool<Postgres>) -> anyhow::Result<()> {
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
        .run_until_complete(&db, false, server.addr.port())
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
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_fail_step_one(db: Pool<Postgres>) -> anyhow::Result<()> {
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
        .run_until_complete(&db, false, server.addr.port())
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
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base"))]
    async fn test_with_failure_module(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        // let server = ApiServer::start(db.clone()).await?;

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
            .run_until_complete(&db, false, server.addr.port())
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
        Ok(())
    }
}
