import { yamlStringify } from "./deps.ts";
import { Confirm } from "./deps.ts";
import { colors } from "./deps.ts";
import { yamlParseFile } from "./deps.ts";
import { log } from "./deps.ts";
import { compareInstanceObjects } from "./instance.ts";
import { isSuperset } from "./types.ts";
import { deepEqual } from "./utils.ts";
import * as wmill from "./gen/services.gen.ts";
import { Config, GlobalSetting } from "./gen/types.gen.ts";
import { removeWorkerPrefix } from "./worker_groups.ts";
import process from "node:process";

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
  openai_resource_path?: string;
  code_completion_enabled: boolean;
  large_file_storage?: any;
  git_sync?: any;
  default_app?: string;
  default_scripts?: any;
  name: string;
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
      openai_resource_path: remoteSettings.openai_resource_path,
      code_completion_enabled: remoteSettings.code_completion_enabled,
      large_file_storage: remoteSettings.large_file_storage,
      git_sync: remoteSettings.git_sync,
      default_app: remoteSettings.default_app,
      default_scripts: remoteSettings.default_scripts,
      name: workspaceName,
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
  if (
    localSettings.openai_resource_path !== settings.openai_resource_path ||
    localSettings.code_completion_enabled !== settings.code_completion_enabled
  ) {
    log.debug(`Updating openai settings...`);
    await wmill.editCopilotConfig({
      workspace,
      requestBody: {
        openai_resource_path: localSettings.openai_resource_path,
        code_completion_enabled: localSettings.code_completion_enabled,
      },
    });
  }
  if (
    localSettings.error_handler !== settings.error_handler ||
    !deepEqual(
      localSettings.error_handler_extra_args,
      settings.error_handler_extra_args
    ) ||
    localSettings.error_handler_muted_on_cancel !==
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
  if (localSettings.deploy_to !== settings.deploy_to) {
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
  if (localSettings.default_app !== settings.default_app) {
    log.debug(`Updating default app...`);
    await wmill.editWorkspaceDefaultApp({
      workspace,
      requestBody: {
        default_app_path: localSettings.default_app,
      },
    });
  }

  if (localSettings.name !== settings.name) {
    log.debug(`Updating workspace name...`);
    await wmill.changeWorkspaceName({
      workspace,
      requestBody: {
        new_name: localSettings.name,
      },
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

const INSTANCE_SETTINGS_PATH = "instance_settings.yaml";

export async function readInstanceSettings() {
  let localSettings: GlobalSetting[] = [];

  try {
    localSettings = (await yamlParseFile(INSTANCE_SETTINGS_PATH)) as GlobalSetting[];
  } catch {
    log.warn(`No ${INSTANCE_SETTINGS_PATH} found`);
  }
  return localSettings;
}


import { decrypt, encrypt } from "./local_encryption.ts";

const SENSITIVE_FIELD: string[] = ["license_key", "jwt_secret"]

async function processInstanceSettings(settings: GlobalSetting[], mode: "encode" | "decode"): Promise<GlobalSetting[]> {
  const encKey = process.env.WMILL_INSTANCE_LOCAL_ENCRYPTION_KEY;
  if (encKey) {
    const res: GlobalSetting[] = []

    for (const s of settings) {
      if (SENSITIVE_FIELD.includes(s.name) && typeof s.value === "string") {
        res.push(await processField(s, "value", encKey, mode) as GlobalSetting);
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
    log.warn("No encryption key found, skipping encryption. Recommend setting WMILL_INSTANCE_LOCAL_ENCRYPTION_KEY");
  }
  return settings;
}

async function processField(obj: { [key: string]: any }, field: string, encKey: string, mode: "encode" | "decode"): Promise<{ [key: string]: any }> {
  return {
    ...obj,
    [field]: mode === "encode" ? await encrypt(obj[field], encKey) : await decrypt(obj[field], encKey) as any,
  }
}

export async function pullInstanceSettings(preview = false) {
  const remoteSettings = await wmill.listGlobalSettings();

  if (preview) {
    const localSettings: GlobalSetting[] = await readInstanceSettings();
    const processedSettings = await processInstanceSettings(remoteSettings, "encode");
    return compareInstanceObjects(
      processedSettings,
      localSettings,
      "name",
      "setting"
    );
  } else {
    log.info("Pulling settings from instance");

    const processedSettings = await processInstanceSettings(remoteSettings, "encode");
    await Deno.writeTextFile(
      INSTANCE_SETTINGS_PATH,
      yamlStringify(processedSettings)
    );

    log.info(colors.green(`Settings written to ${INSTANCE_SETTINGS_PATH}`));
  }
}

export async function pushInstanceSettings(
  preview: boolean = false,
  baseUrl?: string
) {
  const remoteSettings = await wmill.listGlobalSettings();
  let localSettings: GlobalSetting[] = await readInstanceSettings();
  localSettings = await processInstanceSettings(localSettings, "decode");

  if (baseUrl) {
    localSettings = localSettings.filter((s) => s.name !== "base_url");
    localSettings.push({
      name: "base_url",
      //@ts-ignore
      value: baseUrl,
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

export async function readLocalConfigs() {
  let localConfigs: Config[] = [];

  try {
    localConfigs = (await yamlParseFile("instance_configs.yaml")) as Config[];
  } catch {
    log.warn("No instance_configs.yaml found");
  }
  return localConfigs;
}

export async function pullInstanceConfigs(preview = false) {
  const remoteConfigs = (await wmill.listConfigs()).map((x) => {
    return {
      ...x,
      name: removeWorkerPrefix(x.name),
    };
  });

  if (preview) {
    const localConfigs: Config[] = await readLocalConfigs();

    return compareInstanceObjects(
      remoteConfigs,
      localConfigs,
      "name",
      "config"
    );
  } else {
    log.info("Pulling configs from instance");

    await Deno.writeTextFile(
      "instance_configs.yaml",
      yamlStringify(remoteConfigs as any)
    );

    log.info(colors.green("Configs written to instance_configs.yaml"));
  }
}

export async function pushInstanceConfigs(preview: boolean = false) {
  const remoteConfigs = (await wmill.listConfigs()).map((x) => {
    return {
      ...x,
      name: removeWorkerPrefix(x.name),
    };
  });
  const localConfigs = await readLocalConfigs();

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
