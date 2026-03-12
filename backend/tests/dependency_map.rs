use sqlx::{Pool, Postgres};
use tokio_stream::StreamExt;

use windmill_api_client::types::NewScript;
use windmill_test_utils::{
    in_test_worker, init_client, listen_for_completed_jobs, rebuild_dmap, ApiServer,
};

mod dependency_map {
    use super::*;
    fn quick_ns(
        content: &str,
        language: windmill_api_client::types::ScriptLang,
        path: &str,
        lock: Option<String>,
        parent_hash: Option<String>,
    ) -> NewScript {
        NewScript {
            content: content.into(),
            language,
            lock,
            parent_hash,
            path: path.into(),
            concurrent_limit: None,
            concurrency_time_window_s: None,
            cache_ttl: None,
            dedicated_worker: None,
            description: "".to_string(),
            draft_only: None,
            envs: vec![],
            is_template: None,
            kind: None,
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
        }
    }
    async fn init(db: Pool<Postgres>) -> (windmill_api_client::Client, u16, ApiServer) {
        init_client(db).await
    }

    async fn _clear_dmap(db: &Pool<Postgres>) {
        sqlx::query!("DELETE FROM dependency_map WHERE workspace_id = 'test-workspace'")
            .execute(db)
            .await
            .unwrap();
    }

    /// Corrects map according to provided replacements.
    /// Only changes importer_path and/or id
    /// Does not affect imported_path nor kind!
    fn corrected_dmap(replacements: Vec<(&str, &str)>) -> Vec<(String, String, String, String)> {
        CORRECT_DMAP
            .clone()
            .into_iter()
            .map(|e| {
                let mut r = (
                    e.0.to_owned(),
                    e.1.to_owned(),
                    e.2.to_owned(),
                    e.3.to_owned(),
                );
                for (from, to) in &replacements {
                    r = (
                        r.0.replace(from, to),
                        r.1, // Kind should be immutable
                        r.2, // Imported path should be immutable
                        // We do not modify script contents in test, so we can assume scripts always import the same path
                        // Modification of kind or imported path considered to be incorrect.
                        r.3.replace(from, to),
                    );
                }
                r
            })
            .collect()
    }

    async fn assert_dmap(
        db: &Pool<Postgres>,
        importer: Option<String>,
        expected: Vec<(
            impl Into<String>,
            impl Into<String>,
            impl Into<String>,
            impl Into<String>,
        )>,
    ) {
        let mut dmap = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT importer_path, importer_kind::text, imported_path, importer_node_id FROM dependency_map WHERE workspace_id = 'test-workspace' AND ($1::text IS NULL OR importer_path = $1::text)",
        )
        .bind(importer)
        .fetch_all(db)
        .await
        .unwrap();

        let mut expected = expected
            .into_iter()
            .map(|(f, s, t, fo)| (f.into(), s.into(), t.into(), fo.into()))
            .collect::<Vec<(String, String, String, String)>>();

        dmap.sort();
        expected.sort();

        assert_eq!(dmap, expected);
    }

    lazy_static::lazy_static! {
    pub static ref CORRECT_DMAP: Vec<(&'static str, &'static str, &'static str, &'static str)> =  vec![
        ("f/rel/root_script", "script", "dependencies/test.requirements.in", ""),
        ("f/rel/root_flow", "flow", "dependencies/test.requirements.in", "nstep1"),
        ("f/rel/root_app", "app", "dependencies/test.requirements.in", "dontpressmeplz"),
        ("f/rel/branch", "script", "f/rel/leaf_1", ""),
        ("f/rel/root_script", "script", "f/rel/branch", ""),
        ("f/rel/root_script", "script", "f/rel/leaf_1", ""),
        ("f/rel/root_script", "script", "f/rel/leaf_2", ""),
        ("f/rel/root_app", "app", "f/rel/leaf_2", "dontpressmeplz"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "failure"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "nstep1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_1", "nstep1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "nstep1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "nstep2_2"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "nstep4_1"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "nstep5_1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_1", "nstep5_1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "nstep5_1"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "preprocessor"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_1", "preprocessor"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "preprocessor"),
        ("f/rel/root_app", "app", "f/rel/branch", "pressmeplz"),
        ("f/rel/root_app", "app", "f/rel/leaf_1", "pressmeplz"),
        ("f/rel/root_app", "app", "f/rel/leaf_2", "pressmeplz"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "qtool1"),
        ("f/rel/root_app", "app", "f/rel/branch", "youcanpressme"),

        // Default
        ("f/rel/leaf_1", "script", "dependencies/requirements.in", ""),
        ("f/rel/leaf_2", "script", "dependencies/requirements.in", ""),
        ("f/rel/branch", "script", "dependencies/requirements.in", ""),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "failure"),
        ("f/rel/root_flow", "flow", "dependencies/package.json", "nstep2_1"),
        ("f/rel/root_flow", "flow", "dependencies/package.json", "nstep3_2"),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "nstep2_2"),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "nstep3_1"),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "nstep4_1"),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "nstep5_1"),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "preprocessor"),
        ("f/rel/root_app", "app", "dependencies/requirements.in", "pressmeplz"),
        ("f/rel/root_flow", "flow", "dependencies/requirements.in", "qtool1"),
        ("f/rel/root_app", "app", "dependencies/requirements.in", "youcanpressme")
    ];
    }
    // TODO:
    // Test that checks that we can run rebuild_dmap multiple times in tests.

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rebuild_correctness(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, _port, _s) = init(db.clone()).await;
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        // rebuild map
        assert!(rebuild_dmap(&client).await);
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rebuild_lock(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, _port, _s) = init(db.clone()).await;

        // Spawn first rebuild
        let handle = {
            let client = client.clone();
            tokio::spawn(async move { rebuild_dmap(&client).await })
        };

        // Immidiately spawn another
        let res = client
            .client()
            .post(format!(
                "{}/w/test-workspace/workspaces/rebuild_dependency_map",
                client.baseurl()
            ))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        // Should tell us there is already rebuilt in progress
        // Or if it is too fast we will be able to trigger it second time
        assert!(&res == "There is already one task pending, try again later." || &res == "Success");

        assert!(handle.await.unwrap());
        Ok(())
    }

    // If you deploy from cli and you use raw requirements you don't want the script be included in dmap
    // Otherwise script will be overwritten once any relative import is updated
    //     #[cfg(feature = "python")]
    //     #[sqlx::test(fixtures("base", "dependency_map"))]
    //     async fn relative_imports_test_with_legacy(db: Pool<Postgres>) -> anyhow::Result<()> {
    //         let (client, _port, _s) = init(db.clone()).await;

    //         client
    //             .create_script(
    //                 "test-workspace",
    //                 &quick_ns(
    //                     "
    // from f.rel.branch import main as br;
    // from f.rel.leaf_1 import main as lf_1;
    // from f.rel.leaf_2 import main as lf_2;

    // def main():
    //     return [br(), lf_1(), lf_2];
    //                             ",
    //                     windmill_api_client::types::ScriptLang::Python3,
    //                     "f/rel/root_script",
    //                     Some("# from requirements.txt".to_string()),
    //                     Some("000000000005165B".into()),
    //                 ),
    //             )
    //             .await
    //             .unwrap();

    //         assert_dmap(
    //             &db,
    //             Some("f/rel/root_script".into()),
    //             vec![
    //                 ("f/rel/root_script", "script", "f/rel/branch", ""),
    //                 ("f/rel/root_script", "script", "f/rel/leaf_1", ""),
    //                 ("f/rel/root_script", "script", "f/rel/leaf_2", ""),
    //             ],
    //         )
    //         .await;

    //         tokio::time::sleep(std::time::Duration::from_secs(13)).await;

    //         assert_dmap(
    //             &db,
    //             Some("f/rel/root_script".into()),
    //             vec![
    //                 ("f/rel/root_script", "script", "f/rel/branch", ""),
    //                 ("f/rel/root_script", "script", "f/rel/leaf_1", ""),
    //                 ("f/rel/root_script", "script", "f/rel/leaf_2", ""),
    //             ],
    //         )
    //         .await;

    //         Ok(())
    //     }

    // Consider simple one. Only referenced directly. No deep connections
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_leaf_2(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init(db.clone()).await;
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
def main():
    return 'leaf3';
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/leaf_2_renamed",
                    None,
                    Some("0000000000051659".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        // Changing leafs should not change dependency map
        assert_dmap(
            &db,
            None,
            corrected_dmap(vec![("f/rel/leaf_2", "f/rel/leaf_2_renamed")]),
        )
        .await;
        Ok(())
    }

    // Consider hard one. Referenced deeply and exists in double references.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_leaf_1(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init(db.clone()).await;
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
def main():
    return 'leaf1';
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/leaf_1_renamed",
                    None,
                    Some("0000000000051658".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        // Changing leafs should not change dependency map
        assert_dmap(
            &db,
            None,
            corrected_dmap(vec![("f/rel/leaf_1", "f/rel/leaf_1_renamed")]),
        )
        .await;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_branch(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init(db.clone()).await;
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.leaf_1 import main as lf_1;

def main():
    return lf_1();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/branch_renamed",
                    None,
                    Some("000000000005165A".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        // Changing branches SHOULD change dependency map
        // Though it should only change branch item in dmap when it is importer.
        // All entries when branch is imported should not change.
        let corrected_dmap = CORRECT_DMAP
            .clone()
            .iter_mut()
            .map(|el| {
                if el.0 == "f/rel/branch" {
                    el.0 = "f/rel/branch_renamed";
                }
                *el
            })
            .collect::<Vec<_>>();
        assert_dmap(&db, None, corrected_dmap).await;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_primary_script(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init(db.clone()).await;

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
# requirements: test
from f.rel.branch import main as br;
from f.rel.leaf_1 import main as lf_1;
from f.rel.leaf_2 import main as lf_2;

def main():
    return [br(), lf_1(), lf_2];
                            ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_script_renamed",
                    None,
                    Some("000000000005165B".into()),
                ),
            )
            .await
            .unwrap();

        let corrected_dmap = corrected_dmap(vec![("root_script", "root_script_renamed")]);
        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;
        assert_dmap(&db, None, corrected_dmap.clone()).await;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_primary_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
        use windmill_common::{cache::flow::fetch_version, flows::NewFlow, worker::to_raw_value};

        let (client, port, _s) = init(db.clone()).await;
        let flow = fetch_version(&db, 1443253234253454).await.unwrap();
        let res = client
            .client()
            .post(format!(
                "{}/w/test-workspace/flows/update/{}",
                client.baseurl(),
                "f/rel/root_flow" // encode_path()
            ))
            .json(&NewFlow {
                path: "f/rel/root_flow_renamed".into(),
                summary: "".into(),
                description: None,
                value: to_raw_value(
                    &serde_json::from_str::<serde_json::Value>(
                        &serde_json::to_string(flow.value())
                            .unwrap()
                            .replace("nstep1", "Foxes")
                            .replace("nstep2_2", "like")
                            .replace("nstep_4_1", "Emeralds"),
                    )
                    .unwrap(),
                ),
                schema: None,
                draft_only: None,
                tag: None,
                dedicated_worker: None,
                timeout: None,
                deployment_message: None,
                visible_to_runner_only: None,
                on_behalf_of_email: None,
                preserve_on_behalf_of: None,
                ws_error_handler_muted: None,
            })
            .send()
            .await
            .unwrap();

        assert_eq!(res.text().await.unwrap(), "f/rel/root_flow_renamed");

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        assert_dmap(
            &db,
            None,
            corrected_dmap(vec![
                ("f/rel/root_flow", "f/rel/root_flow_renamed"),
                ("nstep1", "Foxes"),
                ("nstep2_2", "like"),
                ("nstep_4_1", "Emeralds"),
            ]),
        )
        .await;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_primary_app(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init(db.clone()).await;

        let app_value: String =
            sqlx::query_scalar!("SELECT value::text FROM app_version WHERE id = 0 AND app_id = 2")
                .fetch_one(&db)
                .await
                .unwrap()
                .unwrap();

        // TODO: There is:
        // 1. update app
        // 2. create app
        // 3. update app raw
        // Ideally all of them should be handled
        let res = client
            .client()
            .post(format!(
                "{}/w/test-workspace/apps/update/{}",
                client.baseurl(),
                "f/rel/root_app" // encode_path()
            ))
            .json(&windmill_api::EditApp {
                path: Some("f/rel/root_app_renamed".into()),
                summary: None,
                value: serde_json::from_str(
                    &app_value
                        .replace("dontpressmeplz", "Apps")
                        .replace("youcanpressme", "Work"),
                )
                .unwrap(),
                policy: None,
                deployment_message: None,
                custom_path: None,
                preserve_on_behalf_of: None,
            })
            .send()
            .await
            .unwrap();

        assert_eq!(
            res.text().await.unwrap(),
            "app f/rel/root_app updated (npath: \"f/rel/root_app_renamed\")"
        );

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        assert_dmap(
            &db,
            None,
            corrected_dmap(vec![
                ("f/rel/root_app", "f/rel/root_app_renamed"),
                ("dontpressmeplz", "Apps"),
                ("youcanpressme", "Work"),
            ]),
        )
        .await;
        Ok(())
    }
}
