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

    async fn init(db: Pool<Postgres>) -> (windmill_api_client::Client, u16, ApiServer) {
        // initialize_tracing().await;
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

    async fn assert_dmap(db: &Pool<Postgres>, expected: Vec<(String, String, String, String)>) {
        let dmap = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT importer_path, importer_kind::text, imported_path, importer_node_id FROM dependency_map WHERE workspace_id = 'test-workspace'",
        )
        .fetch_all(db)
        .await
        .unwrap();

        assert_eq!(dmap, expected);
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

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rebuild_dmap_no_double_referencing_deep(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        clear_dmap(&db).await;

        let (client, port, _) = init(db.clone()).await;

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.branch import main as br;
from f.rel.leaf_2 import main as lf_2;

def main():
    return [br(), lf_2];
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/system/root_script",
                    None,
                    Some("000000000005165B".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        assert_dmap(
            &db,
            vec![
                (
                    "f/system/root_script".into(),
                    "script".into(),
                    "f/rel/branch".into(),
                    "".into(),
                ),
                (
                    "f/system/root_script".into(),
                    "script".into(),
                    "f/rel/leaf_1".into(),
                    "".into(),
                ),
                (
                    "f/system/root_script".into(),
                    "script".into(),
                    "f/rel/leaf_2".into(),
                    "".into(),
                ),
            ],
        )
        .await;

        Ok(())
    }

    // TODO: Needs to be limited to 1 for this to work
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_rebuild_dmap_double_referencing_deep(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let (client, port, _) = init(db.clone()).await;

        clear_dmap(&db).await;

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
                    "f/system/root_script",
                    None,
                    Some("000000000005165B".into()),
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(
            &db,
            async {
                completed.next().await;
                // completed.next().await;
                // completed.next().await;
            },
            port,
        )
        .await;

        assert_dmap(
            &db,
            vec![
                (
                    "f/system/root_script".into(),
                    "script".into(),
                    "f/rel/branch".into(),
                    "".into(),
                ),
                (
                    "f/system/root_script".into(),
                    "script".into(),
                    "f/rel/leaf_1".into(),
                    "".into(),
                ),
                (
                    "f/system/root_script".into(),
                    "script".into(),
                    "f/rel/leaf_2".into(),
                    "".into(),
                ),
            ],
        )
        .await;

        Ok(())
    }
    // TEST:
    // 1. Simple relative dependency - check if dmap is even being created, and then check if changing it creates new hash and new lock. Check when leaf changed imports and lock (separately)
    // 2. Multiple independent simple scripts - check if they can coexist
    // 3. renaming:
    //   - Renaming top-level flow/app/script works
    //   - Renaming just flow/app step
    //   - Renaming flow/app step and top-level
    // 4. selfhealing:
    //   - empty map
    //   - mess up things a bit
    // 5. CLI:
    //   - use requirements.txt (emulate that bug)
    //   - no requirements.txt
    // 6. Script/Flow/App have two imports where first points to branch script, second to leaf script. But branch script also imports leaf script
    // 7. Deep - update only lock, update only content
    // 8. Flow/App multiple imports
    //

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_change_lock_for_imported(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        relative_imports_test_for_imported(
            db,
            r#"# py311
# requirements:
# tiny==0.1.3

def main():
    return "f/rel/leaf_2#rev2"
                    "#,
        )
        .await
    }
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relative_imports_test_no_change_lock_for_imported(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        relative_imports_test_for_imported(
            db,
            r#"# py311
# requirements:
# tiny==0.1.3

def main():
    return "f/rel/leaf_2#rev2"
                    "#,
        )
        .await
    }
    async fn relative_imports_test_for_imported(
        db: Pool<Postgres>,
        new_content: &str,
    ) -> anyhow::Result<()> {
        let (client, port, _server) = init(db.clone()).await;

        let old_hash = sqlx::query_scalar::<_, String>(
                "SELECT hash::text FROM script WHERE workspace_id = 'test-workspace' AND archived = false AND path = 'f/rel/root_script'",
            )
            .fetch_one(&db)
            .await
            .unwrap();

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    new_content,
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/leaf_2",
                    None,
                    Some("0000000000051659".into()), // Value taken from relative_python fixture
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        let (new_lock, new_hash) = sqlx::query_as::<_, (String, String)>(
                "SELECT lock, hash::text FROM script WHERE workspace_id = 'test-workspace' AND archived = false AND path = 'f/rel/root_script'",
            )
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(new_lock, "# py: 3.11\ntiny==0.1.3".to_owned());
        assert_ne!(new_hash, old_hash);
        Ok(())
    }

    // If you deploy from cli and you use raw requirements you don't want the script be included in dmap
    // Otherwise script will be overwritten once any relative import is updated
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "relative_python"))]
    async fn relative_imports_test_with_requirements_txt(db: Pool<Postgres>) -> anyhow::Result<()> {
        clear_dmap(&db).await;
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();
        let client = windmill_api_client::create_client(
            &format!("http://localhost:{port}"),
            "SECRET_TOKEN".to_string(),
        );

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.leaf_2 import main as lf_2;

def main():
    return lf_2();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_2",
                    Some(format!("# from requirements.txt")),
                    None,
                ),
            )
            .await
            .unwrap();

        // Insert fake entry. (We expect it to be clean up in 10 seconds)
        sqlx::query!("INSERT INTO dependency_map(workspace_id, importer_path, importer_kind, imported_path) VALUES ('test-workspace', 'f/rel/root_2', 'script', 'f/rel/leaf_2')").execute(&db).await?;

        assert_dmap(
            &db,
            vec![(
                "f/rel/root_2".into(),
                "script".into(),
                "f/rel/leaf_2".into(),
                "".into(),
            )],
        )
        .await;

        tokio::time::sleep(std::time::Duration::from_secs(13)).await;

        assert_dmap(&db, vec![]).await;
        Ok(())
    }

    // Same as above, but not requirements.txt provided
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "relative_python"))]
    async fn relative_imports_test_3_without_requirements_txt(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        clear_dmap(&db).await;
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();
        let client = windmill_api_client::create_client(
            &format!("http://localhost:{port}"),
            "SECRET_TOKEN".to_string(),
        );

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.leaf_2 import main as lf_2;

def main():
    return lf_2();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_3",
                    None,
                    None,
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        assert_dmap(
            &db,
            vec![(
                "f/rel/root_3".into(),
                "script".into(),
                "f/rel/leaf_2".into(),
                "".into(),
            )],
        )
        .await;

        tokio::time::sleep(std::time::Duration::from_secs(13)).await;

        // Should still be the same
        assert_dmap(
            &db,
            vec![(
                "f/rel/root_3".into(),
                "script".into(),
                "f/rel/leaf_2".into(),
                "".into(),
            )],
        )
        .await;
        Ok(())
    }

    //
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "relative_python"))]
    async fn relative_imports_test_4_deep(db: Pool<Postgres>) -> anyhow::Result<()> {
        clear_dmap(&db).await;
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();
        let client = windmill_api_client::create_client(
            &format!("http://localhost:{port}"),
            "SECRET_TOKEN".to_string(),
        );

        // We already have leaf_1 with known hash
        // So start with branch
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.leaf_1 import main as fwd;

def main():
    return fwd();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/branch_deep",
                    None,
                    None,
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;

        in_test_worker(&db, completed.next(), port).await;

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.branch_deep import main as fwd;

def main():
    return fwd();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_deep",
                    None,
                    None,
                ),
            )
            .await
            .unwrap();

        in_test_worker(&db, completed.next(), port).await;

        // Verify the map is built correctly
        assert_dmap(
            &db,
            vec![
                (
                    "f/rel/branch_deep".into(),
                    "script".into(),
                    "f/rel/leaf_1".into(),
                    "".into(),
                ),
                (
                    "f/rel/root_deep".into(),
                    "script".into(),
                    "f/rel/branch_deep".into(),
                    "".into(),
                ),
            ],
        )
        .await;

        // Now update leaf to see if changes propagated
        // Verify lock is correct
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    r#"# py311
# requirements:
# tiny==0.1.2

def main():
    return "f/rel/leaf_1"
                    "#,
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/leaf_1",
                    None,
                    Some("0000000000051658".into()),
                ),
            )
            .await
            .unwrap();

        in_test_worker(
            &db,
            async {
                completed.next().await;
                completed.next().await;
            },
            port,
        )
        .await;

        let lock = sqlx::query_scalar::<_, String>(
                "SELECT lock FROM script WHERE workspace_id = 'test-workspace' AND archived = false AND path = 'f/rel/root_deep'",
            )
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(lock, "# py: 3.11\ntiny==0.1.2".to_owned());
        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "relative_python"))]
    async fn relative_imports_test_5_deep_without_lockfile_change(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        clear_dmap(&db).await;
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();
        let client = windmill_api_client::create_client(
            &format!("http://localhost:{port}"),
            "SECRET_TOKEN".to_string(),
        );

        // We already have leaf_1 with known hash
        // So start with branch
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.leaf_1 import main as fwd;

def main():
    return fwd();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/branch_deep",
                    None,
                    None,
                ),
            )
            .await
            .unwrap();

        let mut completed = listen_for_completed_jobs(&db).await;

        in_test_worker(&db, completed.next(), port).await;

        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "
from f.rel.branch_deep import main as fwd;

def main():
    return fwd();
                    ",
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/root_deep",
                    None,
                    None,
                ),
            )
            .await
            .unwrap();

        in_test_worker(&db, completed.next(), port).await;

        // Verify the map is built correctly
        assert_dmap(
            &db,
            vec![
                (
                    "f/rel/branch_deep".into(),
                    "script".into(),
                    "f/rel/leaf_1".into(),
                    "".into(),
                ),
                (
                    "f/rel/root_deep".into(),
                    "script".into(),
                    "f/rel/branch_deep".into(),
                    "".into(),
                ),
            ],
        )
        .await;

        // Now update leaf to see if changes propagated
        // Verify lock is correct
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    r#"# py311
# requirements:
# tiny==0.1.2

def main():
    return "f/rel/leaf_1"
                    "#,
                    windmill_api_client::types::ScriptLang::Python3,
                    "f/rel/leaf_1",
                    None,
                    Some("0000000000051658".into()),
                ),
            )
            .await
            .unwrap();

        in_test_worker(
            &db,
            async {
                completed.next().await;
                completed.next().await;
            },
            port,
        )
        .await;

        let lock = sqlx::query_scalar::<_, String>(
                "SELECT lock FROM script WHERE workspace_id = 'test-workspace' AND archived = false AND path = 'f/rel/root_deep'",
            )
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(lock, "# py: 3.11\ntiny==0.1.2".to_owned());
        Ok(())
    }
}
