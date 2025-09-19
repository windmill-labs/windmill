// TODO: move all related logic here (if anything left anywhere in codebase)
mod common;

mod relative_imports_languages {
    use sqlx::{Pool, Postgres};
    use windmill_common::scripts::ScriptLang;

    use crate::common::{run_deployed_relative_imports, run_preview_relative_imports};

    #[sqlx::test(fixtures("base", "relative_bun"))]
    async fn test_relative_imports_bun(db: Pool<Postgres>) -> anyhow::Result<()> {
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

        run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await?;
        run_preview_relative_imports(&db, content, ScriptLang::Bun).await?;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "relative_bun"))]
    async fn test_nested_imports_bun(db: Pool<Postgres>) -> anyhow::Result<()> {
        let content = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export async function main() {
  return test();
}
"#
        .to_string();

        run_deployed_relative_imports(&db, content.clone(), ScriptLang::Bun).await?;
        run_preview_relative_imports(&db, content, ScriptLang::Bun).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "relative_deno"))]
    async fn test_relative_imports_deno(db: Pool<Postgres>) -> anyhow::Result<()> {
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

        run_deployed_relative_imports(&db, content.clone(), ScriptLang::Deno).await?;
        run_preview_relative_imports(&db, content, ScriptLang::Deno).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "relative_deno"))]
    async fn test_nested_imports_deno(db: Pool<Postgres>) -> anyhow::Result<()> {
        let content = r#"
import { main as test } from "/f/system_relative/nested_script.ts";

export async function main() {
  return test();
}
"#
        .to_string();

        run_deployed_relative_imports(&db, content.clone(), ScriptLang::Deno).await?;
        run_preview_relative_imports(&db, content, ScriptLang::Deno).await?;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "relative_python"))]
    async fn test_relative_imports_python(db: Pool<Postgres>) -> anyhow::Result<()> {
        let content = r#"
from f.system.same_folder_script import main as test1
from .same_folder_script import main as test2
from f.system_relative.different_folder_script import main as test3
from ..system_relative.different_folder_script import main as test4
    
def main():
    return [test1(), test2(), test3(), test4()]
"#
        .to_string();

        run_deployed_relative_imports(&db, content.clone(), ScriptLang::Python3).await?;
        run_preview_relative_imports(&db, content, ScriptLang::Python3).await?;
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "relative_python"))]
    async fn test_nested_imports_python(db: Pool<Postgres>) -> anyhow::Result<()> {
        let content = r#"

from f.system_relative.nested_script import main as test

def main():
    return test()
"#
        .to_string();

        run_deployed_relative_imports(&db, content.clone(), ScriptLang::Python3).await?;
        run_preview_relative_imports(&db, content, ScriptLang::Python3).await?;
        Ok(())
    }
}
mod dependency_map {
    use sqlx::{Pool, Postgres};
    use tokio_stream::StreamExt;
    use windmill_api_client::types::NewScript;
    use windmill_common::{jobs::JobPayload, scripts::ScriptHash};

    use crate::common::{
        completed_job, in_test_worker, listen_for_completed_jobs, ApiServer, RunJob,
    };

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

    async fn rebuild_dmap(client: &windmill_api_client::Client) -> bool {
        client
            .client()
            .post(format!(
                "{}/w/test-workspace/workspaces/rebuild_dependency_map",
                client.baseurl()
            ))
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    }

    async fn init(db: Pool<Postgres>) -> (windmill_api_client::Client, u16, ApiServer) {
        initialize_tracing().await;
        let server = ApiServer::start(db).await.unwrap();
        let port = server.addr.port();
        let client = windmill_api_client::create_client(
            &format!("http://localhost:{port}"),
            "SECRET_TOKEN".to_string(),
        );
        (client, port, server)
    }

    async fn clear_dmap(db: &Pool<Postgres>) {
        sqlx::query!("DELETE FROM dependency_map WHERE workspace_id = 'test-workspace'")
            .execute(db)
            .await
            .unwrap();
    }

    async fn assert_dmap(
        db: &Pool<Postgres>,
        importer: Option<String>,
        expected: Vec<(&str, &str, &str, &str)>,
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
                .map(|(f, s, t, fo)| (f.to_owned(), s.to_owned(), t.to_owned(), fo.to_owned()))
                .collect::<Vec<(String, String, String, String)>>()
        );
    }

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
        let (client, port, _s) = init(db.clone()).await;
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        // rebuild map
        assert!(rebuild_dmap(&client).await);
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        Ok(())
    }

    // Consider simple one. Only referenced directly. No deep connections
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_redeploy_leaf_2(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init(db.clone()).await;
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
def main():
    return 'leaf2';
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/leaf_2",
                    None,
                    Some("0000000000051659".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        // rebuild map
        assert!(rebuild_dmap(&client).await);
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        Ok(())
    }

    // Consider hard one. Referenced deeply and exists in double references.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_redeploy_leaf_1(db: Pool<Postgres>) -> anyhow::Result<()> {
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
                    "f/rel/leaf_1",
                    None,
                    Some("000000000005165A".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        // rebuild map
        assert!(rebuild_dmap(&client).await);
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        Ok(())
    }

    // If you deploy from cli and you use raw requirements you don't want the script be included in dmap
    // Otherwise script will be overwritten once any relative import is updated
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_with_requirements_txt(db: Pool<Postgres>) -> anyhow::Result<()> {
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

        assert_dmap(&db, Some("f/rel/root_script".into()), vec![]).await;

        // rebuild map
        assert!(rebuild_dmap(&client).await);

        assert_dmap(&db, Some("f/rel/root_script".into()), vec![]).await;

        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_without_requirements_txt(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
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
        // rebuild map
        assert!(rebuild_dmap(&client).await);
        assert_dmap(&db, None, CORRECT_DMAP.clone()).await;
        Ok(())
    }
    // TODO: renames.

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_script(db: Pool<Postgres>) -> anyhow::Result<()> {
        todo!()
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
        todo!()
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rename_app(db: Pool<Postgres>) -> anyhow::Result<()> {
        todo!()
    }
}
