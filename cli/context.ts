// deno-lint-ignore-file no-explicit-any
import { colors, log, Select, Confirm, Input } from "./deps.ts";

import { loginInteractive } from "./login.ts";
import { GlobalOptions } from "./types.ts";
import { getHeaders } from "./utils.ts";
import {
  addWorkspace,
  getActiveWorkspace,
  getWorkspaceByName,
  Workspace,
  allWorkspaces,
} from "./workspace.ts";
import {
  getLastUsedProfile,
  setLastUsedProfile
} from "./branch-profiles.ts";
import { readConfigFile } from "./conf.ts";
import { getCurrentGitBranch, isGitRepository } from "./git.ts";

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
  const lastUsedProfileName = await getLastUsedProfile("", baseUrl, workspaceId, configDir);
  if (lastUsedProfileName) {
    const lastUsedProfile = profiles.find(p => p.name === lastUsedProfileName);
    if (lastUsedProfile) {
      log.info(colors.green(`Using last used profile '${lastUsedProfile.name}' for ${context}`));
      return lastUsedProfile;
    }
  }

  // No last used or it no longer exists - prompt for selection
  if (!Deno.stdin.isTerminal() || !Deno.stdout.isTerminal()) {
    const selectedProfile = profiles[0];
    log.info(colors.yellow(`Multiple profiles found for ${context}. Using first available profile: '${selectedProfile.name}'`));

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

  log.info(colors.yellow(`\nMultiple workspace profiles found for ${context}:`));

  const selectedName = await Select.prompt({
    message: "Select profile",
    options: profiles.map(p => ({
      name: `${p.name} (${p.workspaceId} on ${p.remote})`,
      value: p.name,
    })),
  });

  const selectedProfile = profiles.find(p => p.name === selectedName)!;

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

  const defaultWorkspace = await getActiveWorkspace(opts);
  if (!defaultWorkspace) {
    return {
      isError: true,
      error: colors.red.underline("No workspace given and no default set."),
    };
  }

  return { isError: false, value: defaultWorkspace };
}

async function tryResolveBranchWorkspace(
  opts: GlobalOptions
): Promise<Workspace | undefined> {
  // Only try branch-based resolution if in a Git repository
  if (!isGitRepository()) {
    return undefined;
  }

  const currentBranch = getCurrentGitBranch();
  if (!currentBranch) {
    return undefined;
  }

  // Read wmill.yaml to check for branch workspace configuration
  const config = await readConfigFile();
  const branchConfig = config.git_branches?.[currentBranch];

  // Check if branch has workspace configuration
  if (!branchConfig?.baseUrl || !branchConfig?.workspaceId) {
    return undefined;
  }

  const { baseUrl, workspaceId } = branchConfig;
  let normalizedBaseUrl: string;
  try {
    normalizedBaseUrl = new URL(baseUrl).toString();
  } catch (error) {
    log.error(colors.red(`Invalid baseUrl in branch configuration: ${baseUrl}`));
    return undefined;
  }

  // Find all profiles matching this baseUrl and workspaceId
  const allProfiles = await allWorkspaces(opts.configDir);
  const matchingProfiles = allProfiles.filter(
    w => w.remote === normalizedBaseUrl && w.workspaceId === workspaceId
  );

  if (matchingProfiles.length === 0) {
    // No matching profile exists - prompt to create one
    log.info(colors.yellow(
      `\nNo workspace profile found for branch '${currentBranch}'\n` +
      `(${normalizedBaseUrl}, ${workspaceId})`
    ));

    if (!Deno.stdin.isTerminal() || !Deno.stdout.isTerminal()) {
      log.info("Not a TTY, cannot create profile interactively. Use 'wmill workspace add' first.");
      return undefined;
    }

    const shouldCreate = await Confirm.prompt({
      message: "Would you like to create a new profile?",
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
    await setLastUsedProfile(currentBranch, normalizedBaseUrl, workspaceId, profileName, opts.configDir);

    log.info(colors.green(`✓ Created profile '${profileName}' for ${workspaceId} on ${normalizedBaseUrl}`));
    log.info(colors.green(`✓ Profile '${profileName}' is now active`));

    return newWorkspace;
  }

  // Handle multiple profiles - use special branch-aware logic
  let selectedProfile: Workspace;

  if (matchingProfiles.length === 1) {
    selectedProfile = matchingProfiles[0];
    log.info(colors.green(`Using workspace profile '${selectedProfile.name}' for branch '${currentBranch}'`));
  } else {
    // For multiple profiles, check branch-specific last used first
    const lastUsedName = await getLastUsedProfile(
      currentBranch,
      normalizedBaseUrl,
      workspaceId,
      opts.configDir
    );

    if (lastUsedName) {
      const lastUsedProfile = matchingProfiles.find(p => p.name === lastUsedName);
      if (lastUsedProfile) {
        log.info(colors.green(`Using workspace profile '${lastUsedProfile.name}' for branch '${currentBranch}' (last used)`));
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

    log.info(colors.green(`Using workspace profile '${selectedProfile.name}' for branch '${currentBranch}'`));
  }

  return selectedProfile;
}

export async function resolveWorkspace(
  opts: GlobalOptions
): Promise<Workspace> {
  if (opts.baseUrl) {
    if (opts.workspace && opts.token) {
      let normalizedBaseUrl: string;
      try {
        normalizedBaseUrl = new URL(opts.baseUrl).toString(); // add trailing slash if not present
      } catch (error) {
        log.info(colors.red(`Invalid base URL: ${opts.baseUrl}`));
        return Deno.exit(-1);
      }

      // Try to find existing workspace profile by name, then by workspaceId + remote
      if (opts.workspace) {
        // Try by workspace name first
        let existingWorkspace = await getWorkspaceByName(opts.workspace, opts.configDir);

        // If not found by name, try to find by workspaceId + remote match
        if (!existingWorkspace) {
          const workspaces = await allWorkspaces(opts.configDir);
          const matchingWorkspaces = workspaces.filter(
            w => w.workspaceId === opts.workspace && w.remote === normalizedBaseUrl
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
            return Deno.exit(-1);
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
      return Deno.exit(-1);
    }
  }

  // Try explicit workspace flag first (should override branch-based resolution)
  const res = await tryResolveWorkspace(opts);
  if (!res.isError) {
    return res.value;
  }

  // Fall back to branch-based resolution if no explicit workspace
  const branchWorkspace = await tryResolveBranchWorkspace(opts);
  if (branchWorkspace) {
    return branchWorkspace;
  }

  // If both failed, show the original error from explicit workspace resolution
  log.info(colors.red.bold(res.error));
  return Deno.exit(-1);
}


export async function fetchVersion(baseUrl: string): Promise<string> {
  const requestHeaders = new Headers();

  const extraHeaders = getHeaders();
  if (extraHeaders) {
    for (const [key, value] of Object.entries(extraHeaders)) {
      // @ts-ignore
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
    throw new Error(`Failed to fetch version: ${response.status} ${response.statusText}`);
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
