//! AI decision steps run end to end against a stand-in for the decision endpoints: TypeSafe's
//! System One, Cloudflare Workers AI's, which serves each model at its own URL inside an API
//! envelope, and OpenAI's Decisions API.

use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::*;

/// Answers every question as a choice of `refund`, the way TypeSafe shapes an answer.
fn refund_answers(body: &Value) -> Value {
    let answers = body["questions"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, _)| {
            let answer = json!({
                "type": "choice",
                "choice": "refund",
                "probabilities": {"refund": 0.9, "bug": 0.1},
                "confidence": 0.8
            });
            (name.clone(), answer)
        })
        .collect::<serde_json::Map<_, _>>();
    json!({"model": body["model"], "answers": answers, "usage": {"input_tokens": 10}})
}

async fn start_decision_stub() -> anyhow::Result<u16> {
    let app = Router::new()
        .route(
            "/v1/systemone",
            post(|Json(body): Json<Value>| async move { Json(refund_answers(&body)) }),
        )
        .route(
            "/accounts/acc/ai/run/@cf/cloudflare/clef-flash",
            post(|Json(body): Json<Value>| async move {
                Json(json!({"result": refund_answers(&body), "success": true, "errors": []}))
            }),
        )
        // OpenAI's Decisions API takes the questions as a list and answers with one.
        .route(
            "/openai/v1/decisions",
            post(|Json(body): Json<Value>| async move {
                let answers = body["questions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|q| json!({"type": "choice", "name": q["name"], "choice": "refund"}))
                    .collect::<Vec<_>>();
                Json(json!({"answers": answers}))
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Ok(port)
}

/// A decision's answers reach later steps as `results.<id>.output`, which a Branch to one after it
/// routes on.
#[cfg(feature = "quickjs")]
#[sqlx::test(fixtures("base"))]
async fn test_ai_decision_answers_route_a_following_branch(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // The stub listens on loopback, which the SSRF check refuses without this.
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let stub_port = start_decision_stub().await?;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for provider in [
        json!({
            "kind": "typesafe",
            "resource": {"api_key": "key", "base_url": format!("http://127.0.0.1:{stub_port}/v1")},
            "model": "jev-latest"
        }),
        json!({
            "kind": "cloudflare",
            "resource": {
                "token": "key",
                "base_url": format!("http://127.0.0.1:{stub_port}/accounts/acc/ai/run/@cf/cloudflare")
            },
            "model": "clef-flash"
        }),
        json!({
            "kind": "openai",
            "resource": {"api_key": "key", "base_url": format!("http://127.0.0.1:{stub_port}/openai/v1")},
            "model": "gpt-6-luna"
        }),
    ] {
        let flow: FlowValue = serde_json::from_value(json!({
            "modules": [
                {"id": "d", "value": {
                    "type": "aidecision",
                    "input_transforms": {
                        "provider": {"type": "static", "value": provider},
                        "state": {"type": "static", "value": "I was charged twice"},
                        "questions": {"type": "static", "value": {"intent": {
                            "type": "choice",
                            "instructions": "What does the customer want?",
                            "criteria": {"refund": "Money back", "bug": "Something is broken"}
                        }}}
                    }
                }},
                {"id": "route", "value": {
                    "type": "branchone",
                    "branches": [
                        {
                            "expr": "results.d.output.intent.choice === 'bug'",
                            "modules": [{"id": "b", "value": {"type": "identity"}}]
                        },
                        {
                            "expr": "results.d.output.intent.choice === 'refund'",
                            "modules": [{"id": "r", "value": {
                                "type": "rawscript",
                                "language": "bun",
                                "content": "export function main(choice){ return 'handled ' + choice }",
                                "input_transforms": {"choice": {
                                    "type": "javascript",
                                    "expr": "results.d.output.intent.choice"
                                }}
                            }}]
                        }
                    ],
                    "default": []
                }}
            ]
        }))?;

        let job = run_job_in_new_worker_until_complete(
            &db,
            false,
            JobPayload::RawFlow { value: flow, path: None, restarted_from: None },
            port,
        )
        .await;
        assert_eq!(
            job.json_result(),
            Some(json!("handled refund")),
            "{provider}"
        );
    }
    Ok(())
}
