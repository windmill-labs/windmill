use windmill_test_utils::*;
use sqlx::postgres::Postgres;
use sqlx::Pool;
use windmill_common::scripts::ScriptLang;

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_requirements_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: ==3.11.11
# requirements:
# tiny==0.1.3

import bar
import baz # pin: foo
import baz # repin: fee
import bug # repin: free
    
def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# workspace-dependencies-mode: manual\n# py: 3.11.11",
            "tiny==0.1.3",
        ],
    )
    .await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_extra_requirements_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    {
        use windmill_common::scripts::ScriptLang;

        let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny

import f.system.extra_requirements
import tiny # pin: tiny==0.1.0
import tiny # pin: tiny==0.1.1
import tiny # repin: tiny==0.1.2

def main():
    pass
    "#
        .to_string();

        assert_lockfile(
            &db,
            content,
            ScriptLang::Python3,
            vec![
                "# workspace-dependencies-mode: extra\n# py: 3.11.11",
                "bottle==0.13.2",
                "tiny==0.1.2",
            ],
        )
        .await?;
    }
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_extra_requirements_python2(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny==0.1.3

import simplejson # pin: simplejson==3.20.1
def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# workspace-dependencies-mode: extra\n# py: 3.11.11",
            "simplejson==3.20.1",
            "tiny==0.1.3",
        ],
    )
    .await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "lockfile_python"))]
async fn test_pins_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: ==3.11.11
# extra_requirements:
# tiny==0.1.3
# bottle==0.13.2

import f.system.requirements
import f.system.pins
import tiny # repin: tiny==0.1.3
import simplejson

def main():
    pass
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec![
            "# workspace-dependencies-mode: extra\n# py: 3.11.11",
            "bottle==0.13.2",
            "microdot==2.2.0",
            "simplejson==3.19.3",
            "tiny==0.1.3",
        ],
    )
    .await?;
    Ok(())
}
#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "multipython"))]
async fn test_multipython_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py: <=3.12.2, >=3.12.0
import f.multipython.script1
import f.multipython.aliases
"#
    .to_string();

    assert_lockfile(&db, content, ScriptLang::Python3, vec!["# py: 3.12.1\n"]).await?;
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base", "multipython"))]
async fn test_inline_script_metadata_python(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = r#"# py_select_latest
# /// script
# requires-python = ">3.11,<3.12.3,!=3.12.2"
# dependencies = [
#   "tiny==0.1.3",
# ]
# ///
"#
    .to_string();

    assert_lockfile(
        &db,
        content,
        ScriptLang::Python3,
        vec!["# py: 3.12.1", "tiny==0.1.3"],
    )
    .await?;
    Ok(())
}

#[cfg(feature = "python")]
use windmill_common::jobs::JobPayload;
#[cfg(feature = "python")]
use windmill_common::jobs::RawCode;

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
def main():
    return "hello world"
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("hello world"));
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_global_site_packages(db: Pool<Postgres>) -> anyhow::Result<()> {
    use windmill_common::{cache::concatcp, worker::ROOT_CACHE_DIR};

    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Shared for all 3.12.*
    let path = concatcp!(ROOT_CACHE_DIR, "python_3_12/global-site-packages").to_owned();
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path + "/my_global_site_package_3_12_any.py", "").unwrap();

    // 3.12
    {
        let content = r#"# py: ==3.12
#requirements:
#

import my_global_site_package_3_12_any

def main():
    return "hello world"
                "#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Python3,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!("hello world"));
    }

    // 3.12.1
    {
        let content = r#"# py: ==3.12.1
#requirements:
#

import my_global_site_package_3_12_any

def main():
    return "hello world"
                "#
        .to_owned();

        let job = JobPayload::Code(RawCode {
            hash: None,
            content,
            path: None,
            language: ScriptLang::Python3,
            lock: None,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
        });

        let result = run_job_in_new_worker_until_complete(&db, false, job, port)
            .await
            .json_result()
            .unwrap();

        assert_eq!(result, serde_json::json!("hello world"));
    }
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_heavy_dep(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
import numpy as np

def main():
    a = np.arange(15).reshape(3, 5)
    return len(a)
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!(3));
    Ok(())
}

#[cfg(feature = "python")]
#[sqlx::test(fixtures("base"))]
async fn test_python_job_with_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let content = r#"
import wmill

def main():
    return wmill.get_workspace()
        "#
    .to_owned();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content,
        path: None,
        language: ScriptLang::Python3,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default().into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port)
        .await
        .json_result()
        .unwrap();

    assert_eq!(result, serde_json::json!("test-workspace"));
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
