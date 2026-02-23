import process from "node:process";
import { writeFile } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "./log.ts";
import { yamlParseFile } from "../utils/yaml.ts";
import { stringify as yamlStringify } from "yaml";
import * as wmill from "../../gen/services.gen.ts";
import { AIConfig, Config, GlobalSetting } from "../../gen/types.gen.ts";
import { compareInstanceObjects, InstanceSyncOptions } from "../commands/instance/instance.ts";
import { isSuperset } from "../types.ts";
import { deepEqual } from "../utils/utils.ts";
import { removeWorkerPrefix } from "../commands/worker-groups/worker-groups.ts";
import { decrypt, encrypt } from "../utils/local_encryption.ts";

// New grouped config interfaces
export interface AutoInviteConfig {
  enabled: boolean;
  domain?: string;
  operator: boolean;
  mode: "invite" | "add";
  instance_groups?: string[];
  instance_groups_roles?: Record<string, string>;
}

export interface ErrorHandlerConfig {
  path?: string;
  extra_args?: any;
  muted_on_cancel?: boolean;
  muted_on_user_path?: boolean;
}

export interface SuccessHandlerConfig {
  path?: string;
  extra_args?: any;
}

export interface SimplifiedSettings {
  // Grouped format (current)
  auto_invite?: AutoInviteConfig;
  error_handler?: ErrorHandlerConfig;
  success_handler?: SuccessHandlerConfig;

  // Other fields
  webhook?: string;
  deploy_to?: string;
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

// Legacy settings interface for reading old settings.yaml files
interface LegacySimplifiedSettings {
  auto_invite_enabled?: boolean;
  auto_invite_as?: string;
  auto_invite_mode?: string;
  error_handler?: string;
  error_handler_extra_args?: any;
  error_handler_muted_on_cancel?: boolean;
  success_handler?: string;
  success_handler_extra_args?: any;
  // Other fields same as SimplifiedSettings
  webhook?: string;
  deploy_to?: string;
  ai_config?: AIConfig;
  large_file_storage?: any;
  git_sync?: any;
  default_app?: string;
  default_scripts?: any;
  name?: string;
  mute_critical_alerts?: boolean;
  color?: string;
  operator_settings?: any;
}

// Helper to convert legacy flat settings to new grouped format
export function migrateToGroupedFormat(settings: any): SimplifiedSettings {
  const result: SimplifiedSettings = { name: settings.name ?? "" };

  // Copy non-legacy fields
  if (settings.webhook !== undefined) result.webhook = settings.webhook;
  if (settings.deploy_to !== undefined) result.deploy_to = settings.deploy_to;
  if (settings.ai_config !== undefined) result.ai_config = settings.ai_config;
  if (settings.large_file_storage !== undefined) result.large_file_storage = settings.large_file_storage;
  if (settings.git_sync !== undefined) result.git_sync = settings.git_sync;
  if (settings.default_app !== undefined) result.default_app = settings.default_app;
  if (settings.default_scripts !== undefined) result.default_scripts = settings.default_scripts;
  if (settings.mute_critical_alerts !== undefined) result.mute_critical_alerts = settings.mute_critical_alerts;
  if (settings.color !== undefined) result.color = settings.color;
  if (settings.operator_settings !== undefined) result.operator_settings = settings.operator_settings;

  // Handle auto_invite: check if already grouped or needs migration
  if (settings.auto_invite && typeof settings.auto_invite === "object") {
    result.auto_invite = settings.auto_invite;
  } else if (settings.auto_invite_enabled !== undefined) {
    // Legacy format
    result.auto_invite = {
      enabled: settings.auto_invite_enabled,
      operator: settings.auto_invite_as === "operator",
      mode: (settings.auto_invite_mode as "invite" | "add") ?? "invite",
    };
  }

  // Handle error_handler: check if already grouped or needs migration
  if (settings.error_handler && typeof settings.error_handler === "object") {
    result.error_handler = settings.error_handler;
  } else if (typeof settings.error_handler === "string") {
    // Legacy format (error_handler was a string path)
    result.error_handler = {
      path: settings.error_handler,
      extra_args: settings.error_handler_extra_args,
      muted_on_cancel: settings.error_handler_muted_on_cancel ?? false,
    };
  }

  // Handle success_handler: check if already grouped or needs migration
  if (settings.success_handler && typeof settings.success_handler === "object") {
    result.success_handler = settings.success_handler;
  } else if (typeof settings.success_handler === "string") {
    // Legacy format (success_handler was a string path)
    result.success_handler = {
      path: settings.success_handler,
      extra_args: settings.success_handler_extra_args,
    };
  }

  return result;
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
  localSettings: SimplifiedSettings | any
) {
  // Migrate local settings from legacy format if needed
  localSettings = migrateToGroupedFormat(localSettings);

  try {
    const remoteSettings = await wmill.getSettings({
      workspace,
    });

    const workspaceName = await wmill.getWorkspaceName({
      workspace,
    });

    // Build settings from remote (now using grouped format)
    settings = {
      auto_invite: remoteSettings.auto_invite as AutoInviteConfig | undefined,
      error_handler: remoteSettings.error_handler as ErrorHandlerConfig | undefined,
      success_handler: remoteSettings.success_handler as SuccessHandlerConfig | undefined,
      webhook: remoteSettings.webhook,
      deploy_to: remoteSettings.deploy_to,
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
    log.debug(`Updating webhook...`);
    await wmill.editWebhook({
      workspace,
      requestBody: {
        webhook: localSettings.webhook,
      },
    });
  }

  // Handle auto_invite using grouped format
  if (!deepEqual(localSettings.auto_invite, settings.auto_invite)) {
    log.debug(`Updating auto invite...`);

    const localAutoInvite = localSettings.auto_invite;
    if (localAutoInvite?.mode && !["add", "invite"].includes(localAutoInvite.mode)) {
      throw new Error(
        `Invalid value for auto_invite.mode. Valid values are "invite" and "add"`
      );
    }
    try {
      await wmill.editAutoInvite({
        workspace,
        requestBody: localAutoInvite?.enabled
          ? {
              operator: localAutoInvite.operator ?? false,
              invite_all: true,
              auto_add: localAutoInvite.mode === "add",
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
        requestBody: localAutoInvite?.enabled
          ? {
              operator: localAutoInvite.operator ?? false,
              invite_all: false,
              auto_add: localAutoInvite.mode === "add",
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

  // Handle error_handler using grouped format
  if (!deepEqual(localSettings.error_handler, settings.error_handler)) {
    log.debug(`Updating error handler...`);
    const localErrorHandler = localSettings.error_handler;
    await wmill.editErrorHandler({
      workspace,
      requestBody: {
        path: localErrorHandler?.path,
        extra_args: localErrorHandler?.extra_args,
        muted_on_cancel: localErrorHandler?.muted_on_cancel ?? false,
        muted_on_user_path: localErrorHandler?.muted_on_user_path ?? false,
      },
    });
  }

  // Handle success_handler using grouped format
  if (!deepEqual(localSettings.success_handler, settings.success_handler)) {
    log.debug(`Updating success handler...`);
    const localSuccessHandler = localSettings.success_handler;
    await wmill.editSuccessHandler({
      workspace,
      requestBody: {
        path: localSuccessHandler?.path,
        extra_args: localSuccessHandler?.extra_args,
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

  if (!deepEqual(localSettings.operator_settings, settings.operator_settings)) {
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
    await writeFile(
      instanceSettingsPath,
      yamlStringify(processedSettings),
      "utf-8"
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

    await writeFile(
      instanceConfigsPath,
      yamlStringify(remoteConfigs as any),
      "utf-8"
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
