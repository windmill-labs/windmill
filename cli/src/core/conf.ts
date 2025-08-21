import { log, yamlParseFile, Confirm, yamlStringify } from "../../deps.ts";
import { getCurrentGitBranch, isGitRepository } from "../utils/git.ts";

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

export async function readConfigFile(): Promise<SyncOptions> {
  try {
    const conf = (await yamlParseFile("wmill.yaml")) as SyncOptions;

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
          await Deno.writeTextFile("wmill.yaml", yamlStringify(conf));
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

    // Check if file exists to distinguish between file not found and parsing errors
    try {
      await Deno.stat("wmill.yaml");
      // File exists but couldn't be parsed - this is likely a YAML syntax error
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
    } catch (statError) {
      // Check if it's actually a YAML syntax error vs file not found
      if (e instanceof Error && e.message.includes("Error parsing yaml")) {
        const causeMessage = e.cause instanceof Error ? e.cause.message : String(e.cause);
        // If the cause mentions file not found or permission issues, treat as file access error
        if (causeMessage.includes("No such file or directory") ||
            causeMessage.includes("ENOENT") ||
            causeMessage.includes("Permission denied") ||
            causeMessage.includes("EACCES")) {
          if (causeMessage.includes("Permission denied") || causeMessage.includes("EACCES")) {
            throw new Error(
              "❌ Cannot read wmill.yaml: Permission denied\n" +
              "   Please check file permissions or run with appropriate access rights."
            );
          } else {
            log.warn(
              "No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime."
            );
            return {};
          }
        } else {
          // Real YAML syntax error
          throw new Error(
            "❌ YAML syntax error in wmill.yaml:\n" +
            "   " + causeMessage + "\n" +
            "   Please fix the YAML syntax in wmill.yaml or delete the file to start fresh."
          );
        }
      }

      // File doesn't exist - this is the original behavior
      log.warn(
        "No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime."
      );
      return {};
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
