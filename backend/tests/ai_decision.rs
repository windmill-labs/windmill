//! AI decision steps run end to end against a stand-in for TypeSafe's System One endpoint.

use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_common::{
    flow_status::{BranchChosen, FlowStatus, FlowStatusModule},
    flows::FlowValue,
    jobs::JobPayload,
};
use windmill_test_utils::*;

/// Answers every question as a choice of `refund`, the way TypeSafe shapes an answer.
async fn start_typesafe_stub() -> anyhow::Result<u16> {
    let app = Router::new().route(
        "/v1/systemone",
        post(|Json(body): Json<Value>| async move {
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
            Json(json!({"model": "jev-1.13.0", "answers": answers, "usage": {"input_tokens": 10}}))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    Ok(port)
}

/// The decision chooses its branch from its answers, a branch step reads those answers as
/// `results.<id>`, and the step's result is the branch's.
#[cfg(feature = "quickjs")]
#[sqlx::test(fixtures("base"))]
async fn test_ai_decision_runs_the_branch_its_answer_picks(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    // The stub listens on loopback, which the SSRF check refuses without this.
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let stub_port = start_typesafe_stub().await?;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow: FlowValue = serde_json::from_value(json!({
        "modules": [{
            "id": "d",
            "value": {
                "type": "aidecision",
                "input_transforms": {
                    "provider": {"type": "static", "value": {
                        "kind": "typesafe",
                        "resource": {"api_key": "key", "base_url": format!("http://127.0.0.1:{stub_port}/v1")},
                        "model": "jev-latest"
                    }},
                    "state": {"type": "static", "value": "I was charged twice"},
                    "questions": {"type": "static", "value": {"intent": {
                        "type": "choice",
                        "instructions": "What does the customer want?",
                        "criteria": {"refund": "Money back", "bug": "Something is broken"}
                    }}}
                },
                "branches": [
                    {
                        "expr": "previous_result.output.intent.choice === 'bug'",
                        "modules": [{"id": "b", "value": {"type": "identity"}}]
                    },
                    {
                        "expr": "previous_result.output.intent.choice === 'refund'",
                        "modules": [{"id": "r", "value": {
                            "type": "rawscript",
                            "language": "deno",
                            "content": "export function main(choice){ return 'handled ' + choice }",
                            "input_transforms": {"choice": {
                                "type": "javascript",
                                "expr": "results.d.output.intent.choice"
                            }}
                        }}]
                    }
                ],
                "default": []
            }
        }]
    }))?;

    let job = run_job_in_new_worker_until_complete(
        &db,
        false,
        JobPayload::RawFlow { value: flow, path: None, restarted_from: None },
        port,
    )
    .await;
    assert_eq!(job.json_result(), Some(json!("handled refund")));

    let status: FlowStatus = serde_json::from_value(job.flow_status.expect("flow status"))?;
    let FlowStatusModule::Success { branch_chosen, decision_job, .. } = &status.modules[0] else {
        panic!("decision step did not succeed: {:?}", status.modules[0]);
    };
    assert!(matches!(
        branch_chosen,
        Some(BranchChosen::Branch { branch: 1 })
    ));
    assert!(
        decision_job.is_some(),
        "the decision job is kept once the branch has run"
    );
    Ok(())
}
