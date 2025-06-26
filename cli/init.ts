import { colors, Command, log, yamlStringify, setClient } from "./deps.ts";
import { mergeConfigWithConfigFile, SyncOptions, WorkspaceProfile, DEFAULT_SYNC_OPTIONS, RepositorySyncOptions } from "./conf.ts";
import { resolveWorkspace, requireLogin } from "./context.ts";
import { readLockfile } from "./metadata.ts";
import * as wmill from "./gen/services.gen.ts";
import { fetchBackendSettings, listRepositories } from "./settings.ts";
import { GlobalOptions } from "./types.ts";

const DEFAULT_INIT_SETTINGS: SyncOptions = {
  ...DEFAULT_SYNC_OPTIONS,
  skipVariables: true,
  skipResources: true,
  skipSecrets: true,
};

function filterUndefined<T extends Record<string, any>>(obj: T): Partial<T> {
  const filtered: Partial<T> = {};
  for (const [key, value] of Object.entries(obj)) {
    if (value !== undefined) {
      filtered[key as keyof T] = value;
    }
  }
  return filtered;
}

async function initAction(opts: {
  workspace?: string;
  repository?: string;
  baseUrl?: string;
  token?: string;
}) {
  const existingWmillYaml = await Deno.stat("wmill.yaml").catch(() => null);

  if (existingWmillYaml) {
    if (opts.workspace) {
      await addWorkspaceToConfig(opts);
    } else {
      log.error(colors.yellow("wmill.yaml already exists. Use --workspace <name> to add workspace-specific settings."));
    }
  } else {
    await createWmillYaml(opts);
  }

  await readLockfile();
}

async function setupAuthAndWorkspace(opts: GlobalOptions, config?: any) {
  const mergedOpts = config ? { ...config, ...opts } : await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(mergedOpts);

  if (opts.baseUrl && opts.token) {
    setClient(opts.token, workspace.remote.substring(0, workspace.remote.length - 1));
    try {
      await wmill.globalWhoami();
    } catch (error) {
      throw new Error(`Failed to authenticate with provided credentials: ${error}`);
    }
  } else {
    await requireLogin(mergedOpts);
  }

  return workspace;
}

async function addWorkspaceToConfig(opts: {
  workspace?: string;
  repository?: string;
  baseUrl?: string;
  token?: string;
}) {
  try {
    const existingConfig = await mergeConfigWithConfigFile({});
    const workspace = await setupAuthAndWorkspace(opts as GlobalOptions, existingConfig);

    if (!existingConfig.workspaces) {
      existingConfig.workspaces = {};
    }

    const workspaceProfile = await createWorkspaceProfile(workspace, opts);
    const workspaceName = workspace.name || workspace.workspaceId;
    existingConfig.workspaces[workspaceName] = workspaceProfile;

    await Deno.writeTextFile("wmill.yaml", yamlStringify(existingConfig));
    log.info(colors.green(`Added workspace '${workspaceName}' settings to wmill.yaml`));
  } catch (error) {
    log.error(colors.red(`Failed to add workspace settings: ${error}`));
  }
}

async function createWmillYaml(opts: {
  workspace?: string;
  repository?: string;
  baseUrl?: string;
  token?: string;
}) {
  let config: SyncOptions = { ...DEFAULT_INIT_SETTINGS };

  try {
    const workspace = await setupAuthAndWorkspace(opts as GlobalOptions);
    const workspaceProfile = await createWorkspaceProfile(workspace, opts);
    const workspaceName = workspace.name || workspace.workspaceId;

    config = {
      defaultTs: "bun",
      workspaces: {
        [workspaceName]: workspaceProfile
      },
      defaultWorkspace: workspaceName
    };

    log.info(colors.green(`Fetched sync settings from workspace '${workspaceName}'`));
  } catch (error) {
    if (opts.workspace || opts.baseUrl) {
      log.warn(colors.yellow(`Could not fetch settings from workspace: ${error}`));
      log.info(colors.blue("Using default settings instead. You can manually edit wmill.yaml or use 'wmill settings pull' later."));
    } else {
      log.info(colors.blue("No workspace specified. Using default settings. Add a workspace with 'wmill workspace add' or specify --workspace and --base-url."));
    }
  }

  await Deno.writeTextFile("wmill.yaml", yamlStringify(config));
  log.info(colors.green("wmill.yaml created successfully"));
}

async function createWorkspaceProfile(workspace: any, opts: { repository?: string }): Promise<WorkspaceProfile> {
  const workspaceProfile: WorkspaceProfile = {
    baseUrl: workspace.remote,
    workspaceId: workspace.workspaceId,
  };

  try {
    const repositories = await listRepositories(workspace);

    if (repositories.length === 0) {
      log.warn(colors.yellow(`No git repositories found in workspace '${workspace.workspaceId}'`));
      // When no repositories exist, include default sync settings in the workspace profile
      Object.assign(workspaceProfile, filterUndefined(DEFAULT_SYNC_OPTIONS));
      return workspaceProfile;
    }

    const repoSettings: { [key: string]: RepositorySyncOptions } = {};

    if (opts.repository) {
      // Fetch settings for specified repository
      const backendResult = await fetchBackendSettings(workspace, opts.repository);
      const { workspaces, defaultWorkspace, ...syncOptions } = backendResult.settings;
      repoSettings[opts.repository] = filterUndefined(syncOptions);
      workspaceProfile.currentRepository = opts.repository;
    } else {
      // Fetch settings for all repositories
      for (const repo of repositories) {
        try {
          const backendResult = await fetchBackendSettings(workspace, repo.display_path);
          const { workspaces, defaultWorkspace, ...syncOptions } = backendResult.settings;
          repoSettings[repo.display_path] = filterUndefined(syncOptions);
        } catch (error) {
          log.warn(colors.yellow(`Could not fetch settings for repository '${repo.display_path}': ${error}`));
        }
      }

      // Set first repository as current
      const repoNames = Object.keys(repoSettings);
      if (repoNames.length > 0) {
        workspaceProfile.currentRepository = repoNames[0];
      } else {
        // If repositories exist but no settings could be fetched, include default sync settings
        log.warn(colors.yellow(`Could not fetch settings for any repository in workspace '${workspace.workspaceId}'`));
        Object.assign(workspaceProfile, filterUndefined(DEFAULT_SYNC_OPTIONS));
      }
    }

    workspaceProfile.repositories = repoSettings;
  } catch (error) {
    log.warn(colors.yellow(`Could not fetch repository settings: ${error}`));
    // If repository fetching fails entirely, include default sync settings
    Object.assign(workspaceProfile, filterUndefined(DEFAULT_SYNC_OPTIONS));
  }

  return workspaceProfile;
}

const command = new Command()
  .description("Bootstrap a windmill project with a wmill.yaml file")
  .option("--repository <repo:string>", "Specify git repository path (e.g. u/user/repo)")
  .option("--base-url <url:string>", "Specify the base URL for the workspace")
  .option("--token <token:string>", "Specify the authentication token for the workspace")
  .action(initAction);

export default command;