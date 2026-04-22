import * as log from "./log.ts";
import { yamlParseFile } from "../utils/yaml.ts";
import { Confirm } from "@cliffy/prompt/confirm";
import { stringify as yamlStringify } from "yaml";
import {
  getCurrentGitBranch,
  getOriginalBranchForWorkspaceForks,
  isGitRepository,
} from "../utils/git.ts";
import { join, dirname, resolve, relative } from "node:path";
import { existsSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { execSync } from "node:child_process";
import { setNonDottedPaths } from "../utils/resource_folders.ts";

export let showDiffs = false;
export function setShowDiffs(value: boolean) {
  showDiffs = value;
}

export interface SpecificItemsConfig_Yaml {
  variables?: string[];
  resources?: string[];
  triggers?: string[];
  schedules?: string[];
  folders?: string[];
  settings?: boolean;
}

export interface WorkspaceEntryConfig extends SyncOptions {
  gitBranch?: string;
  workspaceId?: string;
  baseUrl?: string;
  overrides?: Partial<SyncOptions>;
  promotionOverrides?: Partial<SyncOptions>;
  specificItems?: SpecificItemsConfig_Yaml;
}

export type WorkspacesConfig = {
  commonSpecificItems?: SpecificItemsConfig_Yaml;
} & {
  [workspaceName: string]: WorkspaceEntryConfig;
};

// Legacy type alias for backward compat
type LegacyBranchesConfig = {
  commonSpecificItems?: SpecificItemsConfig_Yaml;
} & {
  [branchName: string]: SyncOptions & {
    overrides?: Partial<SyncOptions>;
    promotionOverrides?: Partial<SyncOptions>;
    baseUrl?: string;
    workspaceId?: string;
    specificItems?: SpecificItemsConfig_Yaml;
  };
};

export const SUPPORTED_SYNC_BEHAVIOR_VERSION = 1;

export function parseSyncBehavior(value?: string | number): number {
  if (!value && value !== 0) return 0;
  const s = String(value);
  const match = s.match(/^v(\d+)$/);
  return match ? parseInt(match[1], 10) : 0;
}

export interface SyncOptions {
  stateful?: boolean;
  raw?: boolean;
  yes?: boolean;
  dryRun?: boolean;
  skipPull?: boolean;
  failConflicts?: boolean;
  plainSecrets?: boolean;
  json?: boolean;
  skipVariables?: boolean;
  skipResources?: boolean;
  skipResourceTypes?: boolean;
  skipSecrets?: boolean;
  skipWorkspaceDependencies?: boolean;
  skipScripts?: boolean;
  skipFlows?: boolean;
  skipApps?: boolean;
  skipFolders?: boolean;
  includeSchedules?: boolean;
  includeTriggers?: boolean;
  includeUsers?: boolean;
  includeGroups?: boolean;
  includeSettings?: boolean;
  includeKey?: boolean;
  skipBranchValidation?: boolean;
  message?: string;
  includes?: string[];
  extraIncludes?: string[];
  excludes?: string[];
  defaultTs?: "bun" | "deno";
  codebases?: Codebase[];
  parallel?: number;
  jsonOutput?: boolean;
  nonDottedPaths?: boolean;
  // Primary config key — maps workspace names to workspace configurations
  workspaces?: WorkspacesConfig;
  // Deprecated aliases — normalized to `workspaces` in readConfigFile
  gitBranches?: LegacyBranchesConfig;
  environments?: LegacyBranchesConfig;
  git_branches?: LegacyBranchesConfig;
  promotion?: string;
  lint?: boolean;
  locksRequired?: boolean;
  syncBehavior?: string;
}

export interface Codebase {
  relative_path: string;
  includes?: string[];
  excludes?: string[];
  assets?: {
    from: string;
    to: string;
  }[];
  customBundler?: string;
  external?: string[];
  define?: { [key: string]: string };
  inject?: string[];
  loader?: any;
  format?: "cjs" | "esm";
  banner?: {
    [type: string]: string;
};
}

function getGitRepoRoot(): string | null {
  try {
    const result = execSync("git rev-parse --show-toplevel", {
      encoding: "utf8",
      stdio: "pipe",
    });
    return result.trim();
  } catch (error) {
    return null;
  }
}

export const GLOBAL_CONFIG_OPT = { noCdToRoot: false };
function findWmillYaml(): string | null {
  const startDir = resolve(process.cwd());
  const isInGitRepo = isGitRepository();
  const gitRoot = isInGitRepo ? getGitRepoRoot() : null;

  let currentDir = startDir;
  let foundPath: string | null = null;

  // Search upward for wmill.yaml until we find it, reach git root, or reach filesystem root
  while (true) {
    const wmillYamlPath = join(currentDir, "wmill.yaml");

    if (existsSync(wmillYamlPath)) {
      foundPath = wmillYamlPath;
      break;
    }

    // Check if we've reached the git repository root
    if (gitRoot && resolve(currentDir) === resolve(gitRoot)) {
      break;
    }

    // Check if we've reached the filesystem root
    const parentDir = dirname(currentDir);
    if (parentDir === currentDir) {
      break;
    }

    currentDir = parentDir;
  }

  // If wmill.yaml was found in a parent directory, warn the user and change working directory
  if (
    !GLOBAL_CONFIG_OPT.noCdToRoot &&
    foundPath &&
    resolve(dirname(foundPath)) !== resolve(startDir)
  ) {
    const configDir = dirname(foundPath);
    const relativePath = relative(startDir, foundPath);
    log.warn(`⚠️  wmill.yaml found in parent directory: ${relativePath}`);

    // Change working directory to where wmill.yaml was found
    process.chdir(configDir);
    log.info(`📁 Changed working directory to: ${configDir}`);
  }

  return foundPath;
}

export function getWmillYamlPath(): string | null {
  return findWmillYaml();
}

let legacyConfigWarned = false;

export async function readConfigFile(opts?: { warnIfMissing?: boolean }): Promise<SyncOptions> {
  const warnIfMissing = opts?.warnIfMissing ?? true;
  try {
    // First, try to find wmill.yaml recursively
    const wmillYamlPath = findWmillYaml();

    if (!wmillYamlPath) {
      if (warnIfMissing) {
        log.warn(
          "No wmill.yaml found. Use 'wmill init' to bootstrap it."
        );
      }
      return {};
    }

    const conf = (await yamlParseFile(wmillYamlPath)) as SyncOptions;

    // Handle obsolete overrides format
    if (conf && "overrides" in conf) {
      const overrides = conf.overrides as any;
      const hasSettings =
        overrides &&
        typeof overrides === "object" &&
        Object.keys(overrides).length > 0;

      if (hasSettings) {
        throw new Error(
          "❌ The 'overrides' field is no longer supported.\n" +
            "   The configuration system now uses workspace-based configuration.\n" +
            "   Please delete your wmill.yaml and run 'wmill init' to recreate it with the new format."
        );
      } else {
        delete conf.overrides;
      }
    }

    // Normalize legacy formats to `workspaces` in memory (no file rewrite).
    // Priority: workspaces > gitBranches > environments > git_branches
    if (conf && !conf.workspaces) {
      let legacyKey: string | null = null;
      let legacyData: LegacyBranchesConfig | undefined;

      if (conf.gitBranches) {
        legacyKey = "gitBranches";
        legacyData = conf.gitBranches;
      } else if (conf.environments) {
        legacyKey = "environments";
        legacyData = conf.environments;
      } else if (conf.git_branches) {
        legacyKey = "git_branches";
        legacyData = conf.git_branches;
      }

      if (legacyKey && legacyData) {
        conf.workspaces = convertGitBranchesToWorkspaces(legacyData);
        if (!legacyConfigWarned) {
          log.warn(
            `⚠️  '${legacyKey}' in wmill.yaml is deprecated. Use 'workspaces' instead.\n` +
            `   Run 'wmill config migrate' to update your configuration automatically.`
          );
          legacyConfigWarned = true;
        }
      }
    } else if (conf?.workspaces) {
      // If both workspaces and any legacy key exist, warn and use workspaces
      for (const legacyKey of ["gitBranches", "environments", "git_branches"] as const) {
        if ((conf as any)[legacyKey]) {
          log.warn(
            `⚠️  Both 'workspaces' and '${legacyKey}' found in wmill.yaml. Using 'workspaces' and ignoring '${legacyKey}'.`
          );
        }
      }
    }
    // Clean legacy keys from in-memory config
    delete conf?.gitBranches;
    delete (conf as any)?.environments;
    delete conf?.git_branches;

    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }

    // Initialize global nonDottedPaths setting from config
    setNonDottedPaths(conf?.nonDottedPaths ?? false);

    const syncBehaviorVersion = parseSyncBehavior(conf?.syncBehavior);
    if (syncBehaviorVersion > SUPPORTED_SYNC_BEHAVIOR_VERSION) {
      log.error(
        `Your wmill.yaml specifies syncBehavior: ${conf!.syncBehavior}, but this CLI only supports up to v${SUPPORTED_SYNC_BEHAVIOR_VERSION}. Run 'wmill upgrade' to update.`
      );
      process.exit(1);
    }

    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    if (
      e instanceof Error &&
      (e.message.includes("overrides") ||
        e.message.includes("Obsolete configuration format"))
    ) {
      throw e; // Re-throw the specific obsolete format error
    }

    // Since we already found the file path, this is likely a parsing or access error
    if (e instanceof Error && e.message.includes("Error parsing yaml")) {
      const yamlError =
        e.cause instanceof Error ? e.cause.message : String(e.cause);
      throw new Error(
        "❌ YAML syntax error in wmill.yaml:\n" +
          "   " +
          yamlError +
          "\n" +
          "   Please fix the YAML syntax in wmill.yaml or delete the file to start fresh."
      );
    } else {
      // File exists but has other issues (permissions, etc.)
      throw new Error(
        "❌ Failed to read wmill.yaml:\n" +
          "   " +
          (e instanceof Error ? e.message : String(e)) +
          "\n" +
          "   Please check file permissions or fix the syntax."
      );
    }
  }
}

// Default sync options - shared across the codebase to prevent duplication
export const DEFAULT_SYNC_OPTIONS: Readonly<
  Required<
    Pick<
      SyncOptions,
      | "defaultTs"
      | "includes"
      | "excludes"
      | "codebases"
      | "skipVariables"
      | "skipResources"
      | "skipResourceTypes"
      | "skipSecrets"
      | "includeSchedules"
      | "includeTriggers"
      | "skipWorkspaceDependencies"
      | "skipScripts"
      | "skipFlows"
      | "skipApps"
      | "skipFolders"
      | "includeUsers"
      | "includeGroups"
      | "includeSettings"
      | "includeKey"
      | "nonDottedPaths"
      | "syncBehavior"
    >
  >
> = {
  defaultTs: "bun",
  includes: ["f/**"],
  excludes: [],
  codebases: [],
  skipVariables: false,
  skipResources: false,
  skipResourceTypes: false,
  skipSecrets: true,
  skipScripts: false,
  skipFlows: false,
  skipApps: false,
  skipFolders: false,
  includeSchedules: false,
  includeTriggers: false,
  includeUsers: false,
  includeGroups: false,
  includeSettings: false,
  includeKey: false,
  skipWorkspaceDependencies: false,
  nonDottedPaths: false,
  syncBehavior: "v1",
} as const;

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile ?? {}, opts);
}

// Validate workspace configuration early in the process
export async function validateBranchConfiguration(
  opts: Pick<SyncOptions, "skipBranchValidation" | "yes">,
  workspaceNameOverride?: string
): Promise<void> {
  // When workspace override is provided, skip validation - user is explicitly specifying the workspace
  if (opts.skipBranchValidation || workspaceNameOverride || !isGitRepository()) {
    return;
  }

  const config = await readConfigFile();
  const { workspaces } = config;

  const rawBranch = getCurrentGitBranch();

  const originalBranchIfForked = getOriginalBranchForWorkspaceForks(rawBranch);

  let currentBranch: string | null;
  if (originalBranchIfForked) {
    log.info(
      `Workspace fork detected from branch name \`${rawBranch}\`. Validating workspace configuration using original branch \`${originalBranchIfForked}\``
    );
    currentBranch = originalBranchIfForked;
  } else {
    currentBranch = rawBranch;
  }

  // In a git repository, workspaces section is recommended
  if (!workspaces || getWorkspaceNames(workspaces).length === 0) {
    log.warn(
      "⚠️  WARNING: In a Git repository, the 'workspaces' section is recommended in wmill.yaml.\n" +
        "   Consider adding a workspaces section to map workspace names to Windmill instances.\n" +
        "   Run 'wmill init' to recreate the configuration file with proper workspace setup."
    );
    return;
  }

  // Warn if multiple workspaces map to the same git branch
  const branchToWsNames = new Map<string, string[]>();
  for (const name of getWorkspaceNames(workspaces)) {
    const entry = workspaces[name] as WorkspaceEntryConfig;
    const branch = getEffectiveGitBranch(name, entry);
    const existing = branchToWsNames.get(branch);
    if (existing) {
      existing.push(name);
    } else {
      branchToWsNames.set(branch, [name]);
    }
  }
  for (const [branch, names] of branchToWsNames) {
    if (names.length > 1) {
      log.warn(
        `⚠️  WARNING: Multiple workspaces map to git branch '${branch}': ${names.join(", ")}.\n` +
          `   Only the first ('${names[0]}') will be used during auto-detection. Use --workspace to select explicitly.`
      );
    }
  }

  // Current branch must match a configured workspace's gitBranch
  if (currentBranch && !findWorkspaceByGitBranch(workspaces, currentBranch)) {
    const wsNames = getWorkspaceNames(workspaces);
    const availableInfo = wsNames
      .map((n) => {
        const entry = workspaces[n] as WorkspaceEntryConfig;
        const branch = getEffectiveGitBranch(n, entry);
        return branch !== n ? `${n} (gitBranch: ${branch})` : n;
      })
      .join(", ");

    // In interactive mode, offer to create a workspace entry
    if (!!process.stdin.isTTY) {
      log.info(
        `Current Git branch '${currentBranch}' does not match any workspace in the configuration.\n` +
          `Available workspaces: ${availableInfo}`
      );

      const shouldCreate =
        opts.yes ||
        (await Confirm.prompt({
          message: `Create empty workspace configuration for branch '${currentBranch}'?`,
          default: true,
        }));

      if (shouldCreate) {
        // Warn if branch name contains filesystem-unsafe characters
        if (/[\/\\:*?"<>|.]/.test(currentBranch)) {
          const sanitizedBranchName = currentBranch.replace(
            /[\/\\:*?"<>|.]/g,
            "_"
          );
          log.warn(
            `⚠️  WARNING: Branch name "${currentBranch}" contains filesystem-unsafe characters (/ \\ : * ? " < > | .).`
          );
          log.warn(
            `   Branch-specific files will be saved with sanitized name: "${sanitizedBranchName}"`
          );
          log.warn(
            `   Example: "file.variable.yaml" → "file.${sanitizedBranchName}.variable.yaml"`
          );
        }

        // Read current config, add workspace entry, and write it back
        const currentConfig = await readConfigFile();

        if (!currentConfig.workspaces) {
          currentConfig.workspaces = {} as WorkspacesConfig;
        }
        (currentConfig.workspaces as any)[currentBranch] = {};

        await writeFile("wmill.yaml", yamlStringify(currentConfig), "utf-8");

        log.info(
          `✅ Created empty workspace configuration for '${currentBranch}'`
        );
      } else {
        log.warn(
          "⚠️  WARNING: Workspace creation cancelled. You can manually add a workspace to the 'workspaces' section in wmill.yaml or use 'wmill gitsync-settings pull' to pull configuration from an existing windmill workspace git-sync configuration."
        );
        return;
      }
    } else {
      // Warn about filesystem-unsafe characters in branch name
      if (/[\/\\:*?"<>|.]/.test(currentBranch)) {
        const sanitizedBranchName = currentBranch.replace(
          /[\/\\:*?"<>|.]/g,
          "_"
        );
        log.warn(
          `⚠️  WARNING: Branch name "${currentBranch}" contains filesystem-unsafe characters (/ \\ : * ? " < > | .).`
        );
        log.warn(
          `   Branch-specific files will use sanitized name: "${sanitizedBranchName}"`
        );
      }

      log.warn(
        `⚠️  WARNING: Current Git branch '${currentBranch}' does not match any workspace in the configuration.\n` +
          `   Consider adding a workspace entry for branch '${currentBranch}' in the 'workspaces' section of wmill.yaml.\n` +
          `   Available workspaces: ${availableInfo}`
      );
      return;
    }
  }
}

// Get effective settings by merging top-level settings with workspace-specific overrides.
// workspaceNameOverride selects a workspace by name directly.
// When not provided, auto-detects from the current git branch.
export async function getEffectiveSettings(
  config: SyncOptions,
  promotion?: string,
  skipBranchValidation?: boolean,
  suppressLogs?: boolean,
  workspaceNameOverride?: string
): Promise<SyncOptions> {
  // Start with top-level settings from config
  const { workspaces, ...topLevelSettings } = config;
  const effective = { ...topLevelSettings };

  // Resolve the workspace entry to use
  let resolvedWsName: string | null = null;
  let resolvedWsEntry: WorkspaceEntryConfig | null = null;
  let originalBranchIfForked: string | null = null;
  let rawGitBranch: string | null = null;

  if (workspaceNameOverride) {
    // Direct lookup by workspace name
    if (workspaces && (workspaces as any)[workspaceNameOverride]) {
      resolvedWsName = workspaceNameOverride;
      resolvedWsEntry = (workspaces as any)[workspaceNameOverride] as WorkspaceEntryConfig;
    }
  } else if (isGitRepository()) {
    rawGitBranch = getCurrentGitBranch();
    originalBranchIfForked = getOriginalBranchForWorkspaceForks(rawGitBranch);

    const branch = originalBranchIfForked ?? rawGitBranch;
    if (originalBranchIfForked) {
      log.info(
        `Using overrides from original branch \`${originalBranchIfForked}\``
      );
    }

    if (branch) {
      const match = findWorkspaceByGitBranch(workspaces, branch);
      if (match) {
        [resolvedWsName, resolvedWsEntry] = match;
      }
    }
  } else {
    log.debug("Not in a Git repository and no workspace override provided, using top-level settings");
  }

  // If promotion is specified, use that branch's promotionOverrides or overrides
  if (promotion && workspaces) {
    const promotionMatch = findWorkspaceByGitBranch(workspaces, promotion);
    if (promotionMatch) {
      const [, targetWs] = promotionMatch;

      // First try promotionOverrides, then fall back to overrides
      if (targetWs.promotionOverrides) {
        Object.assign(effective, targetWs.promotionOverrides);
        if (!suppressLogs) {
          log.info(`Applied promotion settings from workspace for branch: ${promotion}`);
        }
      } else if (targetWs.overrides) {
        Object.assign(effective, targetWs.overrides);
        if (!suppressLogs) {
          log.info(
            `Applied settings from workspace for branch: ${promotion} (no promotionOverrides found)`
          );
        }
      } else {
        log.debug(
          `No promotion or regular overrides found for '${promotion}', using top-level settings`
        );
      }
    }
  }
  // Otherwise use resolved workspace's overrides
  else if (resolvedWsEntry?.overrides) {
    Object.assign(effective, resolvedWsEntry.overrides);
    if (!suppressLogs) {
      const extraLog = originalBranchIfForked
        ? ` (because it is the origin of the workspace fork branch \`${rawGitBranch}\`)`
        : "";
      log.info(
        `Applied settings for workspace '${resolvedWsName}'${extraLog}`
      );
    }
  } else if (resolvedWsName) {
    log.debug(
      `No overrides found for workspace '${resolvedWsName}', using top-level settings`
    );
  }

  return effective;
}

const RESERVED_WORKSPACE_KEYS = new Set(["commonSpecificItems"]);

/**
 * Find a workspace config entry whose effective git branch matches branchName.
 * Returns [workspaceName, config] or undefined.
 */
export function findWorkspaceByGitBranch(
  workspaces: WorkspacesConfig | undefined,
  branchName: string
): [string, WorkspaceEntryConfig] | undefined {
  if (!workspaces) return undefined;
  for (const [name, entry] of Object.entries(workspaces)) {
    if (RESERVED_WORKSPACE_KEYS.has(name)) continue;
    const effectiveBranch = (entry as WorkspaceEntryConfig).gitBranch ?? name;
    if (effectiveBranch === branchName) {
      return [name, entry as WorkspaceEntryConfig];
    }
  }
  return undefined;
}

/** Get the effective workspaceId for a workspace entry (defaults to workspace name). */
export function getEffectiveWorkspaceId(
  workspaceName: string,
  config: WorkspaceEntryConfig
): string {
  return config.workspaceId ?? workspaceName;
}

/** Get the effective git branch for a workspace entry (defaults to workspace name). */
export function getEffectiveGitBranch(
  workspaceName: string,
  config: WorkspaceEntryConfig
): string {
  return config.gitBranch ?? workspaceName;
}

/** Get all workspace names from config, excluding reserved keys like commonSpecificItems. */
export function getWorkspaceNames(
  workspaces: WorkspacesConfig | undefined
): string[] {
  if (!workspaces) return [];
  return Object.keys(workspaces).filter((k) => !RESERVED_WORKSPACE_KEYS.has(k));
}

/**
 * Convert legacy gitBranches/environments format to the new workspaces format.
 * Since old keys were branch names, workspace name = branch name.
 * gitBranch is not set (defaults to key name). All existing fields are preserved.
 */
export function convertGitBranchesToWorkspaces(
  gitBranches: LegacyBranchesConfig
): WorkspacesConfig {
  const workspaces: Record<string, any> = {};
  for (const [key, value] of Object.entries(gitBranches)) {
    if (key === "commonSpecificItems") {
      workspaces.commonSpecificItems = value;
      continue;
    }
    // Copy the entire entry as-is. Old format keys = branch names, so gitBranch
    // doesn't need to be set (defaults to key name). All SyncOptions fields,
    // overrides, promotionOverrides, specificItems, baseUrl, workspaceId are preserved.
    workspaces[key] = { ...value };
  }
  return workspaces as WorkspacesConfig;
}
