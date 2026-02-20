import { log, yamlParseFile, Confirm, yamlStringify } from "../../deps.ts";
import {
  getCurrentGitBranch,
  getOriginalBranchForWorkspaceForks,
  isGitRepository,
} from "../utils/git.ts";
import { join, dirname, resolve, relative } from "node:path";
import { existsSync } from "node:fs";
import { execSync } from "node:child_process";
import { setNonDottedPaths } from "../utils/resource_folders.ts";

export let showDiffs = false;
export function setShowDiffs(value: boolean) {
  showDiffs = value;
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
  gitBranches?: {
    commonSpecificItems?: {
      variables?: string[];
      resources?: string[];
      triggers?: string[];
      folders?: string[];
      settings?: boolean;
    };
  } & {
    [branchName: string]: SyncOptions & {
      overrides?: Partial<SyncOptions>;
      promotionOverrides?: Partial<SyncOptions>;
      baseUrl?: string;
      workspaceId?: string;
      specificItems?: {
        variables?: string[];
        resources?: string[];
        triggers?: string[];
        folders?: string[];
        settings?: boolean;
      };
    };
  };
  // Legacy field - deprecated, use gitBranches instead
  git_branches?: {
    commonSpecificItems?: {
      variables?: string[];
      resources?: string[];
      triggers?: string[];
      folders?: string[];
      settings?: boolean;
    };
  } & {
    [branchName: string]: SyncOptions & {
      overrides?: Partial<SyncOptions>;
      promotionOverrides?: Partial<SyncOptions>;
      baseUrl?: string;
      workspaceId?: string;
      specificItems?: {
        variables?: string[];
        resources?: string[];
        triggers?: string[];
        folders?: string[];
        settings?: boolean;
      };
    };
  };
  promotion?: string;
  lint?: boolean;
  locksRequired?: boolean;
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
  const startDir = resolve(Deno.cwd());
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
    Deno.chdir(configDir);
    log.info(`📁 Changed working directory to: ${configDir}`);
  }

  return foundPath;
}

export function getWmillYamlPath(): string | null {
  return findWmillYaml();
}

export async function readConfigFile(): Promise<SyncOptions> {
  try {
    // First, try to find wmill.yaml recursively
    const wmillYamlPath = findWmillYaml();

    if (!wmillYamlPath) {
      log.warn(
        "No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime."
      );
      return {};
    }

    const conf = (await yamlParseFile(wmillYamlPath)) as SyncOptions;

    // Handle legacy format migrations (combine overrides and git_branches)
    let needsConfigWrite = false;
    const migrationMessages: string[] = [];

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
            "   The configuration system now uses Git branch-based configuration only.\n" +
            "   Please delete your wmill.yaml and run 'wmill init' to recreate it with the new format."
        );
      } else {
        // Remove empty overrides
        delete conf.overrides;
        needsConfigWrite = true;
        migrationMessages.push(
          "ℹ️  Removing empty 'overrides: {}' from wmill.yaml (migrated to gitBranches format)"
        );
      }
    }

    // Handle git_branches to gitBranches migration
    if (conf && "git_branches" in conf) {
      if (!conf.gitBranches) {
        // Deep copy git_branches to gitBranches (even if empty)
        conf.gitBranches = JSON.parse(JSON.stringify(conf.git_branches));
        needsConfigWrite = true;
        migrationMessages.push(
          "⚠️  Migrating 'git_branches' to 'gitBranches' (camelCase). The snake_case format is deprecated."
        );
        migrationMessages.push(
          "✅ Successfully migrated 'git_branches' to 'gitBranches' in wmill.yaml"
        );
      } else {
        migrationMessages.push(
          "⚠️  Both 'git_branches' and 'gitBranches' found in wmill.yaml. Using 'gitBranches' and ignoring 'git_branches'."
        );
      }
      // Always remove the old field from config object (both file and memory)
      delete conf.git_branches;
    }

    // Perform single atomic write if any migrations are needed
    if (needsConfigWrite) {
      try {
        await Deno.writeTextFile(wmillYamlPath, yamlStringify(conf));
        // Log all migration messages after successful write
        migrationMessages.forEach((msg) => {
          if (msg.startsWith("⚠️")) {
            log.warn(msg);
          } else {
            log.info(msg);
          }
        });
      } catch (error) {
        log.warn(
          `Could not update wmill.yaml to apply migrations: ${
            error instanceof Error ? error.message : error
          }`
        );
      }
    } else if (migrationMessages.length > 0) {
      // Log messages for non-write cases (like "both found")
      migrationMessages.forEach((msg) => {
        if (msg.startsWith("⚠️")) {
          log.warn(msg);
        } else {
          log.info(msg);
        }
      });
    }

    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }

    // Initialize global nonDottedPaths setting from config
    setNonDottedPaths(conf?.nonDottedPaths ?? false);

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
} as const;

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile ?? {}, opts);
}

// Validate branch configuration early in the process
export async function validateBranchConfiguration(
  opts: Pick<SyncOptions, "skipBranchValidation" | "yes">,
  branchOverride?: string
): Promise<void> {
  // When branch override is provided, skip validation - user is explicitly specifying the branch
  if (opts.skipBranchValidation || branchOverride || !isGitRepository()) {
    return;
  }

  const config = await readConfigFile();
  const { gitBranches } = config;

  const rawBranch = getCurrentGitBranch();

  const originalBranchIfForked = getOriginalBranchForWorkspaceForks(rawBranch);

  let currentBranch: string | null;
  if (originalBranchIfForked) {
    log.info(
      `Workspace fork detected from branch name \`${rawBranch}\`. Validating branch configuration using original branch \`${originalBranchIfForked}\``
    );
    currentBranch = originalBranchIfForked;
  } else {
    currentBranch = rawBranch;
  }

  // In a git repository, gitBranches section is recommended
  if (!gitBranches || Object.keys(gitBranches).length === 0) {
    log.warn(
      "⚠️  WARNING: In a Git repository, the 'gitBranches' section is recommended in wmill.yaml.\n" +
        "   Consider adding a gitBranches section with configuration for your Git branches.\n" +
        "   Run 'wmill init' to recreate the configuration file with proper branch setup."
    );
    return;
  }

  // Current branch must be defined in gitBranches config
  if (currentBranch && !gitBranches[currentBranch]) {
    // In interactive mode, offer to create the branch
    if (Deno.stdin.isTerminal()) {
      const availableBranches = Object.keys(gitBranches).join(", ");
      log.info(
        `Current Git branch '${currentBranch}' is not defined in the gitBranches configuration.\n` +
          `Available branches: ${availableBranches}`
      );

      const shouldCreate =
        opts.yes ||
        (await Confirm.prompt({
          message: `Create empty branch configuration for '${currentBranch}'?`,
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

        // Read current config, add branch, and write it back
        const currentConfig = await readConfigFile();

        if (!currentConfig.gitBranches) {
          currentConfig.gitBranches = {};
        }
        currentConfig.gitBranches[currentBranch] = { overrides: {} };

        await Deno.writeTextFile("wmill.yaml", yamlStringify(currentConfig));

        log.info(
          `✅ Created empty branch configuration for '${currentBranch}'`
        );
      } else {
        log.warn(
          "⚠️  WARNING: Branch creation cancelled. You can manually add the branch to wmill.yaml or use 'wmill gitsync-settings pull' to pull configuration from an existing windmill workspace git-sync configuration."
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
        `⚠️  WARNING: Current Git branch '${currentBranch}' is not defined in the gitBranches configuration.\n` +
          `   Consider adding configuration for branch '${currentBranch}' in the gitBranches section of wmill.yaml.\n` +
          `   Available branches: ${Object.keys(gitBranches).join(", ")}`
      );
      return;
    }
  }
}

// Get effective settings by merging top-level settings with branch-specific overrides
export async function getEffectiveSettings(
  config: SyncOptions,
  promotion?: string,
  skipBranchValidation?: boolean,
  suppressLogs?: boolean,
  branchOverride?: string
): Promise<SyncOptions> {
  // Start with top-level settings from config
  const { gitBranches, ...topLevelSettings } = config;
  const effective = { ...topLevelSettings };

  // Determine the branch to use: branchOverride takes precedence, then git detection
  let currentBranch: string | null = null;
  let originalBranchIfForked: string | null = null;
  let rawGitBranch: string | null = null;

  if (branchOverride) {
    currentBranch = branchOverride;
    // Note: "Using branch override" is logged in context.ts when resolving workspace
  } else if (isGitRepository()) {
    rawGitBranch = getCurrentGitBranch();
    originalBranchIfForked = getOriginalBranchForWorkspaceForks(rawGitBranch);

    if (originalBranchIfForked) {
      log.info(
        `Using overrides from original branch \`${originalBranchIfForked}\``
      );
      currentBranch = originalBranchIfForked;
    } else {
      currentBranch = rawGitBranch;
    }
  } else {
    log.debug("Not in a Git repository and no branch override provided, using top-level settings");
  }

  // If promotion is specified, use that branch's promotionOverrides or overrides
  if (promotion && gitBranches && gitBranches[promotion]) {
    const targetBranch = gitBranches[promotion];

    // First try promotionOverrides, then fall back to overrides
    if (targetBranch.promotionOverrides) {
      Object.assign(effective, targetBranch.promotionOverrides);
      if (!suppressLogs) {
        log.info(`Applied promotion settings from branch: ${promotion}`);
      }
    } else if (targetBranch.overrides) {
      Object.assign(effective, targetBranch.overrides);
      if (!suppressLogs) {
        log.info(
          `Applied settings from branch: ${promotion} (no promotionOverrides found)`
        );
      }
    } else {
      log.debug(
        `No promotion or regular overrides found for branch '${promotion}', using top-level settings`
      );
    }
  }
  // Otherwise use current branch overrides (existing behavior)
  else if (
    currentBranch &&
    gitBranches &&
    gitBranches[currentBranch] &&
    gitBranches[currentBranch].overrides
  ) {
    Object.assign(effective, gitBranches[currentBranch].overrides);
    if (!suppressLogs) {
      const extraLog = originalBranchIfForked
        ? ` (because it is the origin of the workspace fork branch \`${rawGitBranch}\`)`
        : "";
      log.info(
        `Applied settings for Git branch: ${currentBranch}${extraLog}`
      );
    }
  } else if (currentBranch) {
    log.debug(
      `No branch-specific overrides found for '${currentBranch}', using top-level settings`
    );
  }

  return effective;
}
