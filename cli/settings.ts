import process from "node:process";
import { colors, Confirm, log, yamlParseFile } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { AIConfig, Config, GlobalSetting } from "./gen/types.gen.ts";
import { compareInstanceObjects, InstanceSyncOptions } from "./instance.ts";
import { isSuperset } from "./types.ts";
import { deepEqual } from "./utils.ts";
import { removeWorkerPrefix } from "./worker_groups.ts";
import { Command } from "./deps.ts";
import { GlobalOptions } from "./types.ts";
import { SyncOptions, DEFAULT_SYNC_OPTIONS, readConfigFile } from "./conf.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { Workspace } from "./workspace.ts";
import {
  uiStateToSyncOptions,
  parseJsonInput,
  normalizeRepositoryPath,
  displayRepositoryPath,
  selectRepositoryInteractively,
  resolveWorkspaceAndRepositoryForSync,
  createSettingsForComparison,
  yamlSafe,
  yamlSafeForComparison,
  extractSyncOptions,
  extractRepositorySyncOptions,
  mergeBackendSettingsWithLocalCodebases,
  mergePreservingOrder,
  syncOptionsToUIState,
} from "./settings_utils.ts";
import { parse } from "jsr:@std/yaml@^1.0.5";

export interface SimplifiedSettings {
  // slack_team_id?: string;
  // slack_name?: string;
  // slack_command_script?: string;
  // slack_email?: string;
  auto_invite_enabled: boolean;
  auto_invite_as: string;
  auto_invite_mode: string;
  webhook?: string;
  deploy_to?: string;
  error_handler?: string;
  error_handler_extra_args?: any;
  error_handler_muted_on_cancel?: boolean;
  ai_config?: AIConfig;
  large_file_storage?: any;
  git_sync?: any;
  default_app?: string;
  default_scripts?: any;
  name: string;
  mute_critical_alerts?: boolean;
  color?: string;
  operator_settings?: any;
}

const INSTANCE_SETTINGS_PATH = "instance_settings.yaml";
let instanceSettingsPath = INSTANCE_SETTINGS_PATH;
async function checkInstanceSettingsPath(opts: InstanceSyncOptions) {
  if (opts.prefix && opts.folderPerInstance && opts.prefixSettings) {
    instanceSettingsPath = `${opts.prefix}/${INSTANCE_SETTINGS_PATH}`;
  }
}

const INSTANCE_CONFIGS_PATH = "instance_configs.yaml";
let instanceConfigsPath = INSTANCE_CONFIGS_PATH;
async function checkInstanceConfigPath(opts: InstanceSyncOptions) {
  if (opts.prefix && opts.folderPerInstance && opts.prefixSettings) {
    instanceConfigsPath = `${opts.prefix}/${INSTANCE_CONFIGS_PATH}`;
  }
}

export async function pushWorkspaceSettings(
  workspace: string,
  _path: string,
  settings: SimplifiedSettings | undefined,
  localSettings: SimplifiedSettings
) {
  try {
    const remoteSettings = await wmill.getSettings({
      workspace,
    });

    const workspaceName = await wmill.getWorkspaceName({
      workspace,
    });

    settings = {
      // slack_team_id: remoteSettings.slack_team_id,
      // slack_name: remoteSettings.slack_name,
      // slack_command_script: remoteSettings.slack_command_script,
      // slack_email: remoteSettings.slack_email,
      auto_invite_enabled: remoteSettings.auto_invite_domain !== null,
      auto_invite_as: remoteSettings.auto_invite_operator
        ? "operator"
        : "developer",
      auto_invite_mode: remoteSettings.auto_add ? "add" : "invite",
      webhook: remoteSettings.webhook,
      deploy_to: remoteSettings.deploy_to,
      error_handler: remoteSettings.error_handler,
      error_handler_extra_args: remoteSettings.error_handler_extra_args,
      error_handler_muted_on_cancel:
        remoteSettings.error_handler_muted_on_cancel,
      ai_config: remoteSettings.ai_config,
      large_file_storage: remoteSettings.large_file_storage,
      git_sync: remoteSettings.git_sync,
      default_app: remoteSettings.default_app,
      default_scripts: remoteSettings.default_scripts,
      name: workspaceName,
      mute_critical_alerts: remoteSettings.mute_critical_alerts,
      color: remoteSettings.color,
      operator_settings: remoteSettings.operator_settings,
    };
  } catch (err) {
    throw new Error(`Failed to get workspace settings: ${err}`);
  }

  if (isSuperset(localSettings, settings)) {
    log.debug(`Workspace settings are up to date`);
    return;
  }
  log.debug(`Workspace settings are not up-to-date, updating...`);
  if (localSettings.webhook !== settings.webhook) {
    log.debug(`Updateing webhook...`);
    await wmill.editWebhook({
      workspace,
      requestBody: {
        webhook: localSettings.webhook,
      },
    });
  }
  if (
    localSettings.auto_invite_as !== settings.auto_invite_as ||
    localSettings.auto_invite_enabled !== settings.auto_invite_enabled ||
    localSettings.auto_invite_mode !== settings.auto_invite_mode
  ) {
    log.debug(`Updating auto invite...`);

    if (!["operator", "developer"].includes(settings.auto_invite_as)) {
      throw new Error(
        `Invalid value for auto_invite_as. Valid values are "operator" and "developer"`
      );
    }

    if (!["add", "invite"].includes(settings.auto_invite_mode)) {
      throw new Error(
        `Invalid value for auto_invite_mode. Valid values are "invite" and "add"`
      );
    }
    try {
      await wmill.editAutoInvite({
        workspace,
        requestBody: localSettings.auto_invite_enabled
          ? {
              operator: localSettings.auto_invite_as === "operator",
              invite_all: true,
              auto_add: localSettings.auto_invite_mode === "add",
            }
          : {},
      });
    } catch (_) {
      // on cloud
      log.debug(
        `Auto invite is not possible on cloud, only auto-inviting same domain...`
      );
      await wmill.editAutoInvite({
        workspace,
        requestBody: localSettings.auto_invite_enabled
          ? {
              operator: localSettings.auto_invite_as === "operator",
              invite_all: false,
              auto_add: localSettings.auto_invite_mode === "add",
            }
          : {},
      });
    }
  }
  if (!deepEqual(localSettings.ai_config, settings.ai_config)) {
    log.debug(`Updating copilot settings...`);
    await wmill.editCopilotConfig({
      workspace,
      requestBody: localSettings.ai_config ?? {},
    });
  }
  if (
    localSettings.error_handler != settings.error_handler ||
    !deepEqual(
      localSettings.error_handler_extra_args,
      settings.error_handler_extra_args
    ) ||
    localSettings.error_handler_muted_on_cancel !=
      settings.error_handler_muted_on_cancel
  ) {
    log.debug(`Updating error handler...`);
    await wmill.editErrorHandler({
      workspace,
      requestBody: {
        error_handler: localSettings.error_handler,
        error_handler_extra_args: localSettings.error_handler_extra_args,
        error_handler_muted_on_cancel:
          localSettings.error_handler_muted_on_cancel,
      },
    });
  }
  if (localSettings.deploy_to != settings.deploy_to) {
    log.debug(`Updating deploy to...`);
    await wmill.editDeployTo({
      workspace,
      requestBody: {
        deploy_to: localSettings.deploy_to,
      },
    });
  }
  if (
    !deepEqual(localSettings.large_file_storage, settings.large_file_storage)
  ) {
    log.debug(`Updating large file storage...`);
    await wmill.editLargeFileStorageConfig({
      workspace,
      requestBody: {
        large_file_storage: localSettings.large_file_storage,
      },
    });
  }
  if (!deepEqual(localSettings.git_sync, settings.git_sync)) {
    log.debug(`Updating git sync...`);
    await wmill.editWorkspaceGitSyncConfig({
      workspace,
      requestBody: {
        git_sync_settings: localSettings.git_sync,
      },
    });
  }
  if (!deepEqual(localSettings.default_scripts, settings.default_scripts)) {
    log.debug(`Updating default scripts...`);
    await wmill.editDefaultScripts({
      workspace,
      requestBody: localSettings.default_scripts,
    });
  }
  if (localSettings.default_app != settings.default_app) {
    log.debug(`Updating default app...`);
    await wmill.editWorkspaceDefaultApp({
      workspace,
      requestBody: {
        default_app_path: localSettings.default_app,
      },
    });
  }

  if (localSettings.name != settings.name) {
    log.debug(`Updating workspace name...`);
    await wmill.changeWorkspaceName({
      workspace,
      requestBody: {
        new_name: localSettings.name,
      },
    });
  }

  if (localSettings.mute_critical_alerts != settings.mute_critical_alerts) {
    log.debug(`Updating mute critical alerts...`);
    await wmill.workspaceMuteCriticalAlertsUi({
      workspace,
      requestBody: {
        mute_critical_alerts: localSettings.mute_critical_alerts,
      },
    });
  }

  if (localSettings.color != settings.color) {
    log.debug(`Updating workspace color...`);
    await wmill.changeWorkspaceColor({
      workspace,
      requestBody: {
        color: localSettings.color,
      },
    });
  }

  if (localSettings.operator_settings != settings.operator_settings) {
    log.debug(`Updating operator settings...`);
    await wmill.updateOperatorSettings({
      workspace,
      requestBody: localSettings.operator_settings,
    });
  }
}

export async function pushWorkspaceKey(
  workspace: string,
  _path: string,
  key: string | undefined,
  localKey: string
) {
  try {
    key = await wmill
      .getWorkspaceEncryptionKey({
        workspace,
      })
      .then((r) => r.key);
  } catch (err) {
    throw new Error(`Failed to get workspace encryption key: ${err}`);
  }
  if (localKey && key !== localKey) {
    const confirm = await Confirm.prompt({
      message:
        "The local workspace encryption key does not match the remote. Do you want to reencrypt all your secrets on the remote with the new key?\nSay 'no' if your local secrets are already encrypted with the new key (e.g. workspace/instance migration)\nOtherwise, say 'yes' and pull the secrets after the reencryption.\n",
      default: true,
    });
    log.debug(`Updating workspace encryption key...`);
    await wmill.setWorkspaceEncryptionKey({
      workspace,
      requestBody: {
        new_key: localKey,
        skip_reencrypt: !confirm,
      },
    });
  } else {
    log.debug(`Workspace encryption key is up to date`);
  }
}

export async function readInstanceSettings(opts: InstanceSyncOptions) {
  let localSettings: GlobalSetting[] = [];

  await checkInstanceSettingsPath(opts);

  try {
    localSettings = (await yamlParseFile(
      instanceSettingsPath
    )) as GlobalSetting[];
  } catch {
    log.warn(`No ${instanceSettingsPath} found`);
  }
  return localSettings;
}

import { decrypt, encrypt } from "./local_encryption.ts";

const SENSITIVE_FIELD: string[] = ["license_key", "jwt_secret"];

async function processInstanceSettings(
  settings: GlobalSetting[],
  mode: "encode" | "decode"
): Promise<GlobalSetting[]> {
  const encKey = process.env.WMILL_INSTANCE_LOCAL_ENCRYPTION_KEY;
  if (encKey) {
    const res: GlobalSetting[] = [];

    for (const s of settings) {
      if (SENSITIVE_FIELD.includes(s.name) && typeof s.value === "string") {
        res.push(
          (await processField(s, "value", encKey, mode)) as GlobalSetting
        );
      } else if (s.name == "oauths") {
        if (typeof s.value === "object") {
          const oauths = s.value as { [key: string]: any };
          for (const [k, v] of Object.entries(oauths)) {
            oauths[k] = await processField(v, "secret", encKey, mode);
          }
          res.push(s);
        } else {
          log.warn(`Unexpected oauths value type: ${typeof s.value}`);
          res.push(s);
        }
      } else {
        res.push(s);
      }
    }
    return res;
  } else {
    log.warn(
      "No encryption key found, skipping encryption. Recommend setting WMILL_INSTANCE_LOCAL_ENCRYPTION_KEY"
    );
  }
  return settings;
}

async function processField(
  obj: { [key: string]: any },
  field: string,
  encKey: string,
  mode: "encode" | "decode"
): Promise<{ [key: string]: any }> {
  return {
    ...obj,
    [field]:
      mode === "encode"
        ? await encrypt(obj[field], encKey)
        : ((await decrypt(obj[field], encKey)) as any),
  };
}

export async function pullInstanceSettings(
  opts: InstanceSyncOptions,
  preview = false
) {
  const remoteSettings = await wmill.listGlobalSettings();

  await checkInstanceSettingsPath(opts);

  if (preview) {
    const localSettings: GlobalSetting[] = await readInstanceSettings(opts);
    const processedSettings = await processInstanceSettings(
      remoteSettings,
      "encode"
    );
    return compareInstanceObjects(
      processedSettings,
      localSettings,
      "name",
      "setting"
    );
  } else {
    log.info("Pulling settings from instance");

    const processedSettings = await processInstanceSettings(
      remoteSettings,
      "encode"
    );
    await Deno.writeTextFile(
      instanceSettingsPath,
      yamlSafe(processedSettings)
    );

    log.info(colors.green(`Settings written to ${instanceSettingsPath}`));
  }
}

export async function pushInstanceSettings(
  opts: InstanceSyncOptions,
  preview: boolean = false
) {
  const remoteSettings = await wmill.listGlobalSettings();
  let localSettings: GlobalSetting[] = await readInstanceSettings(opts);
  localSettings = await processInstanceSettings(localSettings, "decode");

  if (opts.baseUrl) {
    localSettings = localSettings.filter((s) => s.name !== "base_url");
    localSettings.push({
      name: "base_url",
      //@ts-ignore
      value: opts.baseUrl,
    });
  }

  if (preview) {
    return compareInstanceObjects(
      localSettings,
      remoteSettings,
      "name",
      "setting"
    );
  } else {
    for (const setting of localSettings) {
      const remoteMatch = remoteSettings.find((s) => s.name === setting.name);
      if (remoteMatch && deepEqual(remoteMatch, setting)) {
        continue;
      }
      try {
        await wmill.setGlobal({
          key: setting.name,
          requestBody: {
            value: setting.value,
          },
        });
      } catch (err) {
        log.error(`Failed to set setting ${setting.name}: ${err}`);
      }
    }

    for (const remoteSetting of remoteSettings) {
      const localMatch = localSettings.find(
        (s) => s.name === remoteSetting.name
      );
      if (!localMatch) {
        try {
          await wmill.setGlobal({
            key: remoteSetting.name,
            requestBody: {
              value: null,
            },
          });
        } catch (err) {
          log.error(`Failed to delete setting ${remoteSetting.name}: ${err}`);
        }
      }
    }

    log.info(colors.green("Settings pushed to instance"));
  }
}

export async function readLocalConfigs(opts: InstanceSyncOptions) {
  let localConfigs: Config[] = [];

  await checkInstanceConfigPath(opts);

  try {
    localConfigs = (await yamlParseFile(instanceConfigsPath)) as Config[];
  } catch {
    log.warn(`No ${instanceConfigsPath} found`);
  }
  return localConfigs;
}

export async function pullInstanceConfigs(
  opts: InstanceSyncOptions,
  preview = false
) {
  const remoteConfigs = (await wmill.listConfigs()).map((x) => {
    return {
      ...x,
      name: removeWorkerPrefix(x.name),
    };
  });

  if (preview) {
    const localConfigs: Config[] = await readLocalConfigs(opts);

    return compareInstanceObjects(
      remoteConfigs,
      localConfigs,
      "name",
      "config"
    );
  } else {
    log.info("Pulling configs from instance");

    await Deno.writeTextFile(
      instanceConfigsPath,
      yamlSafe(remoteConfigs as any)
    );

    log.info(colors.green(`Configs written to ${instanceConfigsPath}`));
  }
}

export async function pushInstanceConfigs(
  opts: InstanceSyncOptions,
  preview: boolean = false
) {
  const remoteConfigs = (await wmill.listConfigs()).map((x) => {
    return {
      ...x,
      name: removeWorkerPrefix(x.name),
    };
  });
  const localConfigs = await readLocalConfigs(opts);

  if (preview) {
    return compareInstanceObjects(
      localConfigs,
      remoteConfigs,
      "name",
      "config"
    );
  } else {
    log.info("Pushing configs to instance");
    for (const config of localConfigs) {
      const remoteMatch = remoteConfigs.find((c) => c.name === config.name);
      if (remoteMatch && deepEqual(remoteMatch, config)) {
        continue;
      }
      try {
        await wmill.updateConfig({
          name: config.name.startsWith("worker__")
            ? config.name
            : `worker__${config.name}`,
          requestBody: config.config,
        });
      } catch (err) {
        log.error(`Failed to set config ${config.name}: ${err}`);
      }
    }

    for (const removeConfig of remoteConfigs) {
      const localMatch = localConfigs.find((c) => c.name === removeConfig.name);

      if (!localMatch) {
        try {
          await wmill.deleteConfig({
            name: removeConfig.name,
          });
        } catch (err) {
          log.error(`Failed to delete config ${removeConfig.name}: ${err}`);
        }
      }
    }

    log.info(colors.green("Configs pushed to instance"));
  }
}

// Interface for structured responses
interface SettingsResult {
  success: boolean;
  settings?: SyncOptions;
  yaml?: string;
  diff?: string;
  message?: string;
  error?: string;
}

// Import shared utility functions
export type { UIState } from "./settings_utils.ts";
export { uiStateToSyncOptions, syncOptionsToUIState, parseJsonInput } from "./settings_utils.ts";

// ========== SHARED HELPER FUNCTIONS ==========

// Helper to handle consistent error responses
function handleError(error: unknown): SettingsResult {
  return {
    success: false,
    error: (error as Error).message || "Operation failed"
  };
}

// Shared workspace resolution function to eliminate duplication
async function resolveWorkspaceForSettings(
  opts: GlobalOptions & SyncOptions & {
    repository?: string;
    workspace?: string;
  },
  config?: SyncOptions
): Promise<{
  workspace: Workspace;
  repositoryPath: string | undefined;
  workspaceName: string | undefined;
  mergedOpts: GlobalOptions & SyncOptions;
}> {
  let workspace: Workspace;
  let repositoryPath: string | undefined;
  let workspaceName: string | undefined;

  if (opts.workspace && opts.repository) {
    workspace = await resolveWorkspace(opts);
    workspaceName = opts.workspace;
    repositoryPath = opts.repository;
    const mergedOpts = { ...(config || {}), ...opts };
    return { workspace, repositoryPath, workspaceName, mergedOpts };
  }

  try {
    // Try to resolve using workspace-aware method
    const { workspaceName: resolvedWorkspace, workspaceProfile, repositoryPath: resolvedRepo, syncOptions } =
      await resolveWorkspaceAndRepositoryForSync(opts.workspace, opts.repository, config);

    if (workspaceProfile) {
      // Use workspace profile to create workspace object
      workspace = {
        workspaceId: workspaceProfile.workspaceId,
        name: resolvedWorkspace || '',
        remote: workspaceProfile.baseUrl,
        token: opts.token || '' // Keep existing token resolution
      };
      workspaceName = resolvedWorkspace;
    } else {
      // Fallback to normal workspace resolution
      workspace = await resolveWorkspace(opts);
    }

    repositoryPath = resolvedRepo;
    // Merge resolved repository options with command line options
    const mergedOpts = { ...syncOptions, ...opts };

    return { workspace, repositoryPath, workspaceName, mergedOpts };
  } catch (error) {
    // Fall back to legacy resolution
    workspace = await resolveWorkspace(opts);
    workspaceName = workspace.name;
    repositoryPath = opts.repository; // Legacy: just use specified repository

    // If no explicit repository but we have a workspace, try to resolve from backend
    if (!repositoryPath && workspaceName) {
      try {
        const repositories = await listRepositories(workspace);
        if (repositories.length === 1) {
          // If exactly one repository, use it
          repositoryPath = repositories[0].display_path;
        } else if (repositories.length > 1) {
          // Multiple repositories - would need interactive selection for true CLI usage
          // For now, just use first one to enable multi-workspace format
          repositoryPath = repositories[0].display_path;
        }
      } catch {
        // If backend call fails, just continue with undefined repositoryPath
      }
    }

    const mergedOpts = { ...(config || {}), ...opts };

    return { workspace, repositoryPath, workspaceName, mergedOpts };
  }
}

// Helper function to run diff command cross-platform
async function runDiffCommand(args: string[]): Promise<{ stdout: string; stderr: string; success: boolean }> {
  try {
    if (args.length >= 2) {
      const [file1Path, file2Path] = args.slice(-2);

      try {
        const content1 = await Deno.readTextFile(file1Path);
        const content2 = await Deno.readTextFile(file2Path);

        const { createTwoFilesPatch } = await import("npm:diff");
        const patch = createTwoFilesPatch(
          file1Path,
          file2Path,
          content1,
          content2,
          '',
          '',
          { context: 3 }
        );

        const hasDifferences = content1 !== content2;

        return {
          stdout: hasDifferences ? patch : '',
          stderr: '',
          success: true
        };
      } catch (fileError) {
        return {
          stdout: '',
          stderr: `Failed to read files: ${(fileError as Error).message}`,
          success: false
        };
      }
    } else {
      return {
        stdout: '',
        stderr: 'Invalid arguments for diff command',
        success: false
      };
    }
  } catch (error) {
    return {
      stdout: '',
      stderr: `diff command failed: ${(error as Error).message}`,
      success: false
    };
  }
}

// Helper function to fetch backend settings
export async function fetchBackendSettings(workspace: { workspaceId: string }, repositoryPath?: string): Promise<SyncOptions> {
  const backendSettings = await wmill.getSettings({ workspace: workspace.workspaceId });

  if (backendSettings.git_sync?.repositories && backendSettings.git_sync.repositories.length > 0) {
    let targetRepo;

    if (repositoryPath) {
      // Try both normalized and display formats to handle different backend storage formats
      const normalizedPath = normalizeRepositoryPath(repositoryPath);
      const displayPath = displayRepositoryPath(repositoryPath);


      // Find specific repository by git_repo_resource_path
      targetRepo = backendSettings.git_sync.repositories.find(
        repo => repo.git_repo_resource_path === normalizedPath || repo.git_repo_resource_path === displayPath
      );
      if (!targetRepo) {
        const availableRepos = backendSettings.git_sync.repositories
          .map(r => displayRepositoryPath(r.git_repo_resource_path))
          .join(', ');
        throw new Error(`Repository not found: ${repositoryPath}. Available repositories: ${availableRepos}`);
      }
    } else {
      // Handle multiple repositories - show interactive selection or error
      if (backendSettings.git_sync.repositories.length > 1) {
        const repoOptions = backendSettings.git_sync.repositories.map(repo => ({
          git_repo_resource_path: repo.git_repo_resource_path,
          display_path: displayRepositoryPath(repo.git_repo_resource_path)
        }));

        const selectedRepoPath = await selectRepositoryInteractively(repoOptions, "work with");
        const normalizedSelectedPath = normalizeRepositoryPath(selectedRepoPath);

        targetRepo = backendSettings.git_sync.repositories.find(
          repo => repo.git_repo_resource_path === normalizedSelectedPath
        );

        if (!targetRepo) {
          throw new Error(`Repository not found: ${selectedRepoPath}`);
        }
      } else {
        // Use first repository as default
        targetRepo = backendSettings.git_sync.repositories[0];
      }
    }

    const repoSettings = targetRepo.settings || {};

    // Convert backend repository settings to SyncOptions format using existing utility
    const uiState = {
      include_path: repoSettings.include_path || ['f/**'],
      include_type: repoSettings.include_type || ['script', 'flow', 'app', 'folder']
    };
    const syncOptions = uiStateToSyncOptions(uiState);
    syncOptions.excludes = repoSettings.exclude_path || [];
    syncOptions.extraIncludes = repoSettings.extra_include_path || [];
    return syncOptions;
  } else {
    return { ...DEFAULT_SYNC_OPTIONS };
  }
}

// Helper function to list all repositories in workspace
export async function listRepositories(workspace: { workspaceId: string }): Promise<Array<{
  git_repo_resource_path: string;
  display_path: string;
  script_path: string;
  group_by_folder: boolean;
  use_individual_branch: boolean;
  settings?: any;
}>> {
  const backendSettings = await wmill.getSettings({ workspace: workspace.workspaceId });

  if (backendSettings.git_sync?.repositories && backendSettings.git_sync.repositories.length > 0) {
    return backendSettings.git_sync.repositories.map(repo => ({
      git_repo_resource_path: repo.git_repo_resource_path,
      display_path: displayRepositoryPath(repo.git_repo_resource_path),
      script_path: repo.script_path,
      group_by_folder: repo.group_by_folder ?? false,
      use_individual_branch: repo.use_individual_branch ?? false,
      settings: repo.settings
    }));
  }

  return [];
}

// Helper function to generate diff between two settings
async function generateDiff(fromFile: string, toFile: string): Promise<string> {
  const diffResult = await runDiffCommand([fromFile, toFile]);
  return diffResult.success ? diffResult.stdout : `Failed to compare: ${diffResult.stderr}`;
}

// Helper function to read local settings file
async function readLocalSettingsFile(filePath: string = 'wmill.yaml'): Promise<{ content: string; settings: SyncOptions }> {
  try {
    const content = await Deno.readTextFile(filePath);
    const fullConfig = parse(content) as SyncOptions;

    // For legacy format, just return the full config (no repository awareness)
    return { content, settings: fullConfig };
  } catch (error) {
    throw new Error(`Could not read ${filePath}: ${(error as Error).message}`);
  }
}

// Helper function to create temp files for diff
async function createTempDiffFiles(content1: string, content2: string): Promise<{ file1: string; file2: string }> {
  const file1 = await Deno.makeTempFile({ suffix: '.yaml' });
  const file2 = await Deno.makeTempFile({ suffix: '.yaml' });

  await Deno.writeTextFile(file1, content1);
  await Deno.writeTextFile(file2, content2);

  return { file1, file2 };
}

// Helper function to write settings to a specific workspace repository
async function writeWorkspaceRepositorySettings(
  workspaceName: string,
  repositoryPath: string,
  newSettings: SyncOptions,
  filePath: string = 'wmill.yaml',
  workspace?: Workspace,
  providedConfig?: SyncOptions
): Promise<void> {
  // Use provided config or read current config, or create new one if file doesn't exist
  let currentConfig;
  if (providedConfig) {
    currentConfig = providedConfig;
  } else {
    try {
      currentConfig = await readConfigFile();
    } catch (error) {
      // If file doesn't exist or can't be read, create a new multi-workspace config
      currentConfig = {
        defaultWorkspace: workspaceName,
        workspaces: {}
      };
    }
  }

  // Ensure workspaces object exists
  if (!currentConfig.workspaces) {
    currentConfig.workspaces = {};
  }

  // Create workspace if it doesn't exist
  if (!currentConfig.workspaces[workspaceName]) {
    currentConfig.workspaces[workspaceName] = {
      baseUrl: workspace?.remote || "", // Use actual base URL if available
      workspaceId: workspaceName,
      repositories: {}
    };
  }

  // Set default workspace if not set
  if (!currentConfig.defaultWorkspace) {
    currentConfig.defaultWorkspace = workspaceName;
  }

  // Ensure repositories object exists for this workspace
  if (!currentConfig.workspaces[workspaceName].repositories) {
    currentConfig.workspaces[workspaceName].repositories = {};
  }

  // Use all settings including local-only fields (defaultTs, codebases) for local config
  const repositorySettings = { ...newSettings } as any;

  // Remove undefined values to keep the config clean
  Object.keys(repositorySettings).forEach(key => {
    if (repositorySettings[key] === undefined) {
      delete repositorySettings[key];
    }
  });

  // Update the specific repository settings
  currentConfig.workspaces[workspaceName].repositories![repositoryPath] = repositorySettings;

  let existingObj: any = undefined;
  try {
    existingObj = (await yamlParseFile(filePath)) as any;
  } catch {
    // file may not exist or parse error; treat as different
  }
  const mergedConfig = mergePreservingOrder(existingObj || {}, currentConfig as any, false);

  const existingYaml = yamlSafe(existingObj || {});
  const targetYaml = yamlSafe(mergedConfig as any);
  if (existingYaml.trim() !== targetYaml.trim()) {
    await Deno.writeTextFile(filePath, targetYaml);
  }
}

// Helper function to write settings to config file (legacy format)
async function writeSettings(settings: SyncOptions, filePath: string = 'wmill.yaml'): Promise<void> {
  let existingLegacy: any = undefined;
  try {
    existingLegacy = (await yamlParseFile(filePath)) as any;
  } catch {
    // ignore
  }
  const mergedLegacy = mergePreservingOrder(existingLegacy || {}, settings as any, false);

  const existingLegacyYaml = yamlSafe(existingLegacy || {});
  const targetLegacyYaml = yamlSafe(mergedLegacy as any);
  if (existingLegacyYaml.trim() !== targetLegacyYaml.trim()) {
    await Deno.writeTextFile(filePath, targetLegacyYaml);
  }
}

// Helper function to update git sync repositories with new settings
async function updateGitSyncRepositories(workspace: { workspaceId: string }, localSettings: SyncOptions, repositoryPath?: string): Promise<void> {
  const backendSettings = await wmill.getSettings({ workspace: workspace.workspaceId });

  if (backendSettings.git_sync?.repositories && backendSettings.git_sync.repositories.length > 0) {
    let normalizedRepositoryPath: string | undefined;
    if (repositoryPath) {
      normalizedRepositoryPath = normalizeRepositoryPath(repositoryPath);
    }

    // If no repository specified and multiple repositories exist, require explicit specification
    if (!normalizedRepositoryPath && backendSettings.git_sync.repositories.length > 1) {
      const availableRepos = backendSettings.git_sync.repositories
        .map(r => displayRepositoryPath(r.git_repo_resource_path))
        .join(', ');
      throw new Error(`Multiple repositories found in workspace. Please specify which repository to update using --repository. Available repositories: ${availableRepos}`);
    }

    const repositories = backendSettings.git_sync.repositories.map((repo) => {
            const shouldUpdate = normalizedRepositoryPath ?
        (repo.git_repo_resource_path === normalizedRepositoryPath || repo.git_repo_resource_path === displayRepositoryPath(normalizedRepositoryPath)) :
        backendSettings.git_sync!.repositories!.length === 1; // Only update first repo if there's exactly one

      if (shouldUpdate) {
        return {
          script_path: repo.script_path,
          git_repo_resource_path: repo.git_repo_resource_path,
          use_individual_branch: repo.use_individual_branch ?? false,
          group_by_folder: repo.group_by_folder ?? false,
          collapsed: false,
          settings: (() => {
            // Convert SyncOptions to backend format using existing utility
            const uiState = syncOptionsToUIState(localSettings);
            return {
              include_path: uiState.include_path,
              exclude_path: localSettings.excludes && localSettings.excludes.length > 0 ? localSettings.excludes : [],
              extra_include_path: localSettings.extraIncludes && localSettings.extraIncludes.length > 0 ? localSettings.extraIncludes : [],
              include_type: uiState.include_type as ('script' | 'flow' | 'app' | 'folder' | 'variable' | 'resource' | 'resourcetype' | 'secret' | 'schedule' | 'trigger' | 'user' | 'group' | 'settings' | 'key')[]
            };
          })()
        };
      }
      return repo;
    });

    if (normalizedRepositoryPath && !repositories.find(r => r.git_repo_resource_path === normalizedRepositoryPath || r.git_repo_resource_path === displayRepositoryPath(normalizedRepositoryPath))) {
      const availableRepos = backendSettings.git_sync.repositories
        .map(r => displayRepositoryPath(r.git_repo_resource_path))
        .join(', ');
      throw new Error(`Repository not found: ${repositoryPath}. Available repositories: ${availableRepos}`);
    }

    await wmill.editWorkspaceGitSyncConfig({
      workspace: workspace.workspaceId,
      requestBody: {
        git_sync_settings: {
          repositories
        }
      }
    });
  } else {
    throw new Error("No git sync repositories found in workspace. Please configure a git repository first through the UI.");
  }
}

// Helper function to handle fromJson processing (eliminating duplication between pull/push)
async function handleFromJsonProcessing(
  opts: { fromJson?: string; diff?: boolean; dryRun?: boolean },
  operationType: 'pull' | 'push',
  workspaceName?: string,
  repositoryPath?: string,
  workspace?: Workspace,
  config?: SyncOptions
): Promise<SettingsResult | null> {
  if (!opts.fromJson) return null;

  // Handle JSON input mode - JSON represents simulated backend state
  let uiState;
  try {
    uiState = parseJsonInput(opts.fromJson);
  } catch (error) {
    return {
      success: false,
      error: (error as Error).message
    };
  }
  let jsonSyncOptions = uiStateToSyncOptions(uiState);
  // Strip redundant defaults so behaviour matches backend
  jsonSyncOptions = createSettingsForComparison(jsonSyncOptions);

    if (opts.diff) {
    // For --from-json --diff: Compare JSON (simulated backend) with local wmill.yaml file
    let actualLocalSettings: SyncOptions = {};
    let hasLocalFile = true;

    try {
      // Check if file exists first
      await Deno.stat('wmill.yaml');
      const result = await readLocalSettingsFile();
      actualLocalSettings = result.settings;
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        // File doesn't exist - this is fine for new repositories
        hasLocalFile = false;
        actualLocalSettings = {};
      } else {
        // Some other error (permissions, invalid YAML, etc.)
        return {
          success: false,
          error: `Error reading wmill.yaml: ${(error as Error).message}`
        };
      }
    }

    if (!hasLocalFile) {
      // No local file - show all JSON settings as additions
      const jsonYaml = yamlSafeForComparison(jsonSyncOptions as Record<string, unknown>);
      const emptyYaml = '';

      const { file1: tmpEmptyFile, file2: tmpJsonFile } = await createTempDiffFiles(emptyYaml, jsonYaml);
      const diff = await generateDiff(tmpEmptyFile, tmpJsonFile);

      const message = operationType === 'pull'
        ? "New wmill.yaml file would be created from JSON input"
        : "Diff showing what would be pushed from JSON input (new repository)";

      return {
        success: true,
        yaml: jsonYaml,
        settings: jsonSyncOptions,
        diff: diff,
        message
      };
    } else {
      // Local file exists - normal diff
      const cleanedLocal = createSettingsForComparison(actualLocalSettings);
      const backendForDiff = createSettingsForComparison(jsonSyncOptions);
      const simulatedBackendYaml = yamlSafeForComparison(backendForDiff as Record<string, unknown>);
      const localYaml = yamlSafeForComparison(cleanedLocal as Record<string, unknown>);

      const { file1: tmpSimulatedBackendFile, file2: tmpLocalFile } = await createTempDiffFiles(simulatedBackendYaml, localYaml);
      let diff = '';
      if (operationType === 'pull') {
        diff = await generateDiff(tmpLocalFile, tmpSimulatedBackendFile);
      } else {
        diff = await generateDiff(tmpSimulatedBackendFile, tmpLocalFile);
      }

      const message = operationType === 'pull'
        ? "Diff between local file and JSON input (what local would become)"
        : "Diff between local file and JSON input (what backend would become)";

      return {
        success: true,
        yaml: localYaml,
        settings: actualLocalSettings,
        diff: diff,
        message
      };
    }
  }

      // For non-diff operations, use JSON as the settings
    const yamlContent = yamlSafe(jsonSyncOptions as Record<string, unknown>);

  if (opts.dryRun) {
    const message = operationType === 'pull'
      ? "Dry run - showing what would be written to wmill.yaml from JSON input"
      : "Dry run - showing what would be pushed from JSON input";

    return {
      success: true,
      yaml: yamlContent,
      settings: jsonSyncOptions,
      message
    };
  }

  // For actual operations
  if (operationType === 'pull') {
    // Use multi-workspace format if we have workspace and repository info
    if (workspaceName && repositoryPath) {
      await writeWorkspaceRepositorySettings(workspaceName, repositoryPath, jsonSyncOptions, 'wmill.yaml', workspace, config);
      return {
        success: true,
        yaml: yamlContent,
        settings: jsonSyncOptions,
        message: "Settings written to multi-workspace format from JSON input"
      };
    } else {
      // Legacy format
      let existingObj: any = undefined;
      try {
        existingObj = (await yamlParseFile('wmill.yaml')) as any;
      } catch {
        // ignore
      }
      const merged = mergePreservingOrder(existingObj || {}, jsonSyncOptions as any, false);
      const existingYaml = yamlSafe(existingObj || {});
      const mergedYaml = yamlSafe(merged as Record<string, unknown>);
      if (existingYaml.trim() !== mergedYaml.trim()) {
        await Deno.writeTextFile('wmill.yaml', mergedYaml);
      }
      return {
        success: true,
        yaml: yamlContent,
        settings: jsonSyncOptions,
        message: "Settings written to wmill.yaml from JSON input"
      };
    }
  } else {
    // Push operation requires workspace parameter - will be handled by caller
    return {
      success: true,
      yaml: yamlContent,
      settings: jsonSyncOptions,
      message: "Settings ready to push from JSON input"
    };
  }
}

// ========== CORE FUNCTIONS ==========

export async function pullSettings(opts: GlobalOptions & SyncOptions & {
  format?: 'json' | 'yaml';
  diff?: boolean;
  fromJson?: string;
  repository?: string;
  workspace?: string;
}): Promise<SettingsResult> {
  try {
    // Load config once at the beginning
    const config = await readConfigFile();
    opts = { ...config, ...opts };

    // Ensure authentication before workspace resolution (which may need to call APIs)
    await requireLogin(opts);

    const { workspace, repositoryPath, workspaceName, mergedOpts } = await resolveWorkspaceForSettings(opts, config);
    opts = mergedOpts;

    let currentSettings: SyncOptions;
    let yamlContent: string;

    if (opts.fromJson) {
      const result = await handleFromJsonProcessing(opts, 'pull', workspaceName, repositoryPath, workspace, config);
      if (result) {
        if (!result.success) {
          return result;
        }
        // If handleFromJsonProcessing handled diff/dryRun, return immediately
        if (opts.diff || opts.dryRun) {
          return result;
        }
        currentSettings = result.settings!;
        yamlContent = result.yaml!;
      } else {
        // This shouldn't happen since we checked opts.fromJson, but handle it
        throw new Error("Failed to process JSON input");
      }
    } else {
      // Original backend pulling logic
      currentSettings = await fetchBackendSettings(workspace, repositoryPath);
      yamlContent = yamlSafe(currentSettings as Record<string, unknown>);
    }

    if (opts.diff) {
      // Compare backend with local repository-specific settings within workspace scope
      let currentLocalSettings: SyncOptions = {};
      let hasLocalSettings = true;

      try {
        if (workspaceName && repositoryPath) {
          // Get repository-specific settings from workspace profile
          const { getWorkspaceRepositorySettings } = await import("./conf.ts");
          const fullSettings = getWorkspaceRepositorySettings(config || {}, workspaceName, repositoryPath);
          currentLocalSettings = extractSyncOptions(fullSettings);
          // Preserve original local settings including local-only fields for comparison
          currentLocalSettings.codebases = fullSettings.codebases;
          currentLocalSettings.defaultTs = fullSettings.defaultTs;
        } else {
          // Legacy format - check if file exists first
          await Deno.stat('wmill.yaml');
          const { settings } = await readLocalSettingsFile('wmill.yaml');
          currentLocalSettings = extractSyncOptions(settings);
          // Preserve original local settings including local-only fields for comparison
          currentLocalSettings.codebases = settings.codebases;
          currentLocalSettings.defaultTs = settings.defaultTs;
        }
      } catch (error) {
        if (error instanceof Deno.errors.NotFound) {
          // No local settings file found - this is fine for new repositories
          hasLocalSettings = false;
          currentLocalSettings = {};
        } else {
          // Some other error reading settings
          return {
            success: false,
            error: `Error reading local settings: ${(error as Error).message}`
          };
        }
      }

      // Create repository-scoped semantic comparison
      let diff: string;
      let mergedBackendSettings: SyncOptions;
      let mergedBackendYaml: string;

      if (!hasLocalSettings) {
        // No local settings - show all backend settings as additions
        const cleanedBackendSettings = createSettingsForComparison(currentSettings);
        const backendYaml = yamlSafeForComparison(cleanedBackendSettings);
        const emptyYaml = '';

        const { file1: tmpEmptyFile, file2: tmpBackendFile } = await createTempDiffFiles(emptyYaml, backendYaml);
        const diffResult = await runDiffCommand([tmpEmptyFile, tmpBackendFile]);
        diff = diffResult.success ? diffResult.stdout : `Failed to compare: ${diffResult.stderr}`;

        mergedBackendSettings = currentSettings;
        mergedBackendYaml = backendYaml;
      } else {
        // Local settings exist - semantic repository-scoped diff
        // Both are already repository-scoped, just clean them for semantic comparison
        const cleanedLocalSettings = createSettingsForComparison(currentLocalSettings);
        const cleanedBackendSettings = createSettingsForComparison(currentSettings);

                // Use consistent field ordering for semantic comparison
        const localComparisonYaml = yamlSafeForComparison(cleanedLocalSettings);
        const backendComparisonYaml = yamlSafeForComparison(cleanedBackendSettings);

        const { file1: tmpLocalFile, file2: tmpBackendFile } = await createTempDiffFiles(localComparisonYaml, backendComparisonYaml);
        const diffResult = await runDiffCommand([tmpLocalFile, tmpBackendFile]);
        diff = diffResult.success ? diffResult.stdout : `Failed to compare: ${diffResult.stderr}`;

        // For the result, merge backend settings with local codebases but don't include in diff
        mergedBackendSettings = mergeBackendSettingsWithLocalCodebases(currentSettings, currentLocalSettings);
        mergedBackendYaml = yamlSafe(mergedBackendSettings as Record<string, unknown>);
      }

      const scopeMessage = workspaceName && repositoryPath
        ? `workspace '${workspaceName}' repository '${repositoryPath}'`
        : repositoryPath
        ? `repository '${repositoryPath}'`
        : "settings";

      const diffMessage = !hasLocalSettings
        ? `New ${scopeMessage} would be created from backend settings`
        : `Diff between local ${scopeMessage} settings and backend settings`;

      return {
        success: true,
        yaml: mergedBackendYaml,
        settings: mergedBackendSettings,
        diff: diff,
        message: diffMessage
      };
    }

    if (opts.dryRun) {
      // For dry run, show repository-scoped semantic diff only
      let settingsToWrite = currentSettings;
      let hasExistingSettings = true;

      // Get current repository-specific settings
      let existingRepositorySettings: SyncOptions = {};
      try {
        if (workspaceName && repositoryPath) {
          const { getWorkspaceRepositorySettings } = await import("./conf.ts");
          const fullSettings = getWorkspaceRepositorySettings(config || {}, workspaceName, repositoryPath);
          // Extract only repository-relevant settings, not the full config
          existingRepositorySettings = extractSyncOptions(fullSettings);
        } else {
          // Legacy format - check if file exists first
          await Deno.stat('wmill.yaml');
          const { settings } = await readLocalSettingsFile('wmill.yaml');
          existingRepositorySettings = extractSyncOptions(settings);
        }

        // Merge backend settings with existing local codebases for display
        settingsToWrite = mergeBackendSettingsWithLocalCodebases(currentSettings, existingRepositorySettings);
      } catch (error) {
        if (error instanceof Deno.errors.NotFound) {
          hasExistingSettings = false;
          existingRepositorySettings = {};
          settingsToWrite = currentSettings;
        } else {
          // Some other error reading settings
          return {
            success: false,
            error: `Error reading local settings for dry run: ${(error as Error).message}`
          };
        }
      }

      // Create semantic repository-scoped comparison
      const cleanedLocalSettings = createSettingsForComparison(existingRepositorySettings);
      const cleanedNewSettings = createSettingsForComparison(settingsToWrite);

      const localYaml = yamlSafeForComparison(cleanedLocalSettings);
      const newYaml = yamlSafeForComparison(cleanedNewSettings);

      const { file1: tmpLocalFile, file2: tmpNewFile } = await createTempDiffFiles(localYaml, newYaml);
      const diffResult = await runDiffCommand([tmpLocalFile, tmpNewFile]);
      const diff = diffResult.success ? diffResult.stdout : `Failed to compare: ${diffResult.stderr}`;

      const scopeMessage = workspaceName && repositoryPath
        ? `workspace '${workspaceName}' repository '${repositoryPath}'`
        : repositoryPath
        ? `repository '${repositoryPath}'`
        : "settings";

      const dryRunMessage = hasExistingSettings
        ? `Dry run - showing semantic changes for ${scopeMessage}`
        : `Dry run - showing new ${scopeMessage} that would be created`;

      return {
        success: true,
        yaml: newYaml,
        settings: settingsToWrite,
        diff: diff,
        message: dryRunMessage
      };
    }

    // Write settings to config file - preserve local codebases
    let settingsToWrite = currentSettings;

    // Get existing local settings to preserve codebases
    try {
      let existingLocalSettings: SyncOptions;
      if (workspaceName && repositoryPath) {
        const { getWorkspaceRepositorySettings } = await import("./conf.ts");
        existingLocalSettings = getWorkspaceRepositorySettings(config || {}, workspaceName, repositoryPath);
      } else {
        const { settings } = await readLocalSettingsFile('wmill.yaml');
        existingLocalSettings = settings;
      }

      // Merge backend settings with existing local codebases
      settingsToWrite = mergeBackendSettingsWithLocalCodebases(currentSettings, existingLocalSettings);
    } catch {
      // If no existing settings, use backend settings as-is (will have empty codebases)
      settingsToWrite = currentSettings;
    }

    if (workspaceName && repositoryPath) {
      // Multi-workspace format: Update workspace profile
      await writeWorkspaceRepositorySettings(workspaceName, repositoryPath, settingsToWrite, 'wmill.yaml', workspace, config);
    } else {
      // Legacy format - write directly to wmill.yaml
      await writeSettings(settingsToWrite);
    }

    return {
      success: true,
      yaml: yamlSafe(settingsToWrite as Record<string, unknown>),
      settings: settingsToWrite,
      message: repositoryPath
        ? `Settings pulled for repository '${repositoryPath}'`
        : "Settings pulled successfully"
    };
  } catch (error) {
    return handleError(error);
  }
}

export async function pushSettings(opts: GlobalOptions & SyncOptions & {
  format?: 'json' | 'yaml';
  settingsData?: SyncOptions;
  settingsFile?: string;
  json?: boolean;
  diff?: boolean;
  fromJson?: string;
  repository?: string;
  workspace?: string;
}): Promise<SettingsResult> {
  try {
    // Load config once at the beginning
    const config = await readConfigFile();
    opts = { ...config, ...opts };

    // Ensure authentication before workspace resolution (which may need to call APIs)
    await requireLogin(opts);

    const { workspace, repositoryPath, workspaceName, mergedOpts } = await resolveWorkspaceForSettings(opts, config);
    opts = mergedOpts;

    let localSettings: SyncOptions;

    if (opts.fromJson) {
      const result = await handleFromJsonProcessing(opts, 'push', workspaceName, repositoryPath, workspace, config);
      if (result) {
        if (!result.success) {
          return result;
        }
        // If handleFromJsonProcessing handled diff/dryRun, return immediately
        if (opts.diff || opts.dryRun) {
          return result;
        }
        localSettings = result.settings!;

        const yamlContent = yamlSafe(localSettings as Record<string, unknown>);

        if (opts.dryRun) {
          return {
            success: true,
            yaml: yamlContent,
            settings: localSettings,
            message: "Dry run - showing what would be pushed from JSON input"
          };
        }

        await updateGitSyncRepositories(workspace, localSettings, repositoryPath);
        return {
          success: true,
          message: "Settings successfully pushed to workspace backend from JSON input"
        };
      } else {
        // This shouldn't happen since we checked opts.fromJson, but handle it
        throw new Error("Failed to process JSON input");
      }
    }

    // Handle regular file/data input
    if (opts.settingsData) {
      localSettings = opts.settingsData;
    } else if (opts.settingsFile) {
      const fileContent = await Deno.readTextFile(opts.settingsFile);
      if (opts.format === 'json') {
        localSettings = JSON.parse(fileContent);
      } else {
        localSettings = parse(fileContent) as SyncOptions;
      }
    } else {
      try {
        // Get repository-specific settings from workspace scope if applicable
        if (workspaceName && repositoryPath) {
          const { getWorkspaceRepositorySettings } = await import("./conf.ts");
          const fullSettings = getWorkspaceRepositorySettings(config || {}, workspaceName, repositoryPath);
          localSettings = extractSyncOptions(fullSettings);
          // Preserve original local settings including codebases for display/writing
          localSettings.codebases = fullSettings.codebases;
        } else {
          // Legacy format - check if file exists first
          await Deno.stat('wmill.yaml');
          const { settings } = await readLocalSettingsFile();
          localSettings = extractSyncOptions(settings);
          // Preserve original local settings including codebases for display/writing
          localSettings.codebases = settings.codebases;
        }
      } catch (error) {
        if (error instanceof Deno.errors.NotFound) {
          // For new repositories, there might not be a wmill.yaml file yet
          return {
            success: false,
            error: "wmill.yaml file not found. For new repositories, use --from-json with settings data, or --file to specify a different file."
          };
        } else {
          // Some other error (permissions, invalid YAML, etc.)
          return {
            success: false,
            error: `Error reading wmill.yaml: ${(error as Error).message}`
          };
        }
      }
    }

    if (opts.diff) {
      // Compare local settings with backend settings (excluding codebases since they are local-only)
      const backendSyncOptions = await fetchBackendSettings(workspace, repositoryPath);

      // Create comparison versions without codebases (since codebases are local-only)
      const localForComparison = createSettingsForComparison(localSettings);
      const backendForComparison = createSettingsForComparison(backendSyncOptions);

      const localComparisonYaml = yamlSafeForComparison(localForComparison as Record<string, unknown>);
      const backendComparisonYaml = yamlSafeForComparison(backendForComparison as Record<string, unknown>);

      const { file1: tmpBackendFile, file2: tmpLocalFile } = await createTempDiffFiles(backendComparisonYaml, localComparisonYaml);
      const diff = await generateDiff(tmpBackendFile, tmpLocalFile);

      const scopeMessage = workspaceName && repositoryPath
        ? `workspace '${workspaceName}' repository '${repositoryPath}'`
        : repositoryPath
        ? `repository '${repositoryPath}'`
        : "settings";

      return {
        success: true,
        yaml: yamlSafe(localSettings as Record<string, unknown>),
        settings: localSettings,
        diff: diff,
        message: `Diff between backend settings and local ${scopeMessage} (codebases excluded from comparison as they are local-only)`
      };
    }

    if (opts.dryRun) {
      const localYaml = yamlSafe(localSettings as Record<string, unknown>);

      return {
        success: true,
        yaml: localYaml,
        settings: localSettings,
        message: "Dry run - showing what would be pushed to backend"
      };
    }

    await updateGitSyncRepositories(workspace, localSettings, repositoryPath);
    return {
      success: true,
      message: "Settings successfully pushed to workspace backend"
    };
  } catch (error) {
    return handleError(error);
  }
}

// CLI command structure
const settingsCommand = new Command()
  .description("Manage workspace settings")


  .command("pull")
  .description("Pull workspace settings from backend")
  .option("--format <format:string>", "Output format: json or yaml", { default: "yaml" })
  .option("--dry-run", "Preview changes without applying them")
  .option("--from-json <data:string>", "Use JSON data as backend settings instead of fetching from Windmill backend. JSON format: {\"include_path\": [\"f/**\"], \"include_type\": [\"script\", \"flow\", \"app\"]}. Useful for testing settings changes offline or comparing against hypothetical backend state.")
  .option("--json-output", "Output in JSON format")
  .option("--diff", "Show diff between local and remote settings")
  .option("--repository <repo:string>", "Specify git repository path (e.g. u/user/repo)")
  .action(async (opts) => {
    try {
      const result = await pullSettings(opts as any);

      if (!result.success) {
        if (opts.jsonOutput) {
          console.log(JSON.stringify({ success: false, error: result.error }));
        } else {
          log.error(colors.red(result.error || "Pull failed"));
        }
        Deno.exit(1);
      }

      if (opts.jsonOutput) {
        console.log(JSON.stringify(result));
      } else {
        if (opts.diff) {
          if (result.diff && result.diff.trim()) {
            console.log("Diff:");
            console.log(result.diff);
          } else {
            console.log("No differences found between local wmill.yaml and windmill git-sync settings");
          }
        } else if (result.diff) {
          console.log("Diff:");
          console.log(result.diff);
        } else {
          console.log(result.yaml || result.settings);
        }
      }
    } catch (error) {
      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: false, error: (error as Error).message }));
      } else {
        log.error(colors.red((error as Error).message));
      }
      Deno.exit(1);
    }
  })

  .command("push")
  .description("Push workspace settings to backend")
  .option("--format <format:string>", "Input format: json or yaml", { default: "yaml" })
  .option("--dry-run", "Preview changes without applying them")
  .option("--json-output", "Output in JSON format")
  .option("--diff", "Show diff between local and remote settings")
  .option("--from-json <data:string>", "Push settings from JSON data to Windmill backend instead of reading from wmill.yaml. JSON format: {\"include_path\": [\"f/**\"], \"include_type\": [\"script\", \"flow\", \"app\"]}. Bypasses local wmill.yaml file and directly configures Windmill backend git sync settings.")
  .option("--file <file:string>", "Settings file to push")
  .option("--repository <repo:string>", "Specify git repository path (e.g. u/user/repo)")
  .action(async (opts) => {
    try {
      const { file, fromJson, repository, ...restOpts } = opts;
      const result = await pushSettings({ ...restOpts, settingsFile: file, fromJson, repository } as any);

      if (!result.success) {
        if (opts.jsonOutput) {
          console.log(JSON.stringify({ success: false, error: result.error }));
        } else {
          log.error(colors.red(result.error || "Push failed"));
        }
        Deno.exit(1);
      }

      if (opts.jsonOutput) {
        console.log(JSON.stringify(result));
      } else {
        if (opts.diff) {
          if (result.diff && result.diff.trim()) {
            console.log("Diff:");
            console.log(result.diff);
          } else {
            console.log("No differences found between backend settings and local/provided settings");
          }
        } else if (result.diff) {
          console.log("Diff:");
          console.log(result.diff);
        } else {
          console.log(result.message || "Settings pushed successfully");
        }
      }
    } catch (error) {
      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: false, error: (error as Error).message }));
      } else {
        log.error(colors.red((error as Error).message));
      }
      Deno.exit(1);
    }
  })

  .command("list-workspaces")
  .description("List all workspace profiles in wmill.yaml")
  .option("--json-output", "Output in JSON format")
  .action(async (opts) => {
    try {
      const config = await readConfigFile();
      const { listWorkspaces } = await import("./conf.ts");
      const workspaces = listWorkspaces(config);

      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: true, workspaces }));
      } else {
        if (workspaces.length === 0) {
          console.log("No workspace profiles configured in wmill.yaml");
          console.log("Use multi-workspace format to configure workspace profiles");
        } else {
          console.log("Workspace profiles:");
          for (const workspace of workspaces) {
            const profile = config.workspaces![workspace];
            console.log(`  ${workspace}:`);
            console.log(`    Base URL: ${profile.baseUrl}`);
            console.log(`    Workspace ID: ${profile.workspaceId}`);
            if (profile.currentRepository) {
              console.log(`    Current Repository: ${profile.currentRepository}`);
            }
            const repoCount = Object.keys(profile.repositories || {}).length;
            console.log(`    Repositories: ${repoCount}`);
          }
        }
      }
    } catch (error) {
      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: false, error: (error as Error).message }));
      } else {
        log.error(colors.red(`Failed to list workspaces: ${(error as Error).message}`));
      }
      Deno.exit(1);
    }
  })

  .command("list-repositories")
  .description("List all repositories for a workspace profile")
  .option("--json-output", "Output in JSON format")
  .action(async (opts) => {
    try {
      // deno-lint-ignore no-explicit-any
      const workspaceName = (opts as any).workspace as string | undefined;
      if (!workspaceName) {
        const errorMsg = "Workspace name is required. Use --workspace to specify it.";
        if (opts.jsonOutput) {
          console.log(JSON.stringify({ success: false, error: errorMsg }));
        } else {
          log.error(colors.red(errorMsg));
        }
        Deno.exit(1);
        return;
      }

      const config = await readConfigFile();
      const { listWorkspaceRepositories, getWorkspaceProfile } = await import("./conf.ts");

      const workspaceProfile = getWorkspaceProfile(config, workspaceName);
      if (!workspaceProfile) {
        const errorMsg = `Workspace profile '${workspaceName}' not found in wmill.yaml`;
        if (opts.jsonOutput) {
          console.log(JSON.stringify({ success: false, error: errorMsg }));
        } else {
          log.error(colors.red(errorMsg));
        }
        Deno.exit(1);
        return;
      }

      const repositories = listWorkspaceRepositories(config, workspaceName);

      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: true, workspace: workspaceName, repositories }));
      } else {
        if (repositories.length === 0) {
          console.log(`No repositories configured for workspace '${workspaceName}'`);
        } else {
          console.log(`Repositories for workspace '${workspaceName}':`);
          for (const repo of repositories) {
            const repoSettings = workspaceProfile.repositories![repo];
            console.log(`  ${repo}:`);

            if (repoSettings.includes && repoSettings.includes.length > 0) {
              console.log(`    Includes: ${repoSettings.includes.join(', ')}`);
            }
            if (repoSettings.excludes && repoSettings.excludes.length > 0) {
              console.log(`    Excludes: ${repoSettings.excludes.join(', ')}`);
            }
            if (repoSettings.defaultTs) {
              console.log(`    Default TS: ${repoSettings.defaultTs}`);
            }

            // Show sync options if they differ from defaults
            const syncOptions = [];
            if (repoSettings.skipVariables !== true) syncOptions.push("skip variables");
            if (repoSettings.skipResources !== true) syncOptions.push("skip resources");
            if (repoSettings.skipResourceTypes !== true) syncOptions.push("skip resource types");
            if (repoSettings.skipSecrets !== true) syncOptions.push("skip secrets");
            if (repoSettings.skipScripts !== true) syncOptions.push("skip scripts");
            if (repoSettings.skipFlows !== true) syncOptions.push("skip flows");
            if (repoSettings.skipApps !== true) syncOptions.push("skip apps");
            if (repoSettings.skipFolders !== true) syncOptions.push("skip folders");
            if (repoSettings.includeSchedules) syncOptions.push("include schedules");
            if (repoSettings.includeTriggers) syncOptions.push("include triggers");
            if (repoSettings.includeUsers) syncOptions.push("include users");
            if (repoSettings.includeGroups) syncOptions.push("include groups");
            if (repoSettings.includeSettings) syncOptions.push("include settings");
            if (repoSettings.includeKey) syncOptions.push("include key");

            if (syncOptions.length > 0) {
              console.log(`    Options: ${syncOptions.join(', ')}`);
            }
          }

          if (workspaceProfile.currentRepository) {
            console.log(`\nCurrent repository: ${workspaceProfile.currentRepository}`);
          }
        }
      }
    } catch (error) {
      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: false, error: (error as Error).message }));
      } else {
        log.error(colors.red(`Failed to list repositories: ${(error as Error).message}`));
      }
      Deno.exit(1);
    }
  });

export default settingsCommand;
