import process from "node:process";
import { colors, Confirm, log, yamlParseFile, yamlStringify } from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";
import { AIConfig, Config, GlobalSetting } from "./gen/types.gen.ts";
import { compareInstanceObjects, InstanceSyncOptions } from "./instance.ts";
import { isSuperset } from "./types.ts";
import { deepEqual } from "./utils.ts";
import { removeWorkerPrefix } from "./worker_groups.ts";
import { Command } from "./deps.ts";
import { GlobalOptions } from "./types.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "./conf.ts";
import { requireLogin, resolveWorkspace } from "./context.ts";
import { uiStateToSyncOptions, parseJsonInput } from "./settings_utils.ts";

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
      yamlStringify(processedSettings)
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
      yamlStringify(remoteConfigs as any)
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
async function fetchBackendSettings(workspace: { workspaceId: string }): Promise<SyncOptions> {
  const backendSettings = await wmill.getSettings({ workspace: workspace.workspaceId });

  if (backendSettings.git_sync?.repositories && backendSettings.git_sync.repositories.length > 0) {
    const firstRepo = backendSettings.git_sync.repositories[0];
    const repoSettings = firstRepo.settings || {};

    return {
      defaultTs: 'bun',
      includes: repoSettings.include_path,
      excludes: [],
      codebases: [],
      skipVariables: !repoSettings.include_type?.includes('variable'),
      skipResources: !repoSettings.include_type?.includes('resource'),
      skipResourceTypes: !repoSettings.include_type?.includes('resourcetype'),
      skipSecrets: !repoSettings.include_type?.includes('secret'),
      includeSchedules: repoSettings.include_type?.includes('schedule') || false,
      includeTriggers: repoSettings.include_type?.includes('trigger') || false,
      includeUsers: repoSettings.include_type?.includes('user') || false,
      includeGroups: repoSettings.include_type?.includes('group') || false,
      includeSettings: (repoSettings.include_type as string[])?.includes('settings') || false,
      includeKey: (repoSettings.include_type as string[])?.includes('key') || false
    };
  } else {
    return {
      defaultTs: 'bun',
      includes: ['f/**'],
      excludes: [],
      codebases: [],
      skipVariables: false,
      skipResources: false,
      skipResourceTypes: false,
      skipSecrets: false,
      includeSchedules: false,
      includeTriggers: false,
      includeUsers: false,
      includeGroups: false,
      includeSettings: false,
      includeKey: false
    };
  }
}

// Helper function to generate diff between two settings
async function generateDiff(fromFile: string, toFile: string): Promise<string> {
  const diffResult = await runDiffCommand([fromFile, toFile]);
  return diffResult.success ? diffResult.stdout : `Failed to compare: ${diffResult.stderr}`;
}

// Helper function to get YAML utilities
async function getYamlUtils() {
  const { stringify, parse } = await import('jsr:@std/yaml@^1.0.5');
  return { stringify, parse };
}



// Helper function to read local settings file
async function readLocalSettingsFile(filePath: string = 'wmill.yaml'): Promise<{ content: string; settings: SyncOptions }> {
  try {
    const content = await Deno.readTextFile(filePath);
    const { parse } = await getYamlUtils();
    const settings = parse(content) as SyncOptions;
    return { content, settings };
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

// Helper function to update git sync repositories with new settings
async function updateGitSyncRepositories(workspace: { workspaceId: string }, localSettings: SyncOptions): Promise<void> {
  const backendSettings = await wmill.getSettings({ workspace: workspace.workspaceId });

  if (backendSettings.git_sync?.repositories && backendSettings.git_sync.repositories.length > 0) {
    const repositories = backendSettings.git_sync.repositories.map((repo, index) => {
      if (index === 0) {
        return {
          script_path: repo.script_path,
          git_repo_resource_path: repo.git_repo_resource_path,
          use_individual_branch: repo.use_individual_branch ?? false,
          group_by_folder: repo.group_by_folder ?? false,
          collapsed: false,
          settings: {
            include_path: localSettings.includes || ['f/**'],
            include_type: [
              'script', 'flow', 'app', 'folder',
              ...(localSettings.skipVariables === false ? ['variable'] as const : []),
              ...(localSettings.skipResources === false ? ['resource'] as const : []),
              ...(localSettings.skipResourceTypes === false ? ['resourcetype'] as const : []),
              ...(localSettings.skipSecrets === false ? ['secret'] as const : []),
              ...(localSettings.includeSchedules === true ? ['schedule'] as const : []),
              ...(localSettings.includeTriggers === true ? ['trigger'] as const : []),
              ...(localSettings.includeUsers === true ? ['user'] as const : []),
              ...(localSettings.includeGroups === true ? ['group'] as const : []),
              ...(localSettings.includeSettings === true ? ['settings'] as const : []),
              ...(localSettings.includeKey === true ? ['key'] as const : [])
            ] as ('script' | 'flow' | 'app' | 'folder' | 'variable' | 'resource' | 'resourcetype' | 'secret' | 'schedule' | 'trigger' | 'user' | 'group' | 'settings' | 'key')[]
          }
        };
      }
      return repo;
    });

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
  operationType: 'pull' | 'push'
): Promise<SettingsResult | null> {
  if (!opts.fromJson) return null;

  // Handle JSON input mode - JSON represents simulated backend state
  const uiState = parseJsonInput(opts.fromJson);
  const simulatedBackendSettings = uiStateToSyncOptions(uiState);

  if (opts.diff) {
    // For --from-json --diff: Compare JSON (simulated backend) with local wmill.yamlfile
    try {
      const { settings: actualLocalSettings } = await readLocalSettingsFile();
      const { stringify } = await getYamlUtils();

      const simulatedBackendYaml = stringify(simulatedBackendSettings as Record<string, unknown>);
      const localYaml = stringify(actualLocalSettings as Record<string, unknown>);

      const { file1: tmpSimulatedBackendFile, file2: tmpLocalFile } = await createTempDiffFiles(simulatedBackendYaml, localYaml);
      const diff = await generateDiff(tmpLocalFile, tmpSimulatedBackendFile);

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
    } catch (error) {
      return {
        success: false,
        error: `Could not read local wmill.yaml file: ${(error as Error).message}`
      };
    }
  }

  // For non-diff operations, use JSON as the settings
  const { stringify } = await getYamlUtils();
  const yamlContent = stringify(simulatedBackendSettings as Record<string, unknown>);

  if (opts.dryRun) {
    const message = operationType === 'pull'
      ? "Dry run - showing what would be written to wmill.yaml from JSON input"
      : "Dry run - showing what would be pushed from JSON input";

    return {
      success: true,
      yaml: yamlContent,
      settings: simulatedBackendSettings,
      message
    };
  }

  // For actual operations
  if (operationType === 'pull') {
    await Deno.writeTextFile('wmill.yaml', yamlContent);
    return {
      success: true,
      yaml: yamlContent,
      settings: simulatedBackendSettings,
      message: "Settings written to wmill.yaml from JSON input"
    };
  } else {
    // Push operation requires workspace parameter - will be handled by caller
    return {
      success: true,
      yaml: yamlContent,
      settings: simulatedBackendSettings,
      message: "Settings ready to push from JSON input"
    };
  }
}

// ========== CORE FUNCTIONS ==========

export async function pullSettings(opts: GlobalOptions & SyncOptions & {
  format?: 'json' | 'yaml';
  diff?: boolean;
  fromJson?: string;
}): Promise<SettingsResult> {
  try {
    opts = await mergeConfigWithConfigFile(opts);
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);

    let currentSettings: SyncOptions;
    let yamlContent: string;

    if (opts.fromJson) {
      const result = await handleFromJsonProcessing(opts, 'pull');
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
      currentSettings = await fetchBackendSettings(workspace);
      const { stringify } = await getYamlUtils();
      yamlContent = stringify(currentSettings as Record<string, unknown>);
    }

    if (opts.diff) {
      // Compare backend with local wmill.yaml
      let currentLocalContent = '';
      try {
        currentLocalContent = await Deno.readTextFile('wmill.yaml');
      } catch {
        currentLocalContent = '';
      }

      const { file1: tmpLocalFile, file2: tmpBackendFile } = await createTempDiffFiles(currentLocalContent, yamlContent);
      const diff = await generateDiff(tmpLocalFile, tmpBackendFile);

      return {
        success: true,
        yaml: yamlContent,
        settings: currentSettings,
        diff: diff,
        message: "Diff between local wmill.yaml and windmill git-sync settings"
      };
    }

    if (opts.dryRun) {
      let currentLocalContent = '';
      try {
        currentLocalContent = await Deno.readTextFile('wmill.yaml');
      } catch {
        currentLocalContent = '';
      }

      const { file1: tmpLocalFile, file2: tmpBackendFile } = await createTempDiffFiles(currentLocalContent, yamlContent);
      const diff = await generateDiff(tmpLocalFile, tmpBackendFile);

      return {
        success: true,
        yaml: yamlContent,
        settings: currentSettings,
        diff: diff,
        message: "Dry run - showing what would be pulled from backend"
      };
    }

    await Deno.writeTextFile('wmill.yaml', yamlContent);
    return {
      success: true,
      yaml: yamlContent,
      settings: currentSettings
    };
  } catch (error: any) {
    return {
      success: false,
      error: (error as Error).message || "Pull failed"
    };
  }
}

export async function pushSettings(opts: GlobalOptions & SyncOptions & {
  format?: 'json' | 'yaml';
  settingsData?: SyncOptions;
  settingsFile?: string;
  json?: boolean;
  diff?: boolean;
  fromJson?: string;
}): Promise<SettingsResult> {
  try {
    opts = await mergeConfigWithConfigFile(opts);
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);

    let localSettings: SyncOptions;

    if (opts.fromJson) {
      const result = await handleFromJsonProcessing(opts, 'push');
      if (result) {
        if (!result.success) {
          return result;
        }
        // If handleFromJsonProcessing handled diff/dryRun, return immediately
        if (opts.diff || opts.dryRun) {
          return result;
        }
        localSettings = result.settings!;

        const { stringify } = await getYamlUtils();
        const yamlContent = stringify(localSettings as Record<string, unknown>);

        if (opts.dryRun) {
          return {
            success: true,
            yaml: yamlContent,
            settings: localSettings,
            message: "Dry run - showing what would be pushed from JSON input"
          };
        }

        await updateGitSyncRepositories(workspace, localSettings);
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
        const { parse } = await getYamlUtils();
        localSettings = parse(fileContent) as SyncOptions;
      }
    } else {
      try {
        const { settings } = await readLocalSettingsFile();
        localSettings = settings;
      } catch (error) {
        return {
          success: false,
          error: "Could not read local wmill.yaml file. Make sure it exists or use --file to specify a different file."
        };
      }
    }

    if (opts.diff) {
      // Compare local settings with backend settings
      const backendSyncOptions = await fetchBackendSettings(workspace);
      const { stringify } = await getYamlUtils();

      const localYaml = stringify(localSettings as Record<string, unknown>);
      const backendYaml = stringify(backendSyncOptions as Record<string, unknown>);

      const { file1: tmpBackendFile, file2: tmpLocalFile } = await createTempDiffFiles(backendYaml, localYaml);
      const diff = await generateDiff(tmpBackendFile, tmpLocalFile);

      return {
        success: true,
        yaml: localYaml,
        settings: localSettings,
        diff: diff,
        message: "Diff between backend settings and local wmill.yaml"
      };
    }

    if (opts.dryRun) {
      const { stringify } = await getYamlUtils();
      const localYaml = stringify(localSettings as Record<string, unknown>);

      return {
        success: true,
        yaml: localYaml,
        settings: localSettings,
        message: "Dry run - showing what would be pushed to backend"
      };
    }

    await updateGitSyncRepositories(workspace, localSettings);
    return {
      success: true,
      message: "Settings successfully pushed to workspace backend"
    };
  } catch (error) {
    return {
      success: false,
      error: (error as Error).message
    };
  }
}

// CLI command structure
const settingsCommand = new Command()
  .description("Manage workspace settings")
  .command("pull")
  .description("Pull workspace settings from backend")
  .option("--format <format:string>", "Output format: json or yaml", { default: "yaml" })
  .option("--dry-run", "Preview changes without applying them")
  .option("--from-json <data:string>", "JSON string of UI state data (include_path, include_type)")
  .option("--json-output", "Output in JSON format")
  .option("--diff", "Show diff between local and remote settings")
  .action(async (opts) => {
    try {
      const mergedOpts = await mergeConfigWithConfigFile(opts) as (GlobalOptions & SyncOptions & {
        format?: 'json' | 'yaml';
        diff?: boolean;
        fromJson?: string;
      });
      const result = await pullSettings(mergedOpts);

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
  .option("--from-json <data:string>", "JSON string of UI state data (include_path, include_type)")
  .option("--file <file:string>", "Settings file to push")
  .action(async (opts) => {
    try {
      const { file, fromJson, ...restOpts } = opts;
      const mergedOpts = await mergeConfigWithConfigFile(restOpts) as (GlobalOptions & SyncOptions & {
        format?: 'json' | 'yaml';
        settingsData?: SyncOptions;
        settingsFile?: string;
        diff?: boolean;
        fromJson?: string;
      });
      const result = await pushSettings({ ...mergedOpts, settingsFile: file, fromJson });

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
  });

export default settingsCommand;
