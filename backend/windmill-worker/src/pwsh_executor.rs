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
    static ref RE_POWERSHELL_IMPORTS: Regex = Regex::new(r#"^\s*Import-Module\s+(?:-Name\s+)?"?([^\s"]+)"?(?:\s+-RequiredVersion\s+"?([^\s"]+)"?)?"#).unwrap();
}

use crate::{
    common::{
        build_args_map, build_command_with_isolation, get_reserved_variables, read_file,
        read_file_content, resolve_nsjail_timeout, start_child_process, MaybeLock,
        OccupancyMetrics,
    },
    handle_child::handle_child,
    is_sandboxing_enabled, read_ee_registry_with_workspace_override, DISABLE_NUSER, HOME_ENV,
    NSJAIL_PATH, PATH_ENV, POWERSHELL_CACHE_DIR, POWERSHELL_PATH, POWERSHELL_REPO_PAT,
    POWERSHELL_REPO_URL, PROXY_ENVS, TZ_ENV,
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
    let cache_dir = std::path::Path::new(&*POWERSHELL_CACHE_DIR);

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
                    // and verify a module manifest (.psd1) or script (.psm1) actually exists
                    if version.chars().any(|c| c.is_numeric()) && version.contains('.') {
                        let has_module_files = fs::read_dir(&version_path)
                            .map(|entries| {
                                entries.filter_map(|e| e.ok()).any(|e| {
                                    let name = e.file_name();
                                    let name = name.to_string_lossy();
                                    name.ends_with(".psd1") || name.ends_with(".psm1")
                                })
                            })
                            .unwrap_or(false);
                        if has_module_files {
                            versions.push(version);
                        }
                    }
                }
            }
        }
    }

    // If no version subdirectories found, check if module files exist directly
    // in the module directory (flat/single-version installation)
    if versions.is_empty() {
        let has_module_files = fs::read_dir(module_path)
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.ends_with(".psd1") || name.ends_with(".psm1")
                })
            })
            .unwrap_or(false);
        if has_module_files {
            versions.push("unknown".to_string());
        }
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

/// Parse Import-Module statements from PowerShell code into module requests.
fn parse_script_imports(code: &str) -> Vec<ModuleRequest> {
    let mut modules = Vec::new();
    for line in code.lines() {
        for cap in RE_POWERSHELL_IMPORTS.captures_iter(line) {
            let name = cap.get(1).unwrap().as_str().to_string();
            let version = cap.get(2).map(|m| m.as_str().to_string());
            modules.push(ModuleRequest { name, version });
        }
    }
    modules
}

/// Parse a modules.json workspace dependencies content into module requests.
/// Format: { "modules": { "ModuleName": "1.0.0", "Another": null } }
fn parse_modules_json(content: &str) -> Result<Vec<ModuleRequest>, Error> {
    let parsed: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        Error::internal_err(format!("Failed to parse PowerShell modules.json: {e}"))
    })?;
    let modules = parsed
        .get("modules")
        .and_then(|m| m.as_object())
        .ok_or_else(|| {
            Error::internal_err(
                "PowerShell modules.json must have a \"modules\" object".to_string(),
            )
        })?;
    let mut result = Vec::new();
    for (name, version) in modules {
        let version = match version {
            serde_json::Value::String(v) if v != "*" => Some(v.clone()),
            _ => None,
        };
        result.push(ModuleRequest { name: name.clone(), version });
    }
    Ok(result)
}

/// Merge workspace dependency modules with script import modules.
/// Workspace dependency versions take precedence on overlap.
fn merge_module_requests(
    workspace_modules: Vec<ModuleRequest>,
    script_modules: Vec<ModuleRequest>,
) -> Vec<ModuleRequest> {
    let mut seen: HashMap<String, ModuleRequest> = HashMap::new();
    // Script imports first (lower priority)
    for m in script_modules {
        let key = m.name.to_lowercase();
        seen.entry(key).or_insert(m);
    }
    // Workspace deps override
    for m in workspace_modules {
        let key = m.name.to_lowercase();
        seen.insert(key, m);
    }
    seen.into_values().collect()
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn handle_powershell_job(
    maybe_lock: MaybeLock,
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
    let (pwsh_args, common_pwsh_args) = {
        let args = build_args_map(job, client, &db).await?.map(Json);
        let job_args = if args.is_some() {
            args.as_ref()
        } else {
            job.args.as_ref()
        };

        let parsed_sig = windmill_parser_bash::parse_powershell_sig(&content)?;

        let user_args = parsed_sig
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
            .join(" ");

        // Extract PowerShell common parameters (_wm_ps_* keys)
        let mut common = Vec::new();
        if let Some(args_map) = job_args {
            if let Some(v) = args_map.get("_wm_ps_verbose") {
                if serde_json::from_str::<bool>(v.get()).unwrap_or(false) {
                    common.push("-Verbose".to_string());
                }
            }
            if let Some(v) = args_map.get("_wm_ps_debug") {
                if serde_json::from_str::<bool>(v.get()).unwrap_or(false) {
                    common.push("-Debug".to_string());
                }
            }
            if let Some(v) = args_map.get("_wm_ps_error_action") {
                if let Ok(action) = serde_json::from_str::<String>(v.get()) {
                    if matches!(action.as_str(), "Stop" | "Continue" | "SilentlyContinue") {
                        common.push(format!("-ErrorAction {}", action));
                    }
                }
            }
            // Note: -WhatIf is not supported because $PSCmdlet.ShouldProcess()
            // doesn't work when scripts are invoked through a wrapper
        }
        let common_args = common.join(" ");

        (user_args, common_args)
    };

    // Resolve modules from workspace dependencies and/or script imports
    let all_modules = match &maybe_lock {
        MaybeLock::Resolved { lock } if !lock.is_empty() => {
            // Deployed script with lock: parse workspace deps from lock, merge with script imports
            let ws_modules = parse_modules_json(lock)?;
            let script_modules = parse_script_imports(content);
            merge_module_requests(ws_modules, script_modules)
        }
        MaybeLock::Unresolved { workspace_dependencies } => {
            let script_modules = parse_script_imports(content);
            match workspace_dependencies.get_powershell()? {
                Some(modules_json) => {
                    let ws_modules = parse_modules_json(&modules_json)?;
                    merge_module_requests(ws_modules, script_modules)
                }
                None => script_modules,
            }
        }
        _ => parse_script_imports(content),
    };

    // Only scan the top-level cache directory if there are modules to check
    let module_dirs = if !all_modules.is_empty() {
        scan_module_directories().await?
    } else {
        HashMap::new()
    };

    let mut modules_to_install: Vec<ModuleRequest> = Vec::new();
    let mut logs1 = String::new();

    for module_req in all_modules {
        // Check if this specific module is already installed, only scanning versions if needed
        let (is_installed, installed_versions) = check_module_installed(
            &module_dirs,
            &module_req.name,
            module_req.version.as_deref(),
        )
        .await?;

        if !is_installed {
            modules_to_install.push(module_req);
        } else {
            // Log what versions are actually installed
            let version_info = if let Some(version) = &module_req.version {
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
            logs1.push_str(&format!("\n{}{}", module_req.name, version_info));
        }
    }

    if !logs1.is_empty() {
        append_logs(&job.id, &job.workspace_id, logs1, db).await;
    }

    if !modules_to_install.is_empty() {
        let powershell_repo_url = read_ee_registry_with_workspace_override(
            POWERSHELL_REPO_URL.read().await.clone(),
            "powershell_repo_url",
            "powershell repo url",
            &job.id,
            &job.workspace_id,
            db,
        )
        .await;
        let powershell_repo_pat = read_ee_registry_with_workspace_override(
            POWERSHELL_REPO_PAT.read().await.clone(),
            "powershell_repo_pat",
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
            .replace("{path}", &*POWERSHELL_CACHE_DIR)
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
        *POWERSHELL_CACHE_DIR
    );

    #[cfg(windows)]
    let profile = format!(
        "$PSModuleAutoloadingPreference = 'None'
$PSModulePathBackup = $env:PSModulePath
$env:PSModulePath = \"C:\\Program Files\\PowerShell\\7\\Modules\"
Get-Module -ListAvailable | Import-Module
$env:PSModulePath = \"{};$PSModulePathBackup\"",
        *POWERSHELL_CACHE_DIR
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
            Write-Output \"An error occurred:\"\n\
            Write-Output $_\n\
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

    let all_pwsh_args = if common_pwsh_args.is_empty() {
        pwsh_args
    } else if pwsh_args.is_empty() {
        common_pwsh_args
    } else {
        format!("{pwsh_args} {common_pwsh_args}")
    };

    write_file(
        job_dir,
        "wrapper.ps1",
        &format!(
            "$ErrorActionPreference = 'Stop'\n\
    $pipe = New-TemporaryFile\n\
    ./main.ps1 {all_pwsh_args} 2>&1 4>verbose.log 5>debug.log | Tee-Object -FilePath $pipe\n\
    $exitcode = $LASTEXITCODE\n\
    if (Test-Path verbose.log) {{ Get-Content verbose.log | ForEach-Object {{ [Console]::Error.WriteLine(\"VERBOSE: $_\") }} }}\n\
    if (Test-Path debug.log) {{ Get-Content debug.log | ForEach-Object {{ [Console]::Error.WriteLine(\"DEBUG: $_\") }} }}\n\
    Get-Content -Path $pipe | Select-Object -Last 1 | Set-Content -Path './result2.out'\n\
    Remove-Item $pipe\n\
    exit $exitcode\n"
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
        let nsjail_timeout =
            resolve_nsjail_timeout(db, &job.workspace_id, job.id, job.timeout).await;
        let _ = write_file(
            job_dir,
            "run.config.proto",
            &NSJAIL_CONFIG_RUN_POWERSHELL_CONTENT
                .replace("{JOB_DIR}", job_dir)
                .replace("{CLONE_NEWUSER}", &(!*DISABLE_NUSER).to_string())
                .replace("{SHARED_MOUNT}", shared_mount)
                .replace("{CACHE_DIR}", &*POWERSHELL_CACHE_DIR)
                .replace("{TIMEOUT}", &nsjail_timeout),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- RE_POWERSHELL_IMPORTS regex tests ---

    fn match_import(line: &str) -> Option<(String, Option<String>)> {
        RE_POWERSHELL_IMPORTS.captures(line).map(|cap| {
            let name = cap.get(1).unwrap().as_str().to_string();
            let version = cap.get(2).map(|m| m.as_str().to_string());
            (name, version)
        })
    }

    #[test]
    fn test_import_module_basic() {
        let (name, version) = match_import("Import-Module WindmillClient").unwrap();
        assert_eq!(name, "WindmillClient");
        assert_eq!(version, None);
    }

    #[test]
    fn test_import_module_with_leading_whitespace() {
        let (name, _) = match_import("    Import-Module WindmillClient").unwrap();
        assert_eq!(name, "WindmillClient");
    }

    #[test]
    fn test_import_module_with_tab_indent() {
        let (name, _) = match_import("\tImport-Module WindmillClient").unwrap();
        assert_eq!(name, "WindmillClient");
    }

    #[test]
    fn test_import_module_with_name_flag() {
        let (name, _) = match_import("Import-Module -Name WindmillClient").unwrap();
        assert_eq!(name, "WindmillClient");
    }

    #[test]
    fn test_import_module_with_required_version() {
        let (name, version) =
            match_import(r#"Import-Module WindmillClient -RequiredVersion "1.655.0""#).unwrap();
        assert_eq!(name, "WindmillClient");
        assert_eq!(version, Some("1.655.0".to_string()));
    }

    #[test]
    fn test_import_module_quoted_name() {
        let (name, _) = match_import(r#"Import-Module "WindmillClient""#).unwrap();
        assert_eq!(name, "WindmillClient");
    }

    #[test]
    fn test_import_module_name_flag_quoted_with_version() {
        let (name, version) =
            match_import(r#"Import-Module -Name "WindmillClient" -RequiredVersion "2.0.0""#)
                .unwrap();
        assert_eq!(name, "WindmillClient");
        assert_eq!(version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_import_module_indented_with_version() {
        let (name, version) =
            match_import(r#"    Import-Module WindmillClient -RequiredVersion 1.0.0"#).unwrap();
        assert_eq!(name, "WindmillClient");
        assert_eq!(version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_commented_import_not_matched() {
        assert!(match_import("# Import-Module WindmillClient").is_none());
    }

    // --- get_module_versions / check_module_installed tests ---

    #[tokio::test]
    async fn test_empty_module_dir_not_installed() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        fs::create_dir(&module_dir).unwrap();

        let versions = get_module_versions(module_dir.to_str().unwrap())
            .await
            .unwrap();
        assert!(versions.is_empty(), "empty dir should have no versions");
    }

    #[tokio::test]
    async fn test_empty_version_subdir_not_installed() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        let version_dir = module_dir.join("1.655.0");
        fs::create_dir_all(&version_dir).unwrap();

        let versions = get_module_versions(module_dir.to_str().unwrap())
            .await
            .unwrap();
        assert!(
            versions.is_empty(),
            "version dir without .psd1/.psm1 should not count"
        );
    }

    #[tokio::test]
    async fn test_valid_versioned_module_detected() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        let version_dir = module_dir.join("1.655.0");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(version_dir.join("WindmillClient.psd1"), "# manifest").unwrap();
        fs::write(version_dir.join("WindmillClient.psm1"), "# module").unwrap();

        let versions = get_module_versions(module_dir.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(versions, vec!["1.655.0"]);
    }

    #[tokio::test]
    async fn test_flat_module_with_files_detected() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("MyModule");
        fs::create_dir(&module_dir).unwrap();
        fs::write(module_dir.join("MyModule.psm1"), "# module").unwrap();

        let versions = get_module_versions(module_dir.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(versions, vec!["unknown"]);
    }

    #[tokio::test]
    async fn test_flat_module_without_files_not_detected() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("MyModule");
        fs::create_dir(&module_dir).unwrap();
        fs::write(module_dir.join("readme.txt"), "not a module").unwrap();

        let versions = get_module_versions(module_dir.to_str().unwrap())
            .await
            .unwrap();
        assert!(versions.is_empty());
    }

    #[tokio::test]
    async fn test_check_module_installed_empty_dir_returns_false() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        fs::create_dir(&module_dir).unwrap();

        let mut dirs = HashMap::new();
        dirs.insert(
            "windmillclient".to_string(),
            module_dir.to_str().unwrap().to_string(),
        );

        let (installed, _) = check_module_installed(&dirs, "WindmillClient", None)
            .await
            .unwrap();
        assert!(
            !installed,
            "empty module dir should not be considered installed"
        );
    }

    #[tokio::test]
    async fn test_check_module_installed_valid_module_returns_true() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        let version_dir = module_dir.join("1.655.0");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(version_dir.join("WindmillClient.psd1"), "# manifest").unwrap();

        let mut dirs = HashMap::new();
        dirs.insert(
            "windmillclient".to_string(),
            module_dir.to_str().unwrap().to_string(),
        );

        let (installed, versions) = check_module_installed(&dirs, "WindmillClient", None)
            .await
            .unwrap();
        assert!(installed);
        assert_eq!(versions, vec!["1.655.0"]);
    }

    #[tokio::test]
    async fn test_check_module_installed_wrong_version_returns_false() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        let version_dir = module_dir.join("1.0.0");
        fs::create_dir_all(&version_dir).unwrap();
        fs::write(version_dir.join("WindmillClient.psd1"), "# manifest").unwrap();

        let mut dirs = HashMap::new();
        dirs.insert(
            "windmillclient".to_string(),
            module_dir.to_str().unwrap().to_string(),
        );

        let (installed, _) = check_module_installed(&dirs, "WindmillClient", Some("2.0.0"))
            .await
            .unwrap();
        assert!(!installed, "wrong version should not match");
    }

    // --- parse_modules_json tests ---

    #[test]
    fn test_parse_modules_json_basic() {
        let json = r#"{"modules": {"PSWriteColor": "1.0.0", "ImportExcel": null}}"#;
        let modules = parse_modules_json(json).unwrap();
        assert_eq!(modules.len(), 2);
        let by_name: HashMap<String, Option<String>> =
            modules.into_iter().map(|m| (m.name, m.version)).collect();
        assert_eq!(by_name["PSWriteColor"], Some("1.0.0".to_string()));
        assert_eq!(by_name["ImportExcel"], None);
    }

    #[test]
    fn test_parse_modules_json_wildcard_treated_as_none() {
        let json = r#"{"modules": {"Mod": "*"}}"#;
        let modules = parse_modules_json(json).unwrap();
        assert_eq!(modules[0].version, None);
    }

    #[test]
    fn test_parse_modules_json_empty() {
        let json = r#"{"modules": {}}"#;
        let modules = parse_modules_json(json).unwrap();
        assert!(modules.is_empty());
    }

    #[test]
    fn test_parse_modules_json_missing_modules_key() {
        let json = r#"{"deps": {}}"#;
        assert!(parse_modules_json(json).is_err());
    }

    #[test]
    fn test_parse_modules_json_invalid_json() {
        assert!(parse_modules_json("not json").is_err());
    }

    // --- parse_script_imports tests ---

    #[test]
    fn test_parse_script_imports() {
        let code = r#"Import-Module PSWriteColor
Import-Module ImportExcel -RequiredVersion "7.8.6"
# Import-Module Commented
Write-Host "Hello""#;
        let modules = parse_script_imports(code);
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0].name, "PSWriteColor");
        assert_eq!(modules[0].version, None);
        assert_eq!(modules[1].name, "ImportExcel");
        assert_eq!(modules[1].version, Some("7.8.6".to_string()));
    }

    // --- merge_module_requests tests ---

    #[test]
    fn test_merge_workspace_overrides_script() {
        let ws = vec![ModuleRequest { name: "Mod".to_string(), version: Some("2.0".to_string()) }];
        let script =
            vec![ModuleRequest { name: "Mod".to_string(), version: Some("1.0".to_string()) }];
        let merged = merge_module_requests(ws, script);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].version, Some("2.0".to_string()));
    }

    #[test]
    fn test_merge_combines_distinct_modules() {
        let ws = vec![ModuleRequest { name: "WsMod".to_string(), version: None }];
        let script = vec![ModuleRequest { name: "ScriptMod".to_string(), version: None }];
        let merged = merge_module_requests(ws, script);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn test_merge_case_insensitive() {
        let ws =
            vec![ModuleRequest { name: "MyModule".to_string(), version: Some("2.0".to_string()) }];
        let script =
            vec![ModuleRequest { name: "mymodule".to_string(), version: Some("1.0".to_string()) }];
        let merged = merge_module_requests(ws, script);
        assert_eq!(merged.len(), 1);
        // Workspace version wins
        assert_eq!(merged[0].version, Some("2.0".to_string()));
    }

    #[tokio::test]
    async fn test_multiple_versions_detected() {
        let tmp = TempDir::new().unwrap();
        let module_dir = tmp.path().join("WindmillClient");
        for ver in &["1.0.0", "1.655.0"] {
            let version_dir = module_dir.join(ver);
            fs::create_dir_all(&version_dir).unwrap();
            fs::write(version_dir.join("WindmillClient.psd1"), "# manifest").unwrap();
        }

        let mut versions = get_module_versions(module_dir.to_str().unwrap())
            .await
            .unwrap();
        versions.sort();
        assert_eq!(versions, vec!["1.0.0", "1.655.0"]);
    }
}
