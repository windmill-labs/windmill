import { log, yamlParseFile, Confirm, yamlStringify } from "../../deps.ts";
import { getCurrentGitBranch, isGitRepository } from "../utils/git.ts";
import { join, dirname, resolve } from "node:path";
import { existsSync } from "node:fs";
import { execSync } from "node:child_process";

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
  message?: string;
  includes?: string[];
  extraIncludes?: string[];
  excludes?: string[];
  defaultTs?: "bun" | "deno";
  codebases?: Codebase[];
  parallel?: number;
  jsonOutput?: boolean;
  git_branches?: {
    commonSpecificItems?: {
      variables?: string[];
      resources?: string[];
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
      };
    };
  };
  promotion?: string;
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
}

function getGitRepoRoot(): string | null {
  try {
    const result = execSync("git rev-parse --show-toplevel", {
      encoding: "utf8",
      stdio: "pipe"
    });
    return result.trim();
  } catch (error) {
    return null;
  }
}

function findWmillYaml(): string | null {
  const startDir = resolve(Deno.cwd());
  const isInGitRepo = isGitRepository();

  // If not in git repo, only check current directory
  if (!isInGitRepo) {
    const wmillYamlPath = join(startDir, "wmill.yaml");
    return existsSync(wmillYamlPath) ? wmillYamlPath : null;
  }

  // If in git repo, search up to git repository root
  const gitRoot = getGitRepoRoot();
  let currentDir = startDir;
  let foundPath: string | null = null;

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
  if (foundPath && resolve(dirname(foundPath)) !== resolve(startDir)) {
    const configDir = dirname(foundPath);
    const relativePath = foundPath.replace(startDir, ".");
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

    // Handle obsolete overrides format
    if (conf && 'overrides' in conf) {
      const overrides = conf.overrides as any;
      const hasSettings = overrides && typeof overrides === 'object' && Object.keys(overrides).length > 0;

      if (hasSettings) {
        throw new Error(
          "❌ The 'overrides' field is no longer supported.\n" +
          "   The configuration system now uses Git branch-based configuration only.\n" +
          "   Please delete your wmill.yaml and run 'wmill init' to recreate it with the new format."
        );
      } else {
        // Remove empty overrides with a note
        log.info("ℹ️  Removing empty 'overrides: {}' from wmill.yaml (migrated to git_branches format)");
        delete conf.overrides;
        // Write the updated config back to file
        try {
          await Deno.writeTextFile(wmillYamlPath, yamlStringify(conf));
        } catch (error) {
          log.warn(`Could not update wmill.yaml to remove empty overrides: ${error instanceof Error ? error.message : error}`);
        }
      }
    }

    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    if (e instanceof Error && (e.message.includes("overrides") || e.message.includes("Obsolete configuration format"))) {
      throw e; // Re-throw the specific obsolete format error
    }

    // Since we already found the file path, this is likely a parsing or access error
    if (e instanceof Error && e.message.includes("Error parsing yaml")) {
      const yamlError = e.cause instanceof Error ? e.cause.message : String(e.cause);
      throw new Error(
        "❌ YAML syntax error in wmill.yaml:\n" +
        "   " + yamlError + "\n" +
        "   Please fix the YAML syntax in wmill.yaml or delete the file to start fresh."
      );
    } else {
      // File exists but has other issues (permissions, etc.)
      throw new Error(
        "❌ Failed to read wmill.yaml:\n" +
        "   " + (e instanceof Error ? e.message : String(e)) + "\n" +
        "   Please check file permissions or fix the syntax."
      );
    }
  }
}

// Default sync options - shared across the codebase to prevent duplication
export const DEFAULT_SYNC_OPTIONS: Readonly<Required<Pick<SyncOptions,
  'defaultTs' | 'includes' | 'excludes' | 'codebases' | 'skipVariables' | 'skipResources' |
  'skipResourceTypes' | 'skipSecrets' | 'includeSchedules' | 'includeTriggers' |
  'skipScripts' | 'skipFlows' | 'skipApps' | 'skipFolders' |
  'includeUsers' | 'includeGroups' | 'includeSettings' | 'includeKey'
>>> = {
  defaultTs: 'bun',
  includes: ['f/**'],
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
  includeKey: false
} as const;

export async function mergeConfigWithConfigFile<T>(
  opts: T
): Promise<T & SyncOptions> {
  const configFile = await readConfigFile();
  return Object.assign(configFile ?? {}, opts);
}

// Validate branch configuration early in the process
export async function validateBranchConfiguration(skipValidation?: boolean, autoAccept?: boolean): Promise<void> {
  if (skipValidation || !isGitRepository()) {
    return;
  }

  const config = await readConfigFile();
  const { git_branches } = config;
  const currentBranch = getCurrentGitBranch();

  // In a git repository, git_branches section is recommended
  if (!git_branches || Object.keys(git_branches).length === 0) {
    log.warn(
      "⚠️  WARNING: In a Git repository, the 'git_branches' section is recommended in wmill.yaml.\n" +
      "   Consider adding a git_branches section with configuration for your Git branches.\n" +
      "   Run 'wmill init' to recreate the configuration file with proper branch setup."
    );
    return;
  }

  // Current branch must be defined in git_branches config
  if (currentBranch && !git_branches[currentBranch]) {
    // In interactive mode, offer to create the branch
    if (Deno.stdin.isTerminal()) {
      const availableBranches = Object.keys(git_branches).join(', ');
      log.info(
        `Current Git branch '${currentBranch}' is not defined in the git_branches configuration.\n` +
        `Available branches: ${availableBranches}`
      );

      const shouldCreate = autoAccept || await Confirm.prompt({
        message: `Create empty branch configuration for '${currentBranch}'?`,
        default: true,
      });

      if (shouldCreate) {
        // Read current config, add branch, and write it back
        const currentConfig = await readConfigFile();

        if (!currentConfig.git_branches) {
          currentConfig.git_branches = {};
        }
        currentConfig.git_branches[currentBranch] = { overrides: {} };

        await Deno.writeTextFile("wmill.yaml", yamlStringify(currentConfig));

        log.info(`✅ Created empty branch configuration for '${currentBranch}'`);
      } else {
        log.warn("⚠️  WARNING: Branch creation cancelled. You can manually add the branch to wmill.yaml or use 'wmill gitsync-settings pull' to pull configuration from an existing windmill workspace git-sync configuration.");
        return;
      }
    } else {
      log.warn(
        `⚠️  WARNING: Current Git branch '${currentBranch}' is not defined in the git_branches configuration.\n` +
        `   Consider adding configuration for branch '${currentBranch}' in the git_branches section of wmill.yaml.\n` +
        `   Available branches: ${Object.keys(git_branches).join(', ')}`
      );
      return;
    }
  }
}

// Get effective settings by merging top-level settings with branch-specific overrides
export async function getEffectiveSettings(config: SyncOptions, promotion?: string, skipBranchValidation?: boolean, suppressLogs?: boolean): Promise<SyncOptions> {
  // Start with top-level settings from config
  const { git_branches, ...topLevelSettings } = config;
  let effective = { ...topLevelSettings };

  if (isGitRepository()) {
    const currentBranch = getCurrentGitBranch();

    // If promotion is specified, use that branch's promotionOverrides or overrides
    if (promotion && git_branches && git_branches[promotion]) {
      const targetBranch = git_branches[promotion];

      // First try promotionOverrides, then fall back to overrides
      if (targetBranch.promotionOverrides) {
        Object.assign(effective, targetBranch.promotionOverrides);
        if (!suppressLogs) {
          log.info(`Applied promotion settings from branch: ${promotion}`);
        }
      } else if (targetBranch.overrides) {
        Object.assign(effective, targetBranch.overrides);
        if (!suppressLogs) {
          log.info(`Applied settings from branch: ${promotion} (no promotionOverrides found)`);
        }
      } else {
        log.debug(`No promotion or regular overrides found for branch '${promotion}', using top-level settings`);
      }
    }
    // Otherwise use current branch overrides (existing behavior)
    else if (currentBranch && git_branches && git_branches[currentBranch] && git_branches[currentBranch].overrides) {
      Object.assign(effective, git_branches[currentBranch].overrides);
      if (!suppressLogs) {
        log.info(`Applied settings for Git branch: ${currentBranch}`);
      }
    } else if (currentBranch) {
      log.debug(`No branch-specific overrides found for '${currentBranch}', using top-level settings`);
    }
  } else {
    log.debug("Not in a Git repository, using top-level settings");
  }

  return effective;
}
