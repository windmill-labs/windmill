import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { Select } from "@cliffy/prompt/select";
import { Confirm } from "@cliffy/prompt/confirm";
import { Input } from "@cliffy/prompt/input";

import { loginInteractive } from "./login.ts";
import { GlobalOptions } from "../types.ts";
import { getHeaders } from "../utils/utils.ts";

import {
  getActiveWorkspace,
  getWorkspaceByName,
  Workspace,
  allWorkspaces,
  addWorkspace,
} from "../commands/workspace/workspace.ts";
import { getLastUsedProfile, setLastUsedProfile } from "./branch-profiles.ts";
import { readConfigFile } from "./conf.ts";
import {
  getCurrentGitBranch,
  getOriginalBranchForWorkspaceForks,
  getWorkspaceIdForWorkspaceForkFromBranchName,
  isGitRepository,
} from "../utils/git.ts";
import { WM_FORK_PREFIX } from "./constants.ts";

// Helper function to select from multiple matching profiles
async function selectFromMultipleProfiles(
  profiles: Workspace[],
  baseUrl: string,
  workspaceId: string,
  context: string,
  configDir?: string
): Promise<Workspace> {
  if (profiles.length === 1) {
    return profiles[0];
  }

  // Check for last used profile
  const lastUsedProfileName = await getLastUsedProfile(
    "",
    baseUrl,
    workspaceId,
    configDir
  );
  if (lastUsedProfileName) {
    const lastUsedProfile = profiles.find(
      (p) => p.name === lastUsedProfileName
    );
    if (lastUsedProfile) {
      log.info(
        colors.green(
          `Using last used profile '${lastUsedProfile.name}' for ${context}`
        )
      );
      return lastUsedProfile;
    }
  }

  // No last used or it no longer exists - prompt for selection
  if (!!!process.stdin.isTTY || !!!process.stdout.isTTY) {
    const selectedProfile = profiles[0];
    log.info(
      colors.yellow(
        `Multiple profiles found for ${context}. Using first available profile: '${selectedProfile.name}'`
      )
    );

    // Save selection for next time
    await setLastUsedProfile(
      "", // No branch context for general workspace selection
      baseUrl,
      workspaceId,
      selectedProfile.name,
      configDir
    );

    return selectedProfile;
  }

  log.info(
    colors.yellow(`\nMultiple workspace profiles found for ${context}:`)
  );

  const selectedName = await Select.prompt({
    message: "Select profile",
    options: profiles.map((p) => ({
      name: `${p.name} (${p.workspaceId} on ${p.remote})`,
      value: p.name,
    })),
  });

  const selectedProfile = profiles.find((p) => p.name === selectedName)!;

  // Save selection for next time
  await setLastUsedProfile(
    "", // No branch context for general workspace selection
    baseUrl,
    workspaceId,
    selectedProfile.name,
    configDir
  );

  return selectedProfile;
}

/**
 * Prompts the user to create a new workspace profile interactively
 */
async function createWorkspaceProfileInteractively(
  normalizedBaseUrl: string,
  workspaceId: string,
  currentBranch: string,
  opts: GlobalOptions,
  context: { rawBranch: string; isForked: boolean }
): Promise<Workspace | undefined> {
  // Log appropriate message based on context
  if (!context.isForked) {
    log.info(
      colors.yellow(
        `\nNo workspace profile found for branch '${context.rawBranch}'\n` +
          `(${normalizedBaseUrl}, ${workspaceId})`
      )
    );
  } else {
    log.info(
      colors.yellow(
        `\nNo workspace profile was found for this forked workspace\n` +
          `(${normalizedBaseUrl}, ${workspaceId})`
      )
    );
  }

  if (!!!process.stdin.isTTY || !!!process.stdout.isTTY) {
    log.info(
      "Not a TTY, cannot create profile interactively. Use 'wmill workspace add' first."
    );
    return undefined;
  }

  const shouldCreate = await Confirm.prompt({
    message: "Would you like to create a new workspace profile?",
    default: true,
  });

  if (!shouldCreate) {
    return undefined;
  }

  // Prompt for profile details
  const profileName = await Input.prompt({
    message: "Profile name",
    default: workspaceId,
  });

  const token = await loginInteractive(normalizedBaseUrl);
  if (!token) {
    log.error("Failed to obtain token");
    return undefined;
  }

  // Create the new profile
  const newWorkspace: Workspace = {
    name: profileName,
    remote: normalizedBaseUrl,
    workspaceId: workspaceId,
    token: token,
  };

  await addWorkspace(newWorkspace, opts);

  // Set as last used for this branch
  await setLastUsedProfile(
    currentBranch,
    normalizedBaseUrl,
    workspaceId,
    profileName,
    opts.configDir
  );

  log.info(
    colors.green(
      `✓ Created profile '${profileName}' for ${workspaceId} on ${normalizedBaseUrl}`
    )
  );
  log.info(colors.green(`✓ Profile '${profileName}' is now active`));

  return newWorkspace;
}

export type Context = {
  workspace: string;
  baseUrl: string;
  urlStore: string;
  token: string;
};

async function tryResolveWorkspace(
  opts: GlobalOptions
): Promise<
  { isError: false; value: Workspace } | { isError: true; error: string }
> {
  const cache = (opts as any).__secret_workspace;
  if (cache) return { isError: false, value: cache };

  if (opts.workspace) {
    const e = await getWorkspaceByName(opts.workspace, opts.configDir);
    if (!e) {
      return {
        isError: true,
        error: colors.red.underline("Given workspace does not exist."),
      };
    }
    (opts as any).__secret_workspace = e;
    return { isError: false, value: e };
  }

  // Only check for explicit workspace, don't fallback to active workspace here
  return {
    isError: true,
    error: colors.red.underline("No explicit workspace given."),
  };
}

export async function tryResolveBranchWorkspace(
  opts: GlobalOptions,
  branchOverride?: string
): Promise<Workspace | undefined> {
  let rawBranch: string | null = null;
  let currentBranch: string;
  let originalBranchIfForked: string | null = null;
  let workspaceIdIfForked: string | null = null;

  if (branchOverride) {
    // Use branch override directly
    currentBranch = branchOverride;
    log.info(`Using branch override: ${branchOverride}`);
  } else {
    // Only try branch-based resolution if in a Git repository
    if (!isGitRepository()) {
      return undefined;
    }

    rawBranch = getCurrentGitBranch();
    if (!rawBranch) {
      return undefined;
    }

    originalBranchIfForked = getOriginalBranchForWorkspaceForks(rawBranch);
    workspaceIdIfForked =
      getWorkspaceIdForWorkspaceForkFromBranchName(rawBranch);
    if (originalBranchIfForked) {
      log.info(
        `Using original branch \`${originalBranchIfForked}\` for finding workspace profile from gitBranches section in wmill.yaml`
      );
      currentBranch = originalBranchIfForked;
    } else {
      currentBranch = rawBranch;
    }
  }

  // Read wmill.yaml to check for branch workspace configuration
  const config = await readConfigFile();
  const branchConfig = config.gitBranches?.[currentBranch];

  // Check if branch has workspace configuration
  if (!branchConfig?.baseUrl || !branchConfig?.workspaceId) {
    return undefined;
  }

  log.info(
    `Using branch configuration for branch \`${currentBranch}\` set in gitBranches`
  );

  const { baseUrl, workspaceId } = branchConfig;

  let normalizedBaseUrl: string;
  try {
    normalizedBaseUrl = new URL(baseUrl).toString();
  } catch (error) {
    log.error(
      colors.red(`Invalid baseUrl in branch configuration: ${baseUrl}`)
    );
    return undefined;
  }

  // Find all profiles matching this baseUrl and workspaceId
  const allProfiles = await allWorkspaces(opts.configDir);
  const matchingProfiles = allProfiles.filter(
    (w) => w.remote === normalizedBaseUrl && w.workspaceId === workspaceId
  );

  if (matchingProfiles.length === 0) {
    // No matching profile exists - prompt to create one
    return await createWorkspaceProfileInteractively(
      normalizedBaseUrl,
      workspaceId,
      currentBranch,
      opts,
      { rawBranch: rawBranch ?? currentBranch, isForked: !!originalBranchIfForked }
    );
  }

  // Handle multiple profiles - use special branch-aware logic
  let selectedProfile: Workspace;

  if (matchingProfiles.length === 1) {
    selectedProfile = matchingProfiles[0];
    log.info(
      colors.green(
        `Using workspace profile '${selectedProfile.name}' for branch '${currentBranch}' with workspace id \`${workspaceId}\``
      )
    );
  } else {
    // For multiple profiles, check branch-specific last used first
    const lastUsedName = await getLastUsedProfile(
      currentBranch,
      normalizedBaseUrl,
      workspaceId,
      opts.configDir
    );

    if (lastUsedName) {
      const lastUsedProfile = matchingProfiles.find(
        (p) => p.name === lastUsedName
      );
      if (lastUsedProfile) {
        log.info(
          colors.green(
            `Using workspace profile '${lastUsedProfile.name}' for branch '${currentBranch}' (last used)`
          )
        );
        return lastUsedProfile;
      }
    }

    // Fall back to general selection logic
    selectedProfile = await selectFromMultipleProfiles(
      matchingProfiles,
      normalizedBaseUrl,
      workspaceId,
      `branch '${currentBranch}'`,
      opts.configDir
    );

    // Save branch-specific selection
    await setLastUsedProfile(
      currentBranch,
      normalizedBaseUrl,
      workspaceId,
      selectedProfile.name,
      opts.configDir
    );

    log.info(
      colors.green(
        `Using workspace profile '${selectedProfile.name}' for branch '${currentBranch}'`
      )
    );
  }

  if (workspaceIdIfForked) {
    selectedProfile.name = `${selectedProfile.name}/${workspaceIdIfForked}`;
    selectedProfile.workspaceId = workspaceIdIfForked;
    log.info(
      `Inferred workspace id \`${workspaceId}\` from branch name because this is a workspace fork branch (\`${rawBranch}\`). `
    );
  }

  return selectedProfile;
}

export async function resolveWorkspace(
  opts: GlobalOptions,
  branchOverride?: string
): Promise<Workspace> {
  const cache = (opts as any).__secret_workspace;
  if (cache) return cache;

  if (opts.baseUrl) {
    if (opts.workspace && opts.token) {
      let normalizedBaseUrl: string;
      try {
        normalizedBaseUrl = new URL(opts.baseUrl).toString(); // add trailing slash if not present
      } catch (error) {
        log.info(colors.red(`Invalid base URL: ${opts.baseUrl}`));
        return process.exit(-1);
      }

      // Try to find existing workspace profile by name, then by workspaceId + remote
      if (opts.workspace) {
        // Try by workspace name first
        let existingWorkspace = await getWorkspaceByName(
          opts.workspace,
          opts.configDir
        );

        // If not found by name, try to find by workspaceId + remote match
        if (!existingWorkspace) {
          const { allWorkspaces } = await import(
            "../commands/workspace/workspace.ts"
          );
          const workspaces = await allWorkspaces(opts.configDir);
          const matchingWorkspaces = workspaces.filter(
            (w) =>
              w.workspaceId === opts.workspace && w.remote === normalizedBaseUrl
          );

          if (matchingWorkspaces.length >= 1) {
            existingWorkspace = await selectFromMultipleProfiles(
              matchingWorkspaces,
              normalizedBaseUrl,
              opts.workspace!,
              `workspace "${opts.workspace}" on ${normalizedBaseUrl}`,
              opts.configDir
            );
          }
        }

        if (existingWorkspace) {
          // Validate that the base URL matches the profile's remote
          if (existingWorkspace.remote !== normalizedBaseUrl) {
            log.info(
              colors.red(
                `Base URL mismatch: --base-url is ${normalizedBaseUrl} but workspace profile "${opts.workspace}" uses ${existingWorkspace.remote}`
              )
            );
            return process.exit(-1);
          }
          // Use the existing workspace profile (preserves workspace name)
          return {
            ...existingWorkspace,
            token: opts.token, // Use the provided token
          };
        }
      }

      // No existing profile found, create temporary workspace
      return {
        remote: normalizedBaseUrl,
        workspaceId: opts.workspace,
        name: opts.workspace,
        token: opts.token,
      };
    } else {
      log.info(
        colors.red(
          "If you specify a base URL with --base-url, you must also specify a workspace (--workspace) and token (--token)."
        )
      );
      return process.exit(-1);
    }
  }

  const branch = branchOverride ?? getCurrentGitBranch();

  // Try explicit workspace flag first (should override branch-based resolution). Unless it's a
  // forked workspace, that we detect through the branch name (only when not using branchOverride)
  const res = await tryResolveWorkspace(opts);
  if (!res.isError) {
    if (branchOverride || !branch || !branch.startsWith(WM_FORK_PREFIX)) {
      return res.value;
    } else {
      log.info(
        `Found an active workspace \`${res.value.name}\` but the branch name indicates this is a forked workspace. Ignoring active workspace and trying to resolve the correct workspace from the branch name \`${branch}\``
      );
    }
  }

  // Try branch-based resolution (medium priority)
  const branchWorkspace = await tryResolveBranchWorkspace(opts, branchOverride);
  if (branchWorkspace) {
    (opts as any).__secret_workspace = branchWorkspace;
    return branchWorkspace;
  } else if (!branchOverride) {
    // Only check for fork errors when not using branchOverride
    const originalBranch = getOriginalBranchForWorkspaceForks(branch);
    if (originalBranch) {
      log.error(
        colors.red.bold(
          `Failed to resolve workspace profile for workspace fork. This most likely means that the original branch \`${originalBranch}\` where \`${branch}\` is originally forked from, is not setup in the wmill.yaml. You need to update the \`gitBranches\` section for \`${originalBranch}\` to include workspaceId and baseUrl.`
        )
      );
      return process.exit(-1);
    }
  }

  // Fall back to active workspace (lowest priority)
  const activeWorkspace = await getActiveWorkspace(opts);
  if (activeWorkspace) {
    (opts as any).__secret_workspace = activeWorkspace;
    return activeWorkspace;
  }

  // If everything failed, show error
  log.info(colors.red.bold("No workspace given and no default set."));
  return process.exit(-1);
}

export async function fetchVersion(baseUrl: string): Promise<string> {
  const requestHeaders = new Headers();

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    for (const [key, value] of Object.entries(extraHeaders)) {
      requestHeaders.set(key, value);
    }
  }

  const response = await fetch(
    new URL(new URL(baseUrl).origin + "/api/version"),
    { headers: requestHeaders, method: "GET" }
  );

  if (!response.ok) {
    // Consume response body even on error to avoid resource leak
    await response.text();
    throw new Error(
      `Failed to fetch version: ${response.status} ${response.statusText}`
    );
  }

  return await response.text();
}
export async function tryResolveVersion(
  opts: GlobalOptions
): Promise<number | undefined> {
  if ((opts as any).__cache_version) {
    return (opts as any).__cache_version;
  }

  const workspaceRes = await tryResolveWorkspace(opts);
  if (workspaceRes.isError) return undefined;
  const version = await fetchVersion(workspaceRes.value.remote);

  try {
    return Number.parseInt(
      version.split("-", 1)[0].replaceAll(".", "").replace("v", "")
    );
  } catch {
    return undefined;
  }
}

export function validatePath(path: string): boolean {
  if (!(path.startsWith("g") || path.startsWith("u") || path.startsWith("f"))) {
    log.info(
      colors.red(
        "Given remote path looks invalid. Remote paths are typically of the form <u|g|f>/<username|group|folder>/..."
      )
    );
    return false;
  }

  return true;
}
