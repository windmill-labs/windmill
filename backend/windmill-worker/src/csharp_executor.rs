use anyhow::anyhow;
use serde_json::value::RawValue;
use std::{collections::HashMap, io, path::Path, process::Stdio};
use uuid::Uuid;
#[cfg(feature = "csharp")]
use windmill_parser_csharp::parse_csharp_reqs;

use itertools::Itertools;
use tokio::{fs::File, io::AsyncReadExt, process::Command};
use windmill_common::{
    error::{self, Error},
    jobs::QueuedJob,
    utils::calculate_hash,
    worker::{save_cache, write_file},
};
use windmill_queue::{append_logs, CanceledBy};

use crate::{
    common::{
        check_executor_binary_exists, create_args_and_out_file, get_reserved_variables,
        read_result, start_child_process, OccupancyMetrics,
    },
    handle_child::handle_child,
    AuthedClientBackgroundTask, CSHARP_CACHE_DIR, DISABLE_NSJAIL, DISABLE_NUSER, DOTNET_PATH,
    HOME_ENV, NSJAIL_PATH, NUGET_CONFIG, PATH_ENV, TZ_ENV,
};

#[cfg(windows)]
use crate::SYSTEM_ROOT;

const NSJAIL_CONFIG_RUN_CSHARP_CONTENT: &str = include_str!("../nsjail/run.csharp.config.proto");

lazy_static::lazy_static! {
    static ref HOME_DIR: String = std::env::var("HOME").expect("Could not find the HOME environment variable");
}

const CSHARP_OBJECT_STORE_PREFIX: &str = "csharpbin/";

#[cfg(feature = "csharp")]
pub async fn generate_nuget_lockfile(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    check_executor_binary_exists("dotnet", DOTNET_PATH.as_str(), "C#")?;

    if let Some(nuget_config) = NUGET_CONFIG.read().await.clone() {
        write_file(job_dir, "nuget.config", &nuget_config)?;
    }

    let (reqs, lines_to_remove) = parse_csharp_reqs(code);

    gen_cs_proj(code, job_dir, reqs, lines_to_remove)?;

    let mut gen_lockfile_cmd = Command::new(DOTNET_PATH.as_str());
    gen_lockfile_cmd
        .current_dir(job_dir)
        .args(vec!["restore", "--use-lock-file"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let gen_lockfile_process = start_child_process(gen_lockfile_cmd, DOTNET_PATH.as_str()).await?;
    handle_child(
        job_id,
        db,
        mem_peak,
        canceled_by,
        gen_lockfile_process,
        false,
        worker_name,
        w_id,
        "dotnet restore",
        None,
        false,
        &mut Some(occupancy_metrics),
    )
    .await?;

    if let Err(e) = std::fs::remove_file(Path::new(job_dir).join("nuget.config")) {
        if e.kind() != io::ErrorKind::NotFound {
            Err(anyhow!("Error erasing nuget.config: {}", e))?;
        }
    }

    let path_lock = format!("{job_dir}/packages.lock.json");
    let mut file = File::open(path_lock).await?;
    let mut req_content = String::new();
    file.read_to_string(&mut req_content).await?;
    Ok(req_content)
}

#[cfg(not(feature = "csharp"))]
pub async fn generate_nuget_lockfile(
    _job_id: &Uuid,
    _code: &str,
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    _job_dir: &str,
    _db: &sqlx::Pool<sqlx::Postgres>,
    _worker_name: &str,
    _w_id: &str,
    _occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    Err(anyhow!("C# is not available because the feature is not enabled").into())
}


#[cfg(feature = "csharp")]
fn gen_cs_proj(
    code: &str,
    job_dir: &str,
    reqs: Vec<(String, Option<String>)>,
    lines_to_remove: Vec<usize>,
) -> anyhow::Result<()> {
    let code = remove_lines_from_text(code, lines_to_remove);

    let pkgs = reqs
        .into_iter()
        .map(|(pkg, vrsion_o)| {
            let version = vrsion_o
                .map(|v| format!("Version=\"{v}\""))
                .unwrap_or("".to_string());
            format!("    <PackageReference Include=\"{pkg}\" {version}/>")
        })
        .join("\n");

    write_file(
        job_dir,
        "Main.csproj",
        &format!(
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net7.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <StartupObject>WindmillScriptCSharpInternal.Wrapper</StartupObject>
    <RestorePackagesWithLockFile>true</RestorePackagesWithLockFile>
  </PropertyGroup>
  <ItemGroup>
{pkgs}
  </ItemGroup>

</Project>
"#
        ),
    )?;

    write_file(job_dir, "Script.cs", code.as_str())?;

    let sig_meta = windmill_parser_csharp::parse_csharp_sig_meta(code.as_str())?;
    let sig = sig_meta.main_sig;
    let spread = &sig
        .args
        .clone()
        .into_iter()
        .map(|x| format!("parsedArgs.{}", &x.name))
        .join(", ");
    let args_class_body = &sig
        .args
        .into_iter()
        .map(|x| {
            Ok(format!(
                "        public {} {} {{ get; set; }}",
                x.otyp
                    .ok_or(anyhow!("Type not found for argument {}", x.name))?,
                &x.name,
            ))
        })
        .collect::<Result<Vec<String>, anyhow::Error>>()?
        .join("\n");

    let class_name = sig_meta.class_name.unwrap_or("Script".to_string());

    let script_call = match (sig_meta.is_async, sig_meta.returns_void) {
        (true, true) => format!(
            r#"
            {class_name}.Main({spread}).Wait();"#
        ),
        (false, true) => format!(
            r#"
            {class_name}.Main({spread});"#
        ),

        (false, false) => format!(
            r#"
            var result = {class_name}.Main({spread});

            var jsonResult = JsonSerializer.Serialize(result);

            File.WriteAllText("result.json", jsonResult);"#
        ),
        (true, false) => format!(
            r#"
            var result = {class_name}.Main({spread}).Result;

            var jsonResult = JsonSerializer.Serialize(result);

            File.WriteAllText("result.json", jsonResult);"#
        ),
    };

    write_file(
        job_dir,
        "Wrapper.cs",
        &format!(
            r#"using System;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;

namespace WindmillScriptCSharpInternal {{

    struct Args {{
{args_class_body}
    }}

    class Wrapper
    {{
        static void Main(string[] args)
        {{
            using FileStream fs = File.OpenRead("args.json");
            Args parsedArgs = JsonSerializer.Deserialize<Args>(fs);

            File.WriteAllText("result.json", "null");

            {script_call}
        }}
    }}
}}
"#,
        ),
    )?;

    Ok(())
}

#[cfg(feature = "csharp")]
async fn build_cs_proj(
    job_id: &Uuid,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
    worker_name: &str,
    w_id: &str,
    base_internal_url: &str,
    hash: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    if let Some(nuget_config) = NUGET_CONFIG.read().await.clone() {
        write_file(job_dir, "nuget.config", &nuget_config)?;
    }

    let mut build_cs_cmd = Command::new(DOTNET_PATH.as_str());

    build_cs_cmd
        .current_dir(job_dir)
        .env_clear()
        .env("PATH", PATH_ENV.as_str())
        .env("BASE_INTERNAL_URL", base_internal_url)
        .env("HOME", HOME_ENV.as_str())
        .args(vec![
            "publish",
            "--configuration",
            "Release",
            "-o",
            job_dir,
            "--no-self-contained",
            "-p:PublishSingleFile=true",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        build_cs_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());
        build_cs_cmd.env(
            "TMP",
            std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
        );
    }

    let build_cs_process = start_child_process(build_cs_cmd, DOTNET_PATH.as_str()).await?;
    handle_child(
        job_id,
        db,
        mem_peak,
        canceled_by,
        build_cs_process,
        false,
        worker_name,
        w_id,
        "dotnet publish",
        None,
        false,
        &mut Some(occupancy_metrics),
    )
    .await?;
    append_logs(job_id, w_id, "\n\n", db).await;

    if let Err(e) = std::fs::remove_file(Path::new(job_dir).join("nuget.config")) {
        if e.kind() != io::ErrorKind::NotFound {
            Err(anyhow!("Error erasing nuget.config: {}", e))?;
        }
    }

    let bin_path = format!("{}/{hash}", CSHARP_CACHE_DIR);

    match save_cache(
        &bin_path,
        &format!("{CSHARP_OBJECT_STORE_PREFIX}{hash}"),
        &format!("{job_dir}/Main"),
    )
    .await
    {
        Err(e) => {
            let em = format!("could not save {job_dir}/Main to C# cache: {e:?}",);
            tracing::error!(em);
            Ok(em)
        }
        Ok(logs) => Ok(logs),
    }
}

fn remove_lines_from_text(contents: &str, indices_to_remove: Vec<usize>) -> String {
    let mut result = Vec::new();

    for (i, line) in contents.lines().enumerate() {
        if !indices_to_remove.contains(&i) {
            result.push(line);
        }
    }

    result.join("\n")
}

#[cfg(not(feature = "csharp"))]
pub async fn handle_csharp_job(
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    _job: &QueuedJob,
    _db: &sqlx::Pool<sqlx::Postgres>,
    _client: &AuthedClientBackgroundTask,
    _inner_content: &str,
    _job_dir: &str,
    _requirements_o: Option<String>,
    _shared_mount: &str,
    _base_internal_url: &str,
    _worker_name: &str,
    _envs: HashMap<String, String>,
    _occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    Err(anyhow!("C# is not available because the feature is not enabled").into())
}

#[cfg(feature = "csharp")]
pub async fn handle_csharp_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &QueuedJob,
    db: &sqlx::Pool<sqlx::Postgres>,
    client: &AuthedClientBackgroundTask,
    inner_content: &str,
    job_dir: &str,
    requirements_o: Option<String>,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    check_executor_binary_exists("dotnet", DOTNET_PATH.as_str(), "C#")?;

    let hash = calculate_hash(&format!(
        "{}{}",
        inner_content,
        requirements_o
            .as_ref()
            .map(|x| x.to_string())
            .unwrap_or_default()
    ));
    let bin_path = format!("{}/{hash}", CSHARP_CACHE_DIR);
    let remote_path = format!("{CSHARP_OBJECT_STORE_PREFIX}{hash}");

    let (cache, cache_logs) = windmill_common::worker::load_cache(&bin_path, &remote_path).await;

    let cache_logs = if cache {
        let target = format!("{job_dir}/Main");

        #[cfg(unix)]
        let symlink = std::os::unix::fs::symlink(&bin_path, &target);
        #[cfg(windows)]
        let symlink = std::os::windows::fs::symlink_dir(&bin_path, &target);

        symlink.map_err(|e| {
            Error::ExecutionErr(format!(
                "could not copy cached binary from {bin_path} to {job_dir}/main: {e:?}"
            ))
        })?;

        cache_logs
    } else {
        let logs1 = format!("{cache_logs}\n\n--- DOTNET BUILD ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, db).await;

        let (reqs, lines_to_remove) = parse_csharp_reqs(inner_content);
        for req in &reqs {
            append_logs(
                &job.id,
                &job.workspace_id,
                format!(
                    "Requirement detected: {} {}\n",
                    req.0,
                    req.1.as_ref().unwrap_or(&"".to_string())
                ),
                db,
            )
            .await;
        }

        gen_cs_proj(inner_content, job_dir, reqs, lines_to_remove)?;

        if let Some(reqs) = requirements_o {
            if !reqs.is_empty() {
                write_file(job_dir, "packages.lock.json", &reqs)?;
            }
        }

        build_cs_proj(
            &job.id,
            mem_peak,
            canceled_by,
            job_dir,
            db,
            worker_name,
            &job.workspace_id,
            base_internal_url,
            &hash,
            occupancy_metrics,
        )
        .await?
    };

    create_args_and_out_file(client, job, job_dir, db).await?;

    let logs2 = format!("{cache_logs}\n\n--- C# CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, format!("{}\n", logs2), db).await;

    let client = &client.get_authed().await;
    let reserved_variables = get_reserved_variables(job, &client.token, db).await?;

    let child = if !*DISABLE_NSJAIL {
        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_CSHARP_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", CSHARP_CACHE_DIR)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(vec!["--config", "run.config.proto", "--", "/tmp/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str()).await?
    } else {
        let compiled_executable_name = "./Main";
        let mut run_csharp = Command::new(compiled_executable_name);
        run_csharp
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        start_child_process(run_csharp, compiled_executable_name).await?
    };

    handle_child(
        &job.id,
        db,
        mem_peak,
        canceled_by,
        child,
        !*DISABLE_NSJAIL,
        worker_name,
        &job.workspace_id,
        "csharp run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
    )
    .await?;
    read_result(job_dir).await
}
