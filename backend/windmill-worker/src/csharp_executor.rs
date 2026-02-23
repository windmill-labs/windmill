use anyhow::anyhow;
use serde_json::value::RawValue;
#[cfg(feature = "csharp")]
use std::{io, path::Path, process::Stdio};

use std::collections::HashMap;
use uuid::Uuid;
#[cfg(feature = "csharp")]
use windmill_parser_csharp::parse_csharp_reqs;

#[cfg(feature = "csharp")]
use itertools::Itertools;
#[cfg(feature = "csharp")]
use tokio::{fs::File, io::AsyncReadExt, process::Command};
#[cfg(feature = "csharp")]
use windmill_common::{
    utils::calculate_hash,
    worker::write_file,
};

use windmill_common::error::{self, Error};
#[cfg(feature = "csharp")]
use crate::global_cache::save_cache;
#[cfg(feature = "csharp")]
use windmill_queue::append_logs;

use windmill_queue::CanceledBy;

#[cfg(feature = "csharp")]
use crate::{
    common::{
        build_command_with_isolation, check_executor_binary_exists, create_args_and_out_file,
        get_reserved_variables, read_result, start_child_process, DEV_CONF_NSJAIL,
    },
    get_proxy_envs_for_lang,
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry, CSHARP_CACHE_DIR, DISABLE_NUSER, DOTNET_PATH,
    HOME_ENV, NSJAIL_PATH, NUGET_CONFIG, PATH_ENV, TRACING_PROXY_CA_CERT_PATH, TZ_ENV,
};
#[cfg(feature = "csharp")]
use windmill_common::scripts::ScriptLang;

use crate::common::OccupancyMetrics;
use windmill_common::client::AuthedClient;

#[cfg(all(windows, feature = "csharp"))]
use crate::SYSTEM_ROOT;

#[cfg(feature = "csharp")]
const NSJAIL_CONFIG_RUN_CSHARP_CONTENT: &str = include_str!("../nsjail/run.csharp.config.proto");

#[cfg(feature = "csharp")]
#[cfg(windows)]
const DOTNET_ROOT_DEFAULT: &str = "C:\\Program Files\\dotnet";

#[cfg(feature = "csharp")]
#[cfg(unix)]
const DOTNET_ROOT_DEFAULT: &str = "/usr/share/dotnet";

#[cfg(feature = "csharp")]
lazy_static::lazy_static! {
    static ref DOTNET_ROOT: String = std::env::var("DOTNET_ROOT").unwrap_or_else(|_| DOTNET_ROOT_DEFAULT.to_string());
}

#[cfg(feature = "csharp")]
const CSHARP_OBJECT_STORE_PREFIX: &str = const_format::concatcp!(
    std::env::consts::OS,
    "_",
    std::env::consts::ARCH,
    "_csharpbin/"
);

#[cfg(feature = "csharp")]
pub async fn generate_nuget_lockfile(
    job_id: &Uuid,
    code: &str,
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job_dir: &str,
    conn: &Connection,
    worker_name: &str,
    w_id: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    check_executor_binary_exists("dotnet", DOTNET_PATH.as_str(), "C#")?;

    if let Some(nuget_config) = read_ee_registry(
        NUGET_CONFIG.read().await.clone(),
        "nuget config",
        job_id,
        w_id,
        conn,
    )
    .await
    {
        if !nuget_config.trim().is_empty() {
            write_file(job_dir, "nuget.config", &nuget_config)?;
        }
    }

    let (reqs, lines_to_remove) = parse_csharp_reqs(code);

    gen_cs_proj(code, job_dir, reqs, lines_to_remove)?;

    let mut gen_lockfile_cmd = Command::new(DOTNET_PATH.as_str());
    gen_lockfile_cmd
        .current_dir(job_dir)
        .env("DOTNET_CLI_HOME", CSHARP_CACHE_DIR)
        .env("NUGET_PACKAGES", format!("{CSHARP_CACHE_DIR}/nuget"))
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "true")
        .env("DOTNET_NOLOGO", "true")
        .env("MSBUILDDISABLENODEREUSE", "1")
        .env("DOTNET_ROOT", DOTNET_ROOT.as_str())
        .args(vec!["restore", "--use-lock-file"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    gen_lockfile_cmd
        .env("SystemRoot", SYSTEM_ROOT.as_str())
        .env("SystemRoot", SYSTEM_ROOT.as_str())
        .env(
            "TMP",
            std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
        )
        .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
        .env(
            "APPDATA",
            std::env::var("APPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Roaming", HOME_ENV.as_str())),
        )
        .env(
            "ProgramFiles",
            std::env::var("ProgramFiles").unwrap_or_else(|_| String::from("C:\\Program Files")),
        )
        .env(
            "LOCALAPPDATA",
            std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
        );

    let gen_lockfile_process =
        start_child_process(gen_lockfile_cmd, DOTNET_PATH.as_str(), true).await?;
    handle_child(
        job_id,
        conn,
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
        None,
        None,
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
    _conn: &Connection,
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

    let item_group = if pkgs.is_empty() {
        "".to_string()
    } else {
        format!(
            r#"  <ItemGroup>
{pkgs}
  </ItemGroup>
"#
        )
    };

    write_file(
        job_dir,
        "Main.csproj",
        &format!(
            r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net9.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <StartupObject>WindmillScriptCSharpInternal.Wrapper</StartupObject>
    <RestorePackagesWithLockFile>true</RestorePackagesWithLockFile>
  </PropertyGroup>
{item_group}
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
            {class_name}.Main({spread}).Wait();
            File.WriteAllText("result.json", "null");
            "#
        ),
        (false, true) => format!(
            r#"
            {class_name}.Main({spread});
            File.WriteAllText("result.json", "null");
            "#
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

            try
            {{
                {script_call}
            }}
            catch (Exception ex)
            {{
                Console.Error.WriteLine("Unhandeled Exception: " + ex.ToString());
                Environment.Exit(1);
            }}
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
    conn: &Connection,
    worker_name: &str,
    w_id: &str,
    base_internal_url: &str,
    hash: &str,
    occupancy_metrics: &mut OccupancyMetrics,
) -> error::Result<String> {
    if let Some(nuget_config) = read_ee_registry(
        NUGET_CONFIG.read().await.clone(),
        "nuget config",
        job_id,
        w_id,
        conn,
    )
    .await
    {
        if !nuget_config.trim().is_empty() {
            write_file(job_dir, "nuget.config", &nuget_config)?;
        }
    }

    let mut build_cs_cmd = Command::new(DOTNET_PATH.as_str());
    build_cs_cmd
        .current_dir(job_dir)
        .env_clear()
        .env("PATH", PATH_ENV.as_str())
        .env("BASE_INTERNAL_URL", base_internal_url)
        .env("HOME", HOME_ENV.as_str())
        .env("DOTNET_CLI_HOME", CSHARP_CACHE_DIR)
        .env("NUGET_PACKAGES", format!("{CSHARP_CACHE_DIR}/nuget"))
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "true")
        .env("DOTNET_NOLOGO", "true")
        .env("MSBUILDDISABLENODEREUSE", "1")
        .env("DOTNET_ROOT", DOTNET_ROOT.as_str())
        .args(vec![
            "publish",
            "--configuration",
            "Release",
            "-o",
            job_dir,
            "--no-self-contained",
            "-p:PublishSingleFile=true",
            "-p:IncludeNativeLibrariesForSelfExtract=true",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    build_cs_cmd
        .env("SystemRoot", SYSTEM_ROOT.as_str())
        .env(
            "TMP",
            std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
        )
        .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
        .env(
            "APPDATA",
            std::env::var("APPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Roaming", HOME_ENV.as_str())),
        )
        .env(
            "ProgramFiles",
            std::env::var("ProgramFiles").unwrap_or_else(|_| String::from("C:\\Program Files")),
        )
        .env(
            "LOCALAPPDATA",
            std::env::var("LOCALAPPDATA")
                .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
        );

    let build_cs_process = start_child_process(build_cs_cmd, DOTNET_PATH.as_str(), true).await?;
    handle_child(
        job_id,
        conn,
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
        None,
        None,
    )
    .await?;
    append_logs(job_id, w_id, "\n\n", conn).await;
    if let Err(e) = std::fs::remove_file(Path::new(job_dir).join("nuget.config")) {
        if e.kind() != io::ErrorKind::NotFound {
            Err(anyhow!("Error erasing nuget.config: {}", e))?;
        }
    }

    let bin_path = format!("{}/{hash}", CSHARP_CACHE_DIR);
    #[cfg(unix)]
    let target = format!("{job_dir}/Main");
    #[cfg(windows)]
    let target = format!("{job_dir}/Main.exe");

    match save_cache(
        &bin_path,
        &format!("{CSHARP_OBJECT_STORE_PREFIX}{hash}"),
        &target,
        false,
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

#[cfg(feature = "csharp")]
fn remove_lines_from_text(contents: &str, indices_to_remove: Vec<usize>) -> String {
    let mut result = Vec::new();

    for (i, line) in contents.lines().enumerate() {
        if !indices_to_remove.contains(&i) {
            result.push(line);
        }
    }

    result.join("\n")
}

use windmill_common::worker::Connection;
use windmill_queue::MiniPulledJob;

#[cfg(not(feature = "csharp"))]
pub async fn handle_csharp_job(
    _mem_peak: &mut i32,
    _canceled_by: &mut Option<CanceledBy>,
    _job: &MiniPulledJob,
    _conn: &Connection,
    _client: &AuthedClient,
    _parent_runnable_path: Option<String>,
    _inner_content: &str,
    _job_dir: &str,
    _requirements_o: Option<&String>,
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
    job: &MiniPulledJob,
    conn: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    inner_content: &str,
    job_dir: &str,
    requirements_o: Option<&String>,
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
        requirements_o.unwrap_or(&String::new())
    ));
    let bin_path = format!("{}/{hash}", CSHARP_CACHE_DIR);
    let remote_path = format!("{CSHARP_OBJECT_STORE_PREFIX}{hash}");

    let (cache, cache_logs) =
        crate::global_cache::load_cache(&bin_path, &remote_path, false).await;

    let cache_logs = if cache {
        #[cfg(unix)]
        {
            let target = format!("{job_dir}/Main");
            let symlink = std::os::unix::fs::symlink(&bin_path, &target);
            symlink.map_err(|e| {
                Error::ExecutionErr(format!(
                    "could not copy cached binary from {bin_path} to {job_dir}/Main: {e:?}"
                ))
            })?;
        }

        cache_logs
    } else {
        let logs1 = format!("{cache_logs}\n\n--- DOTNET BUILD ---\n");
        append_logs(&job.id, &job.workspace_id, logs1, conn).await;

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
                conn,
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
            conn,
            worker_name,
            &job.workspace_id,
            base_internal_url,
            &hash,
            occupancy_metrics,
        )
        .await?
    };

    create_args_and_out_file(client, job, job_dir, conn).await?;

    let logs2 = format!("{cache_logs}\n\n--- C# CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, format!("{}\n", logs2), conn).await;

    let reserved_variables =
        get_reserved_variables(job, &client.token, conn, parent_runnable_path).await?;

    let child = if is_sandboxing_enabled() {
        write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_CSHARP_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CACHE_DIR}", CSHARP_CACHE_DIR)
                .replace("{CACHE_HASH}", &hash)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{TRACING_PROXY_CA_CERT_PATH}", TRACING_PROXY_CA_CERT_PATH)
                .replace("#{DEV}", DEV_CONF_NSJAIL),
        )?;
        let mut nsjail_cmd = Command::new(NSJAIL_PATH.as_str());
        nsjail_cmd
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::CSharp).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("DOTNET_CLI_HOME", CSHARP_CACHE_DIR)
            .env("NUGET_PACKAGES", format!("{CSHARP_CACHE_DIR}/nuget"))
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "true")
            .env("DOTNET_NOLOGO", "true")
            .env("DOTNET_ROOT", DOTNET_ROOT.as_str())
            .args(vec!["--config", "run.config.proto", "--", "/tmp/main"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        nsjail_cmd.env("SystemRoot", SYSTEM_ROOT.as_str());

        start_child_process(nsjail_cmd, NSJAIL_PATH.as_str(), true).await?
    } else {
        #[cfg(unix)]
        let compiled_executable_name = "./Main".to_string();
        #[cfg(windows)]
        let compiled_executable_name = if cache {
            bin_path.to_string()
        } else {
            format!("{job_dir}/Main.exe")
        };

        let mut run_csharp = build_command_with_isolation(&compiled_executable_name, &[]);
        run_csharp
            .current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .envs(get_proxy_envs_for_lang(&ScriptLang::CSharp).await?)
            .env("PATH", PATH_ENV.as_str())
            .env("TZ", TZ_ENV.as_str())
            .env("DOTNET_CLI_HOME", CSHARP_CACHE_DIR)
            .env("NUGET_PACKAGES", format!("{CSHARP_CACHE_DIR}/nuget"))
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "true")
            .env("DOTNET_NOLOGO", "true")
            .env("DOTNET_ROOT", DOTNET_ROOT.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        run_csharp
            .env("SystemRoot", SYSTEM_ROOT.as_str())
            .env("SystemRoot", SYSTEM_ROOT.as_str())
            .env(
                "TMP",
                std::env::var("TMP").unwrap_or_else(|_| "C:\\tmp".to_string()),
            )
            .env("USERPROFILE", crate::USERPROFILE_ENV.as_str())
            .env(
                "APPDATA",
                std::env::var("APPDATA")
                    .unwrap_or_else(|_| format!("{}\\AppData\\Roaming", HOME_ENV.as_str())),
            )
            .env(
                "ProgramFiles",
                std::env::var("ProgramFiles").unwrap_or_else(|_| String::from("C:\\Program Files")),
            )
            .env(
                "LOCALAPPDATA",
                std::env::var("LOCALAPPDATA")
                    .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
            );

        start_child_process(run_csharp, &compiled_executable_name, true).await?
    };

    handle_child(
        &job.id,
        conn,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "csharp run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;
    read_result(job_dir, None).await
}
