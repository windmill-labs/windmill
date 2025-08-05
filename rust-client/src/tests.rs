use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use super::*;

fn wm() -> Windmill {
    Windmill::new(
        // Some("<WM_TOKEN>".into()),
        None,
        Some("admins".into()),
        // Some("storage".into()),
        Some("http://localhost:8000".into()),
    )
    .unwrap()
}

#[cfg(not(feature = "async"))]
#[test]
fn create() {
    wm();
}

#[cfg(not(feature = "async"))]
#[test]
fn resources() {
    #[derive(Deserialize, Debug, PartialEq, PartialOrd)]
    struct MatrixResource {
        token: String,
        #[allow(non_snake_case)]
        baseUrl: String,
    }
    let wm = wm();
    let path = format!("f/tests/delete_me_{}", Uuid::new_v4());

    {
        wm.set_resource(
            Some(json!({
                "token": "token",
                "baseUrl": "url"
            })),
            &path,
            "matrix",
        )
        .unwrap();

        let matrix: MatrixResource = wm.get_resource(&path).unwrap();

        assert_eq!(
            MatrixResource {
                token: "token".into(),
                baseUrl: "url".into()
            },
            matrix
        );
    }
}

#[cfg(not(feature = "async"))]
#[test]
fn variables() {
    let wm = wm();
    let path = format!("f/tests/delete_me_{}", Uuid::new_v4());

    {
        // Create new
        wm.set_variable(
            ":0".into(),
            &path,
            // TODO: Test with true
            false,
        )
        .unwrap();

        // Read
        let out_val = wm.get_variable_raw(&path).unwrap();
        assert_eq!(":0", &out_val);
    }
    {
        // Update
        let in_val = json!({
            "a": true,
            "b": "c"
        });

        wm.set_variable(
            // Serialize as JSON
            serde_yaml::to_string(&in_val).unwrap(),
            &path,
            false,
        )
        .unwrap();

        // Read Updated
        let out_val = wm.get_variable(&path).unwrap();

        assert_eq!(in_val, out_val);
    }
}

#[cfg(not(feature = "async"))]
#[test]
fn create_and_run() {
    let wm = wm();

    let path = format!("f/tests/delete_me_{}", Uuid::new_v4());
    let resp = wm
        .call_api(apis::script_api::create_script(
            &wm.client_config,
            &wm.workspace,
            windmill_api::models::NewScript {
                path: path.clone(),
                parent_hash: None,
                summary: "auto-generated script from rust sdk | testing".into(),
                description: "auto-generated script from rust sdk | testing".into(),
                content: r#"fn main() -> Result<String, String> { Ok("Hello World!".to_owned()) }"#
                    .into(),
                schema: None,
                is_template: None,
                lock: None,
                language: windmill_api::models::ScriptLang::Rust,
                kind: Some(windmill_api::models::new_script::Kind::Script),
                tag: Some("rust".into()),
                draft_only: None,
                envs: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                ws_error_handler_muted: None,
                priority: None,
                restart_unless_cancelled: None,
                timeout: None,
                delete_after_use: None,
                deployment_message: None,
                concurrency_key: None,
                visible_to_runner_only: None,
                no_main_func: None,
                codebase: None,
                has_preprocessor: None,
                on_behalf_of_email: None,
                assets: None,
            },
        ))
        .unwrap();

    assert_eq!(
        json!("Hello World!"),
        wm.run_script_sync(
            //
            &resp,
            true,
            json!(null),
            None,
            None,
            true,
            false,
        )
        .unwrap()
    );
}
#[cfg(not(feature = "async"))]
#[test]
fn test_within() {
    let wm = wm();

    let path = format!("f/tests/delete_me_{}", Uuid::new_v4());
    let resp = wm
        .call_api(apis::script_api::create_script(
            &wm.client_config,
            &wm.workspace,
            windmill_api::models::NewScript {
                path: path.clone(),
                parent_hash: None,
                summary: "auto-generated script from rust sdk | testing".into(),
                description: "auto-generated script from rust sdk | testing".into(),
                content: r#"//! Add dependencies in the following partial Cargo.toml manifest
//!
//! ```cargo
//! [dependencies]
//! anyhow = "1.0.86"
//! rand = "0.7.2"
//! wmill = { path = "<CWD>"  }
//! ```
//!
//! Note that serde is used by default with the `derive` feature.
//! You can still reimport it if you need additional features.

use anyhow::anyhow;
use rand::seq::SliceRandom;
use wmill::Windmill;
use serde::Serialize;

fn main() -> anyhow::Result<i32> {
    let wm = Windmill::default()?;

    // Resources
    let inp = serde_json::json!({
      "token": "token",
      "baseUrl": "url"
    });

    wm.set_resource(Some(inp.clone()), "f/tests/delete_me_test_matrix", "matrix")?;
    let out = wm.get_resource_any("f/tests/delete_me_test_matrix")?;
    assert_eq!(inp, out);

    // States
    let inp = serde_json::json!({ "foo": 2 });
    wm.set_state(Some(inp.clone()))?;
    let out = wm.get_state_any()?;
    assert_eq!(inp, out);

    // Variables
    let inp = "Foo".to_owned();
    wm.set_variable(inp.clone(), "f/tests/my_test_var", false)?;
    let out = wm.get_variable_raw("f/tests/my_test_var")?;
    assert_eq!(inp, out);

    Ok(0)
}
                    "#
                .replace(
                    "<CWD>",
                    std::env::current_dir()
                        .expect("Failed to get current directory")
                        .to_str()
                        .expect("Failed to parse current directory"),
                )
                .into(),
                schema: None,
                is_template: None,
                lock: None,
                language: windmill_api::models::ScriptLang::Rust,
                kind: Some(windmill_api::models::new_script::Kind::Script),
                tag: Some("rust".into()),
                draft_only: None,
                envs: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                ws_error_handler_muted: None,
                priority: None,
                restart_unless_cancelled: None,
                timeout: None,
                delete_after_use: None,
                deployment_message: None,
                concurrency_key: None,
                visible_to_runner_only: None,
                no_main_func: None,
                codebase: None,
                has_preprocessor: None,
                on_behalf_of_email: None,
                assets: None,
            },
        ))
        .unwrap();

    assert_eq!(
        json!(0),
        wm.run_script_sync(
            //
            &resp,
            true,
            json!(null),
            None,
            None,
            true,
            false,
        )
        .unwrap()
    );
}

// ASYNC TESTS
#[cfg(feature = "async")]
#[tokio::test]
async fn simple() {
    let wm = wm();
    let path = format!("f/tests/delete_me_{}", Uuid::new_v4());

    // Create new
    wm.set_variable(
        ":0".into(),
        &path,
        // TODO: Test with true
        false,
    )
    .await
    .unwrap();

    let resp = wm
        .call_api(apis::script_api::create_script(
            &wm.client_config,
            &wm.workspace,
            windmill_api::models::NewScript {
                path: path.clone(),
                parent_hash: None,
                summary: "auto-generated script from rust sdk | testing".into(),
                description: "auto-generated script from rust sdk | testing".into(),
                content: r#"fn main() -> Result<String, String> { Ok("Hello World!".to_owned()) }"#
                    .into(),
                schema: None,
                is_template: None,
                lock: None,
                language: windmill_api::models::ScriptLang::Rust,
                kind: Some(windmill_api::models::new_script::Kind::Script),
                tag: Some("rust".into()),
                draft_only: None,
                envs: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                ws_error_handler_muted: None,
                priority: None,
                restart_unless_cancelled: None,
                timeout: None,
                delete_after_use: None,
                deployment_message: None,
                concurrency_key: None,
                visible_to_runner_only: None,
                no_main_func: None,
                codebase: None,
                has_preprocessor: None,
                on_behalf_of_email: None,
                assets: None,
            },
        ))
        .await
        .unwrap();

    assert_eq!(
        json!("Hello World!"),
        wm.run_script_sync(
            //
            &resp,
            true,
            json!(null),
            None,
            None,
            true,
            false,
        )
        .await
        .unwrap()
    );
}
