use std::{collections::HashMap, fs, process::Stdio};

use regex::Regex;
use serde_json::{json, value::RawValue};
use sqlx::types::Json;
use tokio::process::Command;
use windmill_common::client::AuthedClient;
use windmill_common::error::Error;
use windmill_common::worker::{to_raw_value, write_file, Connection};
use windmill_queue::{
    append_logs, CanceledBy, MiniPulledJob, INIT_SCRIPT_PATH_PREFIX, PERIODIC_SCRIPT_PATH_PREFIX,
};

#[cfg(windows)]
use crate::SYSTEM_ROOT;

const NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT: &str =
    include_str!("../nsjail/run.powershell.config.proto");

lazy_static::lazy_static! {
    static ref RE_POWERSHELL_IMPORTS: Regex = Regex::new(r#"^Import-Module\s+(?:-Name\s+)?"?([^\s"]+)"?(?:\s+-RequiredVersion\s+"?([^\s"]+)"?)?"#).unwrap();
}

use crate::{
    common::{
        build_args_map, build_command_with_isolation, get_reserved_variables, read_file,
        read_file_content, start_child_process, OccupancyMetrics,
    },
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry, DISABLE_NUSER, HOME_ENV, NSJAIL_PATH, PATH_ENV,
    POWERSHELL_CACHE_DIR, POWERSHELL_PATH, POWERSHELL_REPO_PAT, POWERSHELL_REPO_URL, PROXY_ENVS,
    TZ_ENV,
};

fn val_to_pwsh_param(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::Array(x) => format!(
            "@({})",
            x.into_iter()
                .map(|v| val_to_pwsh_param(v))
                .collect::<Vec<_>>()
                .join(",")
        ),
        serde_json::Value::Object(x) => {
            let str = serde_json::to_string(&x).unwrap_or_else(|_| "{}".to_string());
            let escaped = str.replace("'", "''");
            format!("(ConvertFrom-Json '{escaped}')")
        }
        serde_json::Value::Null => "$null".to_string(),
        serde_json::Value::Bool(x) => format!("${x}"),
        serde_json::Value::String(x) => {
            let escaped = x.replace("'", "''");
            format!("'{escaped}'")
        }
        serde_json::Value::Number(x) => x.to_string(),
    }
}

fn raw_to_pwsh_param(x: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(x) {
        Ok(v) => val_to_pwsh_param(v),
        Err(e) => {
            tracing::error!("Error converting JSON to string: {:?}", e);
            "$null".to_string()
        }
    }
}

fn generate_powershell_install_code() -> String {
    r#"
$ErrorActionPreference = 'Stop'
$availableModules = Get-Module -ListAvailable
$path = '{path}'
$hasPrivateRepo = {has_private_repo}
$hasCredentials = {has_credentials}
$jobId = '{job_id}'
$privateRepoUrl = '{private_repo_url}'
$privateRepoPat = '{private_repo_pat}'

# Setup private repository if configured
$repoName = $null
$credentials = $null
if ($hasPrivateRepo) {
    $repoName = "windmill-private-$jobId"
    $repoUri = "$privateRepoUrl"

    # Create PSCredential for authentication only if PAT is provided
    if ($hasCredentials) {
        $username = "token"
        $patToken = ConvertTo-SecureString $privateRepoPat -AsPlainText -Force
        $credentials = New-Object System.Management.Automation.PSCredential($username, $patToken)
    }

    Write-Host "Registering temporary repository: $repoName"

    # Remove repository if it already exists
    Unregister-PSResourceRepository -Name $repoName -ErrorAction SilentlyContinue
    Register-PSResourceRepository -Name $repoName -Uri $repoUri -Trusted
}

try {
    $moduleRequests = @({modules})
    foreach ($moduleRequest in $moduleRequests) {
        $moduleName = $moduleRequest.Name
        $requiredVersion = $moduleRequest.Version

        # Check if module is already installed with the required version (case-insensitive)
        $isInstalled = $false
        if ($requiredVersion) {
            $isInstalled = $availableModules | Where-Object { $_.Name -eq $moduleName -and $_.Version -eq $requiredVersion }
        } else {
            $isInstalled = $availableModules | Where-Object { $_.Name -eq $moduleName }
        }

        if (-not $isInstalled) {
            $moduleFound = $false

            # First try private repository if configured
            if ($hasPrivateRepo) {
                $findParams = @{ Name = $moduleName; Repository = $repoName; ErrorAction = 'SilentlyContinue' }
                if ($credentials) { $findParams.Credential = $credentials }
                if ($requiredVersion) { $findParams.Version = $requiredVersion }

                $privateModule = Find-PSResource @findParams
                if ($privateModule) {
                    $moduleFound = $true
                    $versionInfo = if ($requiredVersion) { " version $requiredVersion" } else { "" }
                    Write-Host "Found module $moduleName$versionInfo in private repository, installing from there..."

                    $saveParams = @{ Name = $moduleName; Path = $path; Repository = $repoName }
                    if ($credentials) { $saveParams.Credential = $credentials }
                    if ($requiredVersion) { $saveParams.Version = $requiredVersion }
                    Save-PSResource @saveParams
                }
            }

            # If not found in private repo (or no private repo configured), try all repositories
            if (-not $moduleFound) {
                $versionInfo = if ($requiredVersion) { " version $requiredVersion" } else { "" }
                Write-Host "Installing module $moduleName$versionInfo from public repositories..."

                $saveParams = @{ Name = $moduleName; Path = $path; TrustRepository = $true }
                if ($requiredVersion) { $saveParams.Version = $requiredVersion }
                Save-PSResource @saveParams
            }
        } else {
            $versionInfo = if ($requiredVersion) { " version $requiredVersion" } else { "" }
            Write-Host "Module $moduleName$versionInfo already installed"
        }
    }
} finally {
    if ($hasPrivateRepo) {
        Write-Host "Unregistering temporary repository: $repoName"
        Unregister-PSResourceRepository -Name $repoName
    }
}
"#.to_string()
}

async fn scan_module_directories() -> Result<HashMap<String, String>, Error> {
    let mut module_dirs = HashMap::new();
    let cache_dir = std::path::Path::new(POWERSHELL_CACHE_DIR);

    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let module_path = entry.path();
                if module_path.is_dir() {
                    if let Some(module_name) = module_path.file_name().and_then(|n| n.to_str()) {
                        module_dirs.insert(
                            module_name.to_lowercase(), // Use lowercase for case-insensitive lookup
                            module_path.to_string_lossy().to_string(),
                        );
                    }
                }
            }
        }
    }

    Ok(module_dirs)
}

async fn get_module_versions(module_path: &str) -> Result<Vec<String>, Error> {
    let mut versions = Vec::new();

    // Look for version subdirectories within the module directory
    if let Ok(version_entries) = fs::read_dir(module_path) {
        for version_entry in version_entries {
            if let Ok(version_entry) = version_entry {
                let version_path = version_entry.path();
                if version_path.is_dir() {
                    let version = version_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string();

                    // Check if this looks like a version (contains dots and numbers)
                    if version.chars().any(|c| c.is_numeric()) && version.contains('.') {
                        versions.push(version);
                    }
                }
            }
        }
    }

    // If no version subdirectories found, treat as single version installation
    if versions.is_empty() {
        versions.push("unknown".to_string());
    }

    Ok(versions)
}

async fn check_module_installed(
    module_dirs: &HashMap<String, String>,
    module_name: &str,
    required_version: Option<&str>,
) -> Result<(bool, Vec<String>), Error> {
    let module_key = module_name.to_lowercase();

    if let Some(module_path) = module_dirs.get(&module_key) {
        let versions = get_module_versions(module_path).await?;
        let is_installed = match required_version {
            Some(version) => versions.iter().any(|v| v == version),
            None => !versions.is_empty(),
        };
        Ok((is_installed, versions))
    } else {
        Ok((false, Vec::new()))
    }
}

#[derive(Debug)]
struct ModuleRequest {
    name: String,
    version: Option<String>,
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_powershell_job(
    mem_peak: &mut i32,
    canceled_by: &mut Option<CanceledBy>,
    job: &MiniPulledJob,
    db: &Connection,
    client: &AuthedClient,
    parent_runnable_path: Option<String>,
    content: &str,
    job_dir: &str,
    shared_mount: &str,
    base_internal_url: &str,
    worker_name: &str,
    envs: HashMap<String, String>,
    occupancy_metrics: &mut OccupancyMetrics,
) -> Result<Box<RawValue>, Error> {
    let pwsh_args = {
        let args = build_args_map(job, client, &db).await?.map(Json);
        let job_args = if args.is_some() {
            args.as_ref()
        } else {
            job.args.as_ref()
        };

        let parsed_sig = windmill_parser_bash::parse_powershell_sig(&content)?;

        parsed_sig
            .args
            .iter()
            .filter_map(|arg| {
                let value_opt = job_args.and_then(|x| x.get(&arg.name));

                // Check if this is a switch parameter (only [switch], not [bool])
                let is_switch = arg
                    .otyp
                    .as_ref()
                    .map(|t| t.to_lowercase() == "switch")
                    .unwrap_or(false);

                if is_switch {
                    // Handle switch parameters: -SwitchName or omit
                    if let Some(value) = value_opt {
                        match serde_json::from_str::<serde_json::Value>(value.get()) {
                            Ok(serde_json::Value::Bool(true)) => {
                                // Switch is enabled: just pass -SwitchName
                                Some(format!("-{}", arg.name))
                            }
                            Ok(serde_json::Value::Bool(false)) | _ => {
                                // Switch is disabled or invalid: omit the parameter
                                None
                            }
                        }
                    } else {
                        // No value provided, omit the switch (defaults to false)
                        None
                    }
                } else {
                    // Regular parameter (including [bool]): format as -ParamName Value
                    // For [bool] parameters, this will be -ParamName $true or -ParamName $false
                    value_opt.map(|v| format!("-{} {}", arg.name, raw_to_pwsh_param(v.get())))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    // First, collect all imported modules
    let mut imported_modules: Vec<(String, Option<String>)> = Vec::new();
    for line in content.lines() {
        for cap in RE_POWERSHELL_IMPORTS.captures_iter(line) {
            let module_name = cap.get(1).unwrap().as_str().to_string();
            let required_version = cap.get(2).map(|m| m.as_str().to_string());
            imported_modules.push((module_name, required_version));
        }
    }

    // Only scan the top-level cache directory if there are modules to check
    let module_dirs = if !imported_modules.is_empty() {
        scan_module_directories().await?
    } else {
        HashMap::new()
    };

    let mut modules_to_install: Vec<ModuleRequest> = Vec::new();
    let mut logs1 = String::new();

    for (module_name, required_version) in imported_modules {
        // Check if this specific module is already installed, only scanning versions if needed
        let (is_installed, installed_versions) =
            check_module_installed(&module_dirs, &module_name, required_version.as_deref()).await?;

        if !is_installed {
            modules_to_install.push(ModuleRequest {
                name: module_name.clone(),
                version: required_version.clone(),
            });
        } else {
            // Log what versions are actually installed
            let version_info = if let Some(version) = &required_version {
                format!(" version {} found in cache", version)
            } else if installed_versions.len() == 1 {
                format!(" (version {}) found in cache", installed_versions[0])
            } else if installed_versions.len() > 1 {
                format!(
                    " (versions: {}) found in cache",
                    installed_versions.join(", ")
                )
            } else {
                " found in cache".to_string()
            };
            logs1.push_str(&format!("\n{}{}", module_name, version_info));
        }
    }

    if !logs1.is_empty() {
        append_logs(&job.id, &job.workspace_id, logs1, db).await;
    }

    if !modules_to_install.is_empty() {
        let powershell_repo_url = read_ee_registry(
            POWERSHELL_REPO_URL.read().await.clone(),
            "powershell repo url",
            &job.id,
            &job.workspace_id,
            db,
        )
        .await;
        let powershell_repo_pat = read_ee_registry(
            POWERSHELL_REPO_PAT.read().await.clone(),
            "powershell repo pat",
            &job.id,
            &job.workspace_id,
            db,
        )
        .await;
        let has_private_repo = powershell_repo_url.is_some();
        let has_credentials = powershell_repo_pat.is_some();

        let modules_list = modules_to_install
            .iter()
            .map(|module_req| {
                if let Some(version) = &module_req.version {
                    format!(
                        "@{{ Name = '{}'; Version = '{}' }}",
                        module_req.name, version
                    )
                } else {
                    format!("@{{ Name = '{}'; Version = $null }}", module_req.name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let install_string = generate_powershell_install_code()
            .replace("{path}", POWERSHELL_CACHE_DIR)
            .replace("{job_id}", &job.id.to_string())
            .replace("{has_private_repo}", &format!("${has_private_repo}"))
            .replace("{has_credentials}", &format!("${has_credentials}"))
            .replace(
                "{private_repo_url}",
                &powershell_repo_url.unwrap_or_default(),
            )
            .replace(
                "{private_repo_pat}",
                &powershell_repo_pat.unwrap_or_default(),
            )
            .replace("{modules}", &modules_list);
        let mut cmd = Command::new(POWERSHELL_PATH.as_str());
        cmd.args(&["-Command", &install_string])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = start_child_process(cmd, POWERSHELL_PATH.as_str(), false).await?;

        handle_child(
            &job.id,
            db,
            mem_peak,
            canceled_by,
            child,
            false,
            worker_name,
            &job.workspace_id,
            "powershell install",
            job.timeout,
            false,
            &mut Some(occupancy_metrics),
            None,
            None,
        )
        .await?;
    }

    let mut logs2 = "".to_string();
    logs2.push_str("\n\n--- POWERSHELL CODE EXECUTION ---\n");
    append_logs(&job.id, &job.workspace_id, logs2, db).await;

    // make sure default (only allhostsallusers) modules are loaded, disable autoload (cache can be large to explore especially on cloud) and add /tmp/windmill/cache to PSModulePath
    #[cfg(unix)]
    let profile = format!(
        "$PSModuleAutoloadingPreference = 'None'
$PSModulePathBackup = $env:PSModulePath
$env:PSModulePath = \"$PSHome/Modules\"
Get-Module -ListAvailable | Import-Module
$env:PSModulePath = \"{}:$PSModulePathBackup\"",
        POWERSHELL_CACHE_DIR
    );

    #[cfg(windows)]
    let profile = format!(
        "$PSModuleAutoloadingPreference = 'None'
$PSModulePathBackup = $env:PSModulePath
$env:PSModulePath = \"C:\\Program Files\\PowerShell\\7\\Modules\"
Get-Module -ListAvailable | Import-Module
$env:PSModulePath = \"{};$PSModulePathBackup\"",
        POWERSHELL_CACHE_DIR
    );

    // NOTE: powershell error handling / termination is quite tricky compared to bash
    // here we're trying to catch terminating errors and propagate the exit code
    // to the caller such that the job will be marked as failed. It's up to the user
    // to catch specific errors in their script not caught by the below as there is no
    // generic set -eu as in bash
    let strict_termination_start = "$ErrorActionPreference = 'Stop'\n\
        Set-StrictMode -Version Latest\n\
        try {\n";

    let strict_termination_end = "\n\
        } catch {\n\
            Write-Output \"An error occurred:\n\"\
            Write-Output $_
            exit 1\n\
        }\n";

    // make sure param() with its attributes is first
    let content: String = if let Some((param_block, remaining_code)) =
        windmill_parser_bash::extract_powershell_param_block_with_attributes(&content, true)
    {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            param_block, profile, strict_termination_start, remaining_code, strict_termination_end
        )
    } else {
        format!("{}\n{}", profile, content)
    };

    write_file(job_dir, "main.ps1", content.as_str())?;

    write_file(
        job_dir,
        "wrapper.ps1",
        &format!(
            "$ErrorActionPreference = 'Stop'\n\
    $pipe = New-TemporaryFile\n\
    ./main.ps1 {pwsh_args} 2>&1 | Tee-Object -FilePath $pipe\n\
    Get-Content -Path $pipe | Select-Object -Last 1 | Set-Content -Path './result2.out'\n\
    Remove-Item $pipe\n\
    exit $LASTEXITCODE\n"
        ),
    )?;

    let mut reserved_variables =
        get_reserved_variables(job, &client.token, db, parent_runnable_path).await?;
    reserved_variables.insert("RUST_LOG".to_string(), "info".to_string());

    let _ = write_file(job_dir, "result.json", "")?;
    let _ = write_file(job_dir, "result.out", "")?;
    let _ = write_file(job_dir, "result2.out", "")?;

    // Check if this is a regular job (not init or periodic script)
    // Init/periodic scripts need full system access without isolation
    let is_regular_job = job
        .runnable_path
        .as_ref()
        .map(|x| {
            !x.starts_with(INIT_SCRIPT_PATH_PREFIX) && !x.starts_with(PERIODIC_SCRIPT_PATH_PREFIX)
        })
        .unwrap_or(true);

    let nsjail = is_sandboxing_enabled() && is_regular_job;
    let child = if nsjail {
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{CACHE_DIR}", POWERSHELL_CACHE_DIR),
        )?;
        let cmd_args = vec![
            "--config",
            "run.config.proto",
            "--",
            POWERSHELL_PATH.as_str(),
            "wrapper.ps1",
        ];
        let mut cmd = Command::new(NSJAIL_PATH.as_str());
        cmd.current_dir(job_dir)
            .env_clear()
            .envs(PROXY_ENVS.clone())
            .envs(reserved_variables)
            .env("TZ", TZ_ENV.as_str())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .args(cmd_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        start_child_process(cmd, NSJAIL_PATH.as_str(), false).await?
    } else {
        let cmd_args;

        #[cfg(unix)]
        {
            cmd_args = vec!["wrapper.ps1"];
        }

        #[cfg(windows)]
        {
            cmd_args = vec![r".\wrapper.ps1"];
        }

        // Only apply unshare isolation for regular jobs, not init/periodic scripts
        let mut cmd = if is_regular_job {
            build_command_with_isolation(POWERSHELL_PATH.as_str(), &cmd_args)
        } else {
            let mut c = Command::new(POWERSHELL_PATH.as_str());
            c.args(&cmd_args);
            c
        };

        cmd.current_dir(job_dir)
            .env_clear()
            .envs(envs)
            .envs(reserved_variables)
            .env("TZ", TZ_ENV.as_str())
            .env("PATH", PATH_ENV.as_str())
            .env("BASE_INTERNAL_URL", base_internal_url)
            .env("HOME", HOME_ENV.as_str())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            cmd.env("SystemRoot", SYSTEM_ROOT.as_str())
                .env("WINDIR", SYSTEM_ROOT.as_str())
                .env(
                    "LOCALAPPDATA",
                    std::env::var("LOCALAPPDATA")
                        .unwrap_or_else(|_| format!("{}\\AppData\\Local", HOME_ENV.as_str())),
                )
                .env(
                    "ProgramData",
                    std::env::var("ProgramData")
                        .unwrap_or_else(|_| String::from("C:\\ProgramData")),
                )
                .env(
                    "ProgramFiles",
                    std::env::var("ProgramFiles")
                        .unwrap_or_else(|_| String::from("C:\\Program Files")),
                )
                .env(
                    "ProgramFiles(x86)",
                    std::env::var("ProgramFiles(x86)")
                        .unwrap_or_else(|_| String::from("C:\\Program Files (x86)")),
                )
                .env(
                    "ProgramW6432",
                    std::env::var("ProgramW6432")
                        .unwrap_or_else(|_| String::from("C:\\Program Files")),
                )
                .env(
                    "TMP",
                    std::env::var("TMP").unwrap_or_else(|_| String::from("/tmp")),
                )
                .env(
                    "PATHEXT",
                    std::env::var("PATHEXT").unwrap_or_else(|_| {
                        String::from(".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC;.CPL")
                    }),
                )
                .env("USERPROFILE", crate::USERPROFILE_ENV.as_str());
        }

        start_child_process(cmd, POWERSHELL_PATH.as_str(), false).await?
    };

    handle_child(
        &job.id,
        db,
        mem_peak,
        canceled_by,
        child,
        is_sandboxing_enabled(),
        worker_name,
        &job.workspace_id,
        "powershell run",
        job.timeout,
        false,
        &mut Some(occupancy_metrics),
        None,
        None,
    )
    .await?;

    let result_json_path = format!("{job_dir}/result.json");
    if let Ok(metadata) = tokio::fs::metadata(&result_json_path).await {
        if metadata.len() > 0 {
            return Ok(read_file(&result_json_path).await?);
        }
    }

    let result_out_path = format!("{job_dir}/result.out");
    if let Ok(metadata) = tokio::fs::metadata(&result_out_path).await {
        if metadata.len() > 0 {
            let result = read_file_content(&result_out_path).await?;
            return Ok(to_raw_value(&json!(result)));
        }
    }

    let result_out_path2 = format!("{job_dir}/result2.out");
    if tokio::fs::metadata(&result_out_path2).await.is_ok() {
        let result = read_file_content(&result_out_path2)
            .await?
            .trim()
            .to_string();
        return Ok(to_raw_value(&json!(result)));
    }

    Ok(to_raw_value(&json!(
        "No result.out, result2.out or result.json found"
    )))
}
