import { colors } from "@cliffy/ansi/colors";
import * as log from "./log.ts";
import { Select } from "@cliffy/prompt/select";
import { Confirm } from "@cliffy/prompt/confirm";
import { Input } from "@cliffy/prompt/input";
import { Table } from "@cliffy/table";

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
import {
  readConfigFile,
  findWorkspaceByGitBranch,
  getEffectiveWorkspaceId,
  WorkspaceEntryConfig,
} from "./conf.ts";
import {
  getCurrentGitBranch,
  getOriginalBranchForWorkspaceForks,
  getWorkspaceIdForWorkspaceForkFromBranchName,
  isGitRepository,
} from "../utils/git.ts";
import { WM_FORK_PREFIX } from "./constants.ts";
import { levenshteinDistance } from "@jsr/std__text/levenshtein-distance";

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
    // First try: look up workspace by name in wmill.yaml workspaces config
    const config = await readConfigFile({ warnIfMissing: false });
    const wsEntry = config.workspaces?.[opts.workspace] as WorkspaceEntryConfig | undefined;
    if (wsEntry?.baseUrl) {
      const workspaceId = getEffectiveWorkspaceId(opts.workspace, wsEntry);
      let normalizedBaseUrl: string;
      try {
        normalizedBaseUrl = new URL(wsEntry.baseUrl).toString();
      } catch {
        return {
          isError: true,
          error: colors.red.underline(`Invalid baseUrl in workspace '${opts.workspace}' configuration: ${wsEntry.baseUrl}`),
        };
      }

      // Find matching profile by baseUrl + workspaceId
      const allProfs = await allWorkspaces(opts.configDir);
      const matching = allProfs.filter(
        (w) => w.remote === normalizedBaseUrl && w.workspaceId === workspaceId
      );

      if (matching.length >= 1) {
        const selected = matching.length === 1
          ? matching[0]
          : await selectFromMultipleProfiles(
              matching,
              normalizedBaseUrl,
              workspaceId,
              `workspace '${opts.workspace}'`,
              opts.configDir
            );
        log.info(
          colors.green(
            `Using workspace profile '${selected.name}' for workspace '${opts.workspace}' (${workspaceId} on ${normalizedBaseUrl})`
          )
        );
        (opts as any).__secret_workspace = selected;
        return { isError: false, value: selected };
      }

      // No matching profile — offer to create one
      log.info(
        `No profile found for workspace '${opts.workspace}' (${workspaceId} on ${normalizedBaseUrl})`
      );
      const ws = await createWorkspaceProfileInteractively(
        normalizedBaseUrl,
        workspaceId,
        opts.workspace,
        opts,
        { rawBranch: opts.workspace, isForked: false }
      );
      if (ws) {
        (opts as any).__secret_workspace = ws;
        return { isError: false, value: ws };
      }
    }

    // Fall back: look up profile by name directly (old behavior)
    const e = await getWorkspaceByName(opts.workspace, opts.configDir);
    if (!e) {
      return {
        isError: true,
        error: colors.red.underline(
          `Workspace '${opts.workspace}' not found in wmill.yaml config or local profiles.`
        ),
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
  workspaceNameOverride?: string
): Promise<Workspace | undefined> {
  let rawBranch: string | null = null;
  let wsName: string | undefined;
  let wsEntry: WorkspaceEntryConfig | undefined;
  let originalBranchIfForked: string | null = null;
  let workspaceIdIfForked: string | null = null;

  // Read wmill.yaml (silent — just probing)
  const config = await readConfigFile({ warnIfMissing: false });

  if (workspaceNameOverride) {
    // Direct lookup by workspace name
    wsEntry = config.workspaces?.[workspaceNameOverride] as WorkspaceEntryConfig | undefined;
    if (wsEntry) {
      wsName = workspaceNameOverride;
      log.info(`Using workspace override: ${workspaceNameOverride}`);
    }
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

    const branchToLookup = originalBranchIfForked ?? rawBranch;
    if (originalBranchIfForked) {
      log.info(
        `Using original branch \`${originalBranchIfForked}\` for finding workspace from workspaces section in wmill.yaml`
      );
    }

    const match = findWorkspaceByGitBranch(config.workspaces, branchToLookup);
    if (match) {
      [wsName, wsEntry] = match;
    }
  }

  if (!wsName || !wsEntry) {
    return undefined;
  }

  if (!wsEntry.baseUrl) {
    if (workspaceNameOverride) {
      // User explicitly asked for this workspace but it has no baseUrl
      log.warn(
        `⚠️  Workspace '${wsName}' has no baseUrl configured. Cannot resolve a profile.\n` +
        `   Add baseUrl to workspace '${wsName}' in wmill.yaml, or use --base-url flag.`
      );
    }
    return undefined;
  }

  const workspaceId = getEffectiveWorkspaceId(wsName, wsEntry);
  const effectiveGitBranch = (wsEntry as any).gitBranch ?? wsName;
  const { baseUrl } = wsEntry;

  // Explain why this workspace was selected
  let reason: string;
  if (workspaceNameOverride) {
    reason = `selected via --workspace`;
  } else if (originalBranchIfForked) {
    reason = `matched via fork branch '${rawBranch}' → original branch '${originalBranchIfForked}'`;
  } else if (effectiveGitBranch !== wsName) {
    reason = `matched git branch '${effectiveGitBranch}' on current branch '${rawBranch}'`;
  } else {
    reason = `matched current git branch '${rawBranch}'`;
  }

  log.info(
    `Using workspace '${wsName}' (${reason}) → ${workspaceId} on ${baseUrl}`
  );

  let normalizedBaseUrl: string;
  try {
    normalizedBaseUrl = new URL(baseUrl).toString();
  } catch (error) {
    log.error(
      colors.red(`Invalid baseUrl in workspace '${wsName}' configuration: ${baseUrl}`)
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
      wsName,
      opts,
      { rawBranch: rawBranch ?? wsName, isForked: !!originalBranchIfForked }
    );
  }

  // Handle multiple profiles
  let selectedProfile: Workspace;

  if (matchingProfiles.length === 1) {
    selectedProfile = matchingProfiles[0];
    log.info(
      colors.green(
        `Using workspace profile '${selectedProfile.name}' for workspace '${wsName}' with workspace id \`${workspaceId}\``
      )
    );
  } else {
    const lastUsedName = await getLastUsedProfile(
      wsName,
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
            `Using workspace profile '${lastUsedProfile.name}' for workspace '${wsName}' (last used)`
          )
        );
        return lastUsedProfile;
      }
    }

    selectedProfile = await selectFromMultipleProfiles(
      matchingProfiles,
      normalizedBaseUrl,
      workspaceId,
      `workspace '${wsName}'`,
      opts.configDir
    );

    await setLastUsedProfile(
      wsName,
      normalizedBaseUrl,
      workspaceId,
      selectedProfile.name,
      opts.configDir
    );

    log.info(
      colors.green(
        `Using workspace profile '${selectedProfile.name}' for workspace '${wsName}'`
      )
    );
  }

  if (workspaceIdIfForked) {
    selectedProfile.name = `${selectedProfile.name}/${workspaceIdIfForked}`;
    selectedProfile.workspaceId = workspaceIdIfForked;
    log.info(
      `Using fork workspace \`${workspaceIdIfForked}\` (parent: \`${workspaceId}\`) from branch \`${rawBranch}\``
    );
  }

  return selectedProfile;
}

export async function resolveWorkspace(
  opts: GlobalOptions,
  workspaceNameOverride?: string
): Promise<Workspace> {
  const cache = (opts as any).__secret_workspace;
  if (cache) return cache;

  if (opts.baseUrl) {
    if (opts.workspace && opts.token) {
      let normalizedBaseUrl: string;
      try {
        normalizedBaseUrl = new URL(opts.baseUrl).toString();
      } catch (error) {
        log.info(colors.red(`Invalid base URL: ${opts.baseUrl}`));
        return process.exit(-1);
      }

      // Try to find existing workspace profile by name, then by workspaceId + remote
      if (opts.workspace) {
        let existingWorkspace = await getWorkspaceByName(
          opts.workspace,
          opts.configDir
        );

        if (!existingWorkspace) {
          const { allWorkspaces } = await import(
            "../commands/workspace/workspace.ts"
          );
          const profiles = await allWorkspaces(opts.configDir);
          const matchingWorkspaces = profiles.filter(
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
          if (existingWorkspace.remote !== normalizedBaseUrl) {
            log.info(
              colors.red(
                `Base URL mismatch: --base-url is ${normalizedBaseUrl} but workspace profile "${opts.workspace}" uses ${existingWorkspace.remote}`
              )
            );
            return process.exit(-1);
          }
          return {
            ...existingWorkspace,
            token: opts.token,
          };
        }
      }

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

  const branch = workspaceNameOverride ? null : getCurrentGitBranch();

  // Try explicit workspace flag first (should override branch-based resolution). Unless it's a
  // forked workspace, that we detect through the branch name (only when not using workspaceNameOverride
  // and --workspace was not explicitly provided)
  const res = await tryResolveWorkspace(opts);
  if (!res.isError) {
    const workspace = (res as { isError: false; value: Workspace }).value;
    if (workspaceNameOverride || opts.workspace || !branch || !branch.startsWith(WM_FORK_PREFIX)) {
      return workspace;
    } else {
      log.info(
        `Found an active workspace \`${workspace.name}\` but the branch name indicates this is a forked workspace. Ignoring active workspace and trying to resolve the correct workspace from the branch name \`${branch}\`. Use --workspace to override.`
      );
    }
  } else if (opts.workspace) {
    // --workspace was explicitly provided but not found — fail immediately
    const profiles = await allWorkspaces(opts.configDir);
    const names = profiles.map((w) => w.name);
    let msg = `Workspace "${opts.workspace}" not found.`;
    const suggestions = names
      .map((n) => ({ name: n, dist: levenshteinDistance(opts.workspace!, n) }))
      .filter((s) => s.dist <= 3)
      .sort((a, b) => a.dist - b.dist)
      .slice(0, 3);
    if (suggestions.length > 0) {
      msg += ` Did you mean: ${suggestions.map((s) => `"${s.name}"`).join(", ")}?`;
    }
    log.info(colors.red.bold(msg));
    if (profiles.length > 0) {
      log.info("\nAvailable workspaces:");
      new Table()
        .header(["name", "remote", "workspace id"])
        .padding(2)
        .border(true)
        .body(profiles.map((w) => [w.name, w.remote, w.workspaceId]))
        .render();
    }
    return process.exit(-1);
  }

  // Try branch-based resolution (medium priority)
  const branchWorkspace = await tryResolveBranchWorkspace(opts, workspaceNameOverride);
  if (branchWorkspace) {
    (opts as any).__secret_workspace = branchWorkspace;
    return branchWorkspace;
  } else if (!workspaceNameOverride) {
    // Only check for fork errors when not using workspaceNameOverride
    const originalBranch = getOriginalBranchForWorkspaceForks(branch);
    if (originalBranch) {
      log.error(
        colors.red.bold(
          `Failed to resolve workspace profile for workspace fork. The original branch \`${originalBranch}\` (forked from \`${branch}\`) is not configured in wmill.yaml.\n` +
            `   Add to the 'workspaces' section: \`${originalBranch}: { baseUrl: "https://...", workspaceId: "..." }\``
        )
      );
      return process.exit(-1);
    }
  }

  // If workspaces config exists, use it rather than falling back to active profile
  const config = await readConfigFile({ warnIfMissing: false });
  const { getWorkspaceNames } = await import("./conf.ts");
  const wsNames = getWorkspaceNames(config.workspaces);

  if (wsNames.length > 0) {
    let pickedWsName: string;

    const wsListStr = wsNames.map((n) => {
      const entry = (config.workspaces as any)[n];
      const info = entry.baseUrl ? ` (${entry.workspaceId ?? n} on ${entry.baseUrl})` : "";
      return `  - ${n}${info}`;
    }).join("\n");

    if (wsNames.length === 1) {
      pickedWsName = wsNames[0];
      log.info(
        `Auto-selected workspace '${pickedWsName}' (only workspace in config).\n` +
        `Use --workspace to override or 'wmill workspace bind' to add more workspaces.`
      );
    } else if (process.stdin.isTTY) {
      log.info(
        `Multiple workspaces configured but none matched the current context.\n` +
        `Configured workspaces:\n${wsListStr}\n` +
        `Use --workspace to skip this prompt.`
      );
      pickedWsName = await Select.prompt({
        message: "Select workspace",
        options: wsNames.map((n) => {
          const entry = (config.workspaces as any)[n];
          const info = entry.baseUrl ? ` (${entry.workspaceId ?? n} on ${entry.baseUrl})` : "";
          return { name: `${n}${info}`, value: n };
        }),
      });
    } else {
      log.error(
        colors.red.bold(
          `Multiple workspaces configured but none matched the current context.\n` +
          `Configured workspaces:\n${wsListStr}\n` +
          `Use --workspace to select one.`
        )
      );
      return process.exit(-1);
    }

    // Resolve the picked workspace via config
    const pickedResult = await tryResolveBranchWorkspace(opts, pickedWsName);
    if (pickedResult) {
      (opts as any).__secret_workspace = pickedResult;
      return pickedResult;
    }
  }

  // Fall back to active workspace (only when no workspaces config)
  const activeWorkspace = await getActiveWorkspace(opts);
  if (activeWorkspace) {
    (opts as any).__secret_workspace = activeWorkspace;
    return activeWorkspace;
  }

  // Last resort: auto-configure from Windmill environment variables
  const envWorkspace = process.env["WM_WORKSPACE"];
  const envToken = process.env["WM_TOKEN"];
  const envBaseUrl =
    process.env["BASE_INTERNAL_URL"] ?? process.env["BASE_URL"];

  if (envWorkspace && envToken && envBaseUrl) {
    let normalizedBaseUrl: string;
    try {
      normalizedBaseUrl = new URL(envBaseUrl).toString();
    } catch {
      log.info(colors.red(`Invalid BASE_INTERNAL_URL: ${envBaseUrl}`));
      return process.exit(-1);
    }
    log.debug(
      `Using workspace from environment variables: ${envWorkspace} on ${normalizedBaseUrl}`
    );
    const ws: Workspace = {
      name: envWorkspace,
      workspaceId: envWorkspace,
      remote: normalizedBaseUrl,
      token: envToken,
    };
    (opts as any).__secret_workspace = ws;
    return ws;
  }

  log.info(colors.red.bold("No workspace given and no default set. Run 'wmill workspace add' to configure one."));
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
  const workspace = (workspaceRes as { isError: false; value: Workspace }).value;
  const version = await fetchVersion(workspace.remote);

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
