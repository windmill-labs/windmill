mod suspend_resume {
    #[cfg(feature = "deno_core")]
    use serde_json::json;

    #[cfg(feature = "deno_core")]
    use windmill_test_utils::*;

    #[cfg(feature = "deno_core")]
    use futures::{Stream, StreamExt};
    #[cfg(feature = "deno_core")]
    use sqlx::types::Uuid;
    #[cfg(feature = "deno_core")]
    use sqlx::{Pool, Postgres};
    #[cfg(feature = "deno_core")]
    use windmill_common::flows::FlowValue;
    #[cfg(feature = "deno_core")]
    use windmill_common::jobs::JobPayload;

    #[cfg(feature = "deno_core")]
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

    #[cfg(feature = "deno_core")]
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

    #[cfg(feature = "deno_core")]
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
    async fn test(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await?;
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

                let token = windmill_common::auth::create_token_for_owner(&db, "test-workspace", "u/test-user", "", 100, "", &Uuid::nil(), None, None).await.unwrap();
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
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn cancel_from_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let result =
            RunJob::from(JobPayload::RawFlow { value: flow(), path: None, restarted_from: None })
                .arg("n", json!(1))
                .arg("op", json!("cancel"))
                .arg("port", json!(port))
                .run_until_complete(&db, false, port)
                .await
                .json_result()
                .unwrap();

        server.close().await.unwrap();

        assert_eq!(
            json!( {"error": {"name": "SuspendedDisapproved", "message": "Disapproved by ruben"}}),
            result
        );
        Ok(())
    }

    /// Test that self-approval is blocked when self_approval_disabled is true.
    ///
    /// This test verifies that when a flow has an approval step with self_approval_disabled=true,
    /// the user who triggered the flow cannot approve it themselves via the owner endpoint
    /// (POST /jobs/flow/resume/:id).
    ///
    /// Bug context: The owner endpoint was missing the approval condition check, allowing
    /// users to bypass self-approval restrictions by using the UI resume button instead
    /// of the HMAC-signed approval link.
    #[cfg(feature = "enterprise")]
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_self_approval_disabled_blocks_owner_resume(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Flow with self_approval_disabled=true on the approval step
        let flow_with_self_approval_disabled: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main() { return 'step1'; }"
                },
                "suspend": {
                    "required_events": 1,
                    "user_auth_required": true,
                    "self_approval_disabled": true
                }
            }, {
                "id": "b",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main() { return 'step2 - after approval'; }"
                }
            }]
        }))
        .unwrap();

        // Push flow as NON-ADMIN user (test-user-2) - admins bypass self-approval check
        // Use a path owned by test-user-2 so require_owner_of_path succeeds
        let flow = RunJob::from(JobPayload::RawFlow {
            value: flow_with_self_approval_disabled,
            path: Some("u/test-user-2/test_approval".to_string()),
            restarted_from: None,
        })
        .push_as(&db, "test-user-2", "test2@windmill.dev")
        .await;

        let queue = listen_for_queue(&db).await;
        let db_ = db.clone();

        in_test_worker(
            &db,
            async move {
                let db = db_;

                // Wait for flow to suspend at approval step
                wait_until_flow_suspends(flow, queue, &db).await;

                // Create a token for the same non-admin user who triggered the flow
                // This simulates clicking "Resume" in the UI as the flow owner
                // Args: db, w_id, owner, label, expires_in, email, job_id, perms, audit_span
                let token = windmill_common::auth::create_token_for_owner(
                    &db,
                    "test-workspace",
                    "u/test-user-2",
                    "test-token",
                    100,
                    "test2@windmill.dev",
                    &Uuid::nil(),
                    None,
                    None,
                )
                .await
                .unwrap();

                // Try to resume via the owner endpoint (POST /jobs/flow/resume/:id)
                // This should FAIL because self_approval_disabled=true and the user
                // is the same as the one who triggered the flow
                let response = reqwest::Client::new()
                    .post(format!(
                        "http://localhost:{port}/api/w/test-workspace/jobs/flow/resume/{flow}"
                    ))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body("{}")
                    .send()
                    .await
                    .unwrap();

                let status = response.status();

                // The request should be rejected with 403 Forbidden
                // (currently this test FAILS because the bug allows self-approval)
                assert!(
                    status == reqwest::StatusCode::FORBIDDEN,
                    "Self-approval should be blocked when self_approval_disabled=true. \
                     Expected 403 Forbidden, got {}. Response: {}",
                    status,
                    response.text().await.unwrap_or_default()
                );
            },
            port,
        )
        .await;

        server.close().await.unwrap();
        Ok(())
    }

    /// Test that self-approval WORKS when self_approval_disabled is false (default behavior).
    ///
    /// This is the complementary test to test_self_approval_disabled_blocks_owner_resume.
    /// When self_approval_disabled is NOT set, the flow owner should be able to approve
    /// their own flow via the owner endpoint.
    #[cfg(feature = "enterprise")]
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_self_approval_allowed_when_not_disabled(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Flow with user_auth_required but WITHOUT self_approval_disabled
        let flow_without_self_approval_disabled: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main() { return 'step1'; }"
                },
                "suspend": {
                    "required_events": 1,
                    "user_auth_required": true
                    // self_approval_disabled is NOT set (defaults to false)
                }
            }, {
                "id": "b",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main() { return 'step2 - after approval'; }"
                }
            }]
        }))
        .unwrap();

        // Push flow as non-admin user
        let flow = RunJob::from(JobPayload::RawFlow {
            value: flow_without_self_approval_disabled,
            path: Some("u/test-user-2/test_approval_allowed".to_string()),
            restarted_from: None,
        })
        .push_as(&db, "test-user-2", "test2@windmill.dev")
        .await;

        let queue = listen_for_queue(&db).await;
        let db_ = db.clone();

        in_test_worker(
            &db,
            async move {
                let db = db_;

                // Wait for flow to suspend at approval step
                wait_until_flow_suspends(flow, queue, &db).await;

                // Create a token for the same user who triggered the flow
                let token = windmill_common::auth::create_token_for_owner(
                    &db,
                    "test-workspace",
                    "u/test-user-2",
                    "test-token",
                    100,
                    "test2@windmill.dev",
                    &Uuid::nil(),
                    None,
                    None,
                )
                .await
                .unwrap();

                // Try to resume via the owner endpoint - this SHOULD succeed
                let response = reqwest::Client::new()
                    .post(format!(
                        "http://localhost:{port}/api/w/test-workspace/jobs/flow/resume/{flow}"
                    ))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body("{}")
                    .send()
                    .await
                    .unwrap();

                let status = response.status();

                // Self-approval should be allowed when self_approval_disabled is not set
                assert!(
                    status.is_success(),
                    "Self-approval should be allowed when self_approval_disabled is not set. \
                     Expected 2xx, got {}. Response: {}",
                    status,
                    response.text().await.unwrap_or_default()
                );
            },
            port,
        )
        .await;

        server.close().await.unwrap();
        Ok(())
    }

    /// Test that a DIFFERENT user can approve a flow even when self_approval_disabled is true.
    ///
    /// This verifies that the self_approval_disabled setting only blocks the flow trigger,
    /// not other users. A different user should always be able to approve.
    #[cfg(feature = "enterprise")]
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn test_different_user_can_approve_when_self_approval_disabled(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Flow with self_approval_disabled=true
        let flow_with_self_approval_disabled: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main() { return 'step1'; }"
                },
                "suspend": {
                    "required_events": 1,
                    "user_auth_required": true,
                    "self_approval_disabled": true
                }
            }, {
                "id": "b",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "content": "export function main() { return 'step2 - after approval'; }"
                }
            }]
        }))
        .unwrap();

        // Push flow as test-user-2
        let flow = RunJob::from(JobPayload::RawFlow {
            value: flow_with_self_approval_disabled,
            path: Some("u/test-user-2/test_approval_by_other".to_string()),
            restarted_from: None,
        })
        .push_as(&db, "test-user-2", "test2@windmill.dev")
        .await;

        let queue = listen_for_queue(&db).await;
        let db_ = db.clone();

        in_test_worker(
            &db,
            async move {
                let db = db_;

                // Wait for flow to suspend at approval step
                wait_until_flow_suspends(flow, queue, &db).await;

                // Create a token for a DIFFERENT user (test-user, who is admin)
                // This simulates a different person approving the flow
                let token = windmill_common::auth::create_token_for_owner(
                    &db,
                    "test-workspace",
                    "u/test-user",
                    "test-token",
                    100,
                    "test@windmill.dev",
                    &Uuid::nil(),
                    None,
                    None,
                )
                .await
                .unwrap();

                // Try to resume via the owner endpoint as a different user - this SHOULD succeed
                let response = reqwest::Client::new()
                    .post(format!(
                        "http://localhost:{port}/api/w/test-workspace/jobs/flow/resume/{flow}"
                    ))
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", "application/json")
                    .body("{}")
                    .send()
                    .await
                    .unwrap();

                let status = response.status();

                // A different user should be able to approve even with self_approval_disabled
                assert!(
                    status.is_success(),
                    "Different user should be able to approve even with self_approval_disabled=true. \
                     Expected 2xx, got {}. Response: {}",
                    status,
                    response.text().await.unwrap_or_default()
                );
            },
            port,
        )
        .await;

        server.close().await.unwrap();
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base"))]
    async fn cancel_after_suspend(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;

        let server = ApiServer::start(db.clone()).await?;
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

                let token = windmill_common::auth::create_token_for_owner(&db, "test-workspace", "u/test-user", "", 100, "", &Uuid::nil(), None, None).await.unwrap();
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
        Ok(())
    }
}
