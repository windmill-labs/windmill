// TODO: move all related logic here (if anything left anywhere in codebase)
mod common;

use windmill_api_client::types::NewScript;

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

mod dependency_map {
    use super::quick_ns;
    use sqlx::{Pool, Postgres};
    use tokio_stream::StreamExt;

    use crate::common::{in_test_worker, init_client, listen_for_completed_jobs, ApiServer};

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
        let dmap = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT importer_path, importer_kind::text, imported_path, importer_node_id FROM dependency_map WHERE workspace_id = 'test-workspace' AND ($1::text IS NULL OR importer_path = $1::text)",
        )
        .bind(importer)
        .fetch_all(db)
        .await
        .unwrap();

        assert_eq!(
            dmap,
            expected
                .into_iter()
                .map(|(f, s, t, fo)| (f.into(), s.into(), t.into(), fo.into()))
                .collect::<Vec<(String, String, String, String)>>()
        );
    }

    lazy_static::lazy_static! {
    pub static ref CORRECT_DMAP: Vec<(&'static str, &'static str, &'static str, &'static str)> =  vec![
        ("f/rel/branch", "script", "f/rel/leaf_1", ""),
        ("f/rel/root_script", "script", "f/rel/branch", ""),
        ("f/rel/root_script", "script", "f/rel/leaf_1", ""),
        ("f/rel/root_script", "script", "f/rel/leaf_2", ""),
        ("f/rel/root_app", "app", "f/rel/leaf_2", "dontpressmeplz"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "nstep1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_1", "nstep1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "nstep1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "nstep2_2"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "nstep4_1"),
        ("f/rel/root_flow", "flow", "f/rel/branch", "nstep5_1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_1", "nstep5_1"),
        ("f/rel/root_flow", "flow", "f/rel/leaf_2", "nstep5_1"),
        ("f/rel/root_app", "app", "f/rel/branch", "pressmeplz"),
        ("f/rel/root_app", "app", "f/rel/leaf_1", "pressmeplz"),
        ("f/rel/root_app", "app", "f/rel/leaf_2", "pressmeplz"),
        ("f/rel/root_app", "app", "f/rel/branch", "youcanpressme")];
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rebuild_correctness(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, _port, _s) = init(db.clone()).await;
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        // rebuild map
        assert!(super::common::rebuild_dmap(&client).await);
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
            tokio::spawn(async move { super::common::rebuild_dmap(&client).await })
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
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_with_requirements_txt(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, _port, _s) = init(db.clone()).await;

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.branch import main as br;
from f.rel.leaf_1 import main as lf_1;
from f.rel.leaf_2 import main as lf_2;

def main():
    return [br(), lf_1(), lf_2];
                            ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_script",
                    Some(format!("# from requirements.txt")),
                    Some("000000000005165B".into()),
                ),
            )
            .await
            .unwrap();

        assert_dmap(
            &db,
            Some("f/rel/root_script".into()),
            vec![
                ("f/rel/root_script", "script", "f/rel/branch", ""),
                ("f/rel/root_script", "script", "f/rel/leaf_1", ""),
                ("f/rel/root_script", "script", "f/rel/leaf_2", ""),
            ],
        )
        .await;

        tokio::time::sleep(std::time::Duration::from_secs(13)).await;

        assert_dmap(
            &db,
            Some("f/rel/root_script".into()),
            Vec::<(String, String, String, String)>::new(),
        )
        .await;

        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_without_requirements_txt(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let (client, _port, _s) = init(db.clone()).await;

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.branch import main as br;
from f.rel.leaf_1 import main as lf_1;
from f.rel.leaf_2 import main as lf_2;

def main():
    return [br(), lf_1(), lf_2];
                            ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_script",
                    // We still want to pass lock to it.
                    Some(format!("# py311")),
                    Some("000000000005165B".into()),
                ),
            )
            .await
            .unwrap();
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        tokio::time::sleep(std::time::Duration::from_secs(13)).await;
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        Ok(())
    }
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
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
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
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
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
        let mut corrected_dmap = CORRECT_DMAP.clone();
        // Corresponds to importer path of branch entry
        corrected_dmap[0].0 = "f/rel/branch_renamed";
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
        use windmill_common::{cache::flow::fetch_version, flows::NewFlow};

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
                value: serde_json::from_str(
                    &serde_json::to_string(flow.value())
                        .unwrap()
                        .replace("nstep1", "Foxes")
                        .replace("nstep2_2", "like")
                        .replace("nstep_4_1", "Emeralds"),
                )
                .unwrap(),
                schema: None,
                draft_only: None,
                tag: None,
                dedicated_worker: None,
                timeout: None,
                deployment_message: None,
                visible_to_runner_only: None,
                on_behalf_of_email: None,
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

mod dependency_job_debouncing {
    use crate::common::{in_test_worker, initialize_tracing, listen_for_completed_jobs, ApiServer};
    use sqlx::{Pool, Postgres};
    use tokio_stream::StreamExt;
    use windmill_api_client::types::NewScript;

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "djob_debouncing"))]
    async fn test_dependency_job_debouncing_simple(db: Pool<Postgres>) -> anyhow::Result<()> {
        use crate::common::init_client;

        let (client, port, _s) = init_client(db.clone()).await;

        client
            .create_script(
                "test-workspace",
                &NewScript {
                    content: "
def main():
    return 'f/rel/leaf'
                            "
                    .into(),
                    language: windmill_api_client::types::ScriptLang::Python3,
                    parent_hash: Some("0000000000051658".into()),
                    path: "f/rel/leaf".into(),
                    lock: None,
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
                },
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(
            &db,
            async {
                completed.next().await;
                completed.next().await;
                completed.next().await;
            },
            port,
        )
        .await;
        Ok(())
    }

    async fn trigger_djob_for(
        client: &windmill_api_client::Client,
        path: &str,
        parent_hash: &str,
        content: Option<String>,
    ) {
        use super::quick_ns;
        use windmill_api_client::types::ScriptLang;
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    &content.unwrap_or(
                        "
def main():
    pass
                        "
                        .into(),
                    ),
                    ScriptLang::Python3,
                    path,
                    None,
                    Some(parent_hash.into()),
                ),
            )
            .await
            .unwrap();
    }

    /// # Double referenced even
    /// It follows this topology:
    ///
    ///   ┌─FLOW──────────┐
    ///   │┌───┐┌───┐┌───┐│
    ///   ││ A ││ B ││ C ││
    ///   │└─▲─┘▲───▲└─▲─┘│
    ///   └──┼──┼───┼──┼──┘
    ///     ┌┴──┴┐ ┌┴──┴┐
    ///     │L_LF│ │R_LF│
    ///     └────┘ └────┘
    ///
    /// p.s: "LF" stands for "Leaf", "L" - "Left", "R" - "Right"
    ///
    /// With this topology we are going to recreate various scenarios including:
    /// 1. LLF and RLF create two djobs for flow at the same and fall into single debounce
    /// 2. LLF creates flow djob and is single job in debounce, RLF will create another flow djob that will fall into second debounce.
    /// 3. Same as above, however first flow djob will take longer than second debounce.
    /// 4. Fail flow djob. (recovery test)
    /// TODO: 4. CLI usage???
    ///
    /// What test gurantees:
    /// 1. nodes_to_relock (which are stored in debounce_stale_data) are always correct, thus all nodes that need to be relocked are relocked.
    /// 2. flow dependency jobs always check for debounce_stale_data
    /// 3. no djobs run in parallel and no race conditions at any point
    /// 4. no djobs are lost
    /// 5. no new version of flow has been created.
    /// 6. even if something goes wrong system can recover
    mod dre {
        use crate::common::{in_test_worker, init_client, listen_for_completed_jobs};
        use crate::dependency_job_debouncing::trigger_djob_for;
        use std::time::Duration;
        use tokio::time::sleep;
        use tokio_stream::StreamExt;

        /// 1. LLF and RLF create two djobs for flow at the same and fall into single debounce
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_1(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            // This tests if debouncing and consolidation works.
            // Also makes sures that dependency job does not create new flow version

            // Neccessery boilerplate
            let (client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // dmap is not built in job_debouncing fixture
            assert!(crate::common::rebuild_dmap(&client).await);

            // Verify locks are empty
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT jsonb_array_elements(value->'modules')->'value'->>'lock' AS lock FROM flow")
                    .fetch_all(&db)
                    .await
                    .unwrap(),
                    vec![
                        Some("# py: 3.11\n".into()),
                        Some("# py: 3.11\n".into()),
                        Some("# py: 3.11\n".into())
                    ]
                );
            }

            // Trigger both at the same time.
            {
                trigger_djob_for(
                    &client,
                    "f/dre/leaf_left",
                    "0000000000051658",
                    Some("#requirements:\n#bottle==0.13.2\ndef main():\npass".into()),
                )
                .await;

                trigger_djob_for(
                    &client,
                    "f/dre/leaf_right",
                    "000000000005165B",
                    Some("#requirements:\n#tiny==0.1.3\ndef main():\npass".into()),
                )
                .await;
            }

            // This should create one job for left leaf and one for the right leaf
            // The first job that finishes will create debounce and consume all djobs withing 5s
            // This will block until both of them are done.
            in_test_worker(
                &db,
                async {
                    // Wait until both djobs are done.
                    completed.next().await;
                    completed.next().await;
                },
                port,
            )
            .await;

            // Verify there is only one queued job that is scheduled for atleast 3s ahead.
            {
                let q = sqlx::query_scalar!(
                "SELECT (q.scheduled_for > (now() + INTERVAL '3 seconds'))::boolean FROM v2_job_queue q"
                )
                .fetch_all(&db)
                .await
                .unwrap();

                assert_eq!(vec![Some(true)], q);
            }

            // Verify debounce_stale_data and debounce_key
            {
                let q = sqlx::query!(
                    "SELECT
                        dsd.to_relock,
                        dk.key
                    FROM debounce_key dk
                    JOIN debounce_stale_data dsd ON dk.job_id = dsd.job_id"
                )
                .fetch_all(&db)
                .await
                .unwrap();

                // Should be single entry
                assert!(q.len() == 1);

                // This verifies that all nodes_to_relock are consolidated correctly
                // AND there is no doublicats
                assert_eq!(
                    q[0].to_relock.clone().unwrap(),
                    vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
                );

                // Should be workspace specific and these specific tests cover only dependency job debouncing
                assert_eq!(q[0].key.clone(), "admins:f/dre/flow:dependency".to_owned(),);
            }

            // Wait until debounce delay is complete
            sleep(Duration::from_secs(6)).await;

            in_test_worker(
                &db,
                async {
                    // Wait for the last debounce on flow
                    completed.next().await;
                },
                port,
            )
            .await;

            // Verify latest flow.version property
            {
                // Latest flow version should not be initial one
                assert_eq!(
                    1, // Automatically assigned
                    dbg!(sqlx::query_scalar!(
                    //                                     _< First
                    "SELECT versions[array_upper(versions, 1)] FROM flow WHERE path = 'f/dre/flow'"
                )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap())
                );

                // Only second element should be our initial version
                assert_eq!(
                    1443253234253454, // < Predefined in fixture
                    dbg!(sqlx::query_scalar!(
                        //                                     _< Second
                        "SELECT versions[1] FROM flow WHERE path = 'f/dre/flow'"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap())
                );
            }

            // Verify that there is only two versions of flow in global flow_version
            {
                assert_eq!(
                    2,
                    sqlx::query_scalar!(
                        "SELECT COUNT(*) FROM flow_version WHERE path = 'f/dre/flow'"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap()
                );
            }

            // Verify locks
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT jsonb_array_elements(value->'modules')->'value'->>'lock' AS lock FROM flow")
                    .fetch_all(&db)
                    .await
                    .unwrap(),
                    vec![
                        Some("# py: 3.11\nbottle==0.13.2".into()),
                        Some("# py: 3.11\nbottle==0.13.2\ntiny==0.1.3".into()),
                        Some("# py: 3.11\ntiny==0.1.3".into())
                    ]
                );
            }

            // TODO:
            // tracing_assertions::assert_has_events!([info("This is supposed to be called")]);
            // 2025-10-06T14:31:10.832469Z  WARN windmill-worker/src/worker.rs:1593: pull took more than 0.1s (0.222477345) this is a sign that the database is undersized for this load. empty: true, err: true worker=wk-default-nixos-EzDEL hostname=nixos

            // Verify cleanup
            {
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_stale_data")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
            }

            Ok(())
        }

        /// 2. LLF creates flow djob and is single job in debounce, RLF will create another flow djob that will fall into second debounce.
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_2(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            // This test mostly verifies if following djobs after first debounce do work.
            Ok(())
        }

        /// 3. Same as second test, however first flow djob will take longer than second debounce.
        /// NOTE: This test should be ran in debug mode. In release it will not work properly.
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_3(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            // This tests checks if concurrency limit works correcly and there is no race conditions.
            let (client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // dmap is not built in job_debouncing fixture
            assert!(crate::common::rebuild_dmap(&client).await);

            // At this point we should have two
            let mut job_ids = vec![];
            let push_job = |delay, db| async move {
                let mut args = std::collections::HashMap::new();
                args.insert(
                    "dbg_djob_sleep".to_owned(),
                    // First one will create delay for 5 seconds
                    // The second will have no delay at all.
                    windmill_common::worker::to_raw_value(&delay),
                );

                let (job_uuid, new_tx) = windmill_queue::push(
                    &db,
                    windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                    "test-workspace",
                    windmill_common::jobs::JobPayload::FlowDependencies {
                        path: "f/dre/flow".to_owned(),
                        dedicated_worker: None,
                        // In newest versions we pass the current version to the djob
                        version: 1443253234253454,
                    },
                    windmill_queue::PushArgs { args: &args, extra: None },
                    "admin",
                    "admin@windmill.dev",
                    "admin".to_owned(),
                    Some("trigger.dependents.to.recompute.dependencies"),
                    // Schedule for now.
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    true,
                    Some("dependency".into()),
                    None,
                    None,
                    None,
                    None,
                    false,
                )
                .await
                .unwrap();

                new_tx.commit().await.unwrap();

                job_uuid
            };

            // Push first
            job_ids.push(push_job(5, db.clone()).await);

            // Start the first one in the background
            let handle = {
                let mut completed = listen_for_completed_jobs(&db).await;
                let db2 = db.clone();
                tokio::spawn(async move {
                    in_test_worker(
                        &db2,
                        // sleep(Duration::from_secs(7)),
                        completed.next(), // Only wait for the single job. We are going to spawn another worker for second one.
                        port,
                    )
                    .await;
                })
            };

            // Wait for the job to be created and started
            // This way next job is not going to be consumed by the first one.
            sleep(Duration::from_secs(2)).await;

            // Push second
            job_ids.push(push_job(0, db.clone()).await);
            // Wait for the second one to finish in separate worker.
            // in_test_worker(&db, completed.next(), port).await;
            in_test_worker(
                &db,
                async {
                    // First job will be pulled
                    completed.next().await;
                    // However since we have concurrency limit enabled it will get rescheduled by creation of new djob.
                    // So we have to wait for that one as well.
                    completed.next().await;
                },
                port,
            )
            .await;

            // Wait for the first one
            handle.await.unwrap();

            // Verify that we have expected outcome
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job",)
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                    2
                );

                assert_eq!(
                    sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_completed",)
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                    2
                );
                // Check that two jobs were executed sequentially
                assert!(sqlx::query_scalar!(
                    "
SELECT
    j1.completed_at < j2.started_at
FROM
    v2_job_completed j1,
    v2_job_completed j2
WHERE
    j1.id = $1  
    AND j2.id = $2",
                    job_ids[0],
                    job_ids[1],
                )
                .fetch_one(&db)
                .await
                .unwrap()
                .unwrap());
            }
            Ok(())
        }
    }

    // TODO: Same for apps

    // TODO: Test CLI

    //   ┌─FLOW──────────┐
    //   │┌───┐┌───┐┌───┐│
    //   ││ A ││ B ││ C ││
    //   │└─▲─┘▲───▲└─▲─┘│
    //   └──┼──┼───┼──┼──┘
    //     ┌┴──┴┐ ┌┴──┴┐
    //     │L_BR│ │R_BR│
    //     └─▲──┘ └──▲─┘
    //     ┌─┴──┐    │
    //     │LEAF├────┘
    //     └────┘
    //

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "djob_debouncing"))]
    async fn test_dependency_job_debouncing_high_latency_and_loaded(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "djob_debouncing"))]
    async fn test_dependency_job_debouncing_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
        // 1. Same double_references

        // 2. Different references at the same time
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn test_dependency_job_debouncing_app(db: Pool<Postgres>) -> anyhow::Result<()> {
        Ok(())
    }

    // TODO: Already visited usage.
    // TODO: Test git sync
    // TODO: Can we find timing when we update the job but at the same time it is being pulled for execution?
}
