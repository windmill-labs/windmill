import { SettingService } from "./deps.ts";
import { yamlStringify } from "./deps.ts";
import { GlobalSetting } from "./deps.ts";
import { Config } from "./deps.ts";
import { ConfigService } from "./deps.ts";
import { yamlParse } from "./deps.ts";
import { WorkspaceService, log } from "./deps.ts";
import { isSuperset } from "./types.ts";
import { deepEqual } from "./utils.ts";

interface SimplifiedSettings {
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
}

export async function pushWorkspaceSettings(
  workspace: string,
  _path: string,
  settings: SimplifiedSettings | undefined,
  localSettings: SimplifiedSettings
) {
  try {
    const remoteSettings = await WorkspaceService.getSettings({
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
    await WorkspaceService.editWebhook({
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
      await WorkspaceService.editAutoInvite({
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
      await WorkspaceService.editAutoInvite({
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
    await WorkspaceService.editCopilotConfig({
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
    await WorkspaceService.editErrorHandler({
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
    await WorkspaceService.editDeployTo({
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
    await WorkspaceService.editLargeFileStorageConfig({
      workspace,
      requestBody: {
        large_file_storage: localSettings.large_file_storage,
      },
    });
  }
  if (!deepEqual(localSettings.git_sync, settings.git_sync)) {
    log.debug(`Updating git sync...`);
    await WorkspaceService.editWorkspaceGitSyncConfig({
      workspace,
      requestBody: {
        git_sync_settings: localSettings.git_sync,
      },
    });
  }
  if (!deepEqual(localSettings.default_scripts, settings.default_scripts)) {
    log.debug(`Updating default scripts...`);
    await WorkspaceService.editDefaultScripts({
      workspace,
      requestBody: localSettings.default_scripts,
    });
  }
  if (localSettings.default_app !== settings.default_app) {
    log.debug(`Updating default app...`);
    await WorkspaceService.editWorkspaceDefaultApp({
      workspace,
      requestBody: {
        default_app_path: localSettings.default_app,
      },
    });
  }
}

export async function pullInstanceSettings() {
  log.info("Pulling settings from instance");
  const settings = await SettingService.listGlobalSettings();

  await Deno.writeTextFile(
    "instance_settings.yaml",
    yamlStringify(settings as any)
  );

  log.info("Settings written to instance_settings.yaml");
}

export async function pushInstanceSettings() {
  log.info("Pushing settings to instance");
  const settings = yamlParse(
    await Deno.readTextFile("instance_settings.yaml")
  ) as GlobalSetting[];

  for (const setting of settings) {
    try {
      await SettingService.setGlobal({
        key: setting.name,
        requestBody: {
          value: setting.value,
        },
      });
    } catch (err) {
      log.error(`Failed to set setting ${setting.name}: ${err}`);
    }
  }

  log.info("Settings pushed to instance");
}

export async function pullInstanceConfigs() {
  log.info("Pulling configs from instance");
  const configs = await ConfigService.listConfigs();

  await Deno.writeTextFile(
    "instance_configs.yaml",
    yamlStringify(configs as any)
  );

  log.info("Configs written to instance_configs.yaml");
}

export async function pushInstanceConfigs() {
  log.info("Pushing configs to instance");
  const configs = yamlParse(
    await Deno.readTextFile("instance_configs.yaml")
  ) as Config[];

  for (const config of configs) {
    try {
      await ConfigService.updateConfig({
        name: config.name,
        requestBody: config.config,
      });
    } catch (err) {
      log.error(`Failed to set config ${config.name}: ${err}`);
    }
  }

  log.info("Configs pushed to instance");
}
