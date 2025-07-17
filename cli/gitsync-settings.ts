import { colors, Command, log, yamlStringify } from "./deps.ts";
import { GlobalOptions } from "./types.ts";
import { requireLogin } from "./auth.ts";
import { resolveWorkspace } from "./context.ts";
import * as wmill from "./gen/services.gen.ts";
import {
  DEFAULT_SYNC_OPTIONS,
  getEffectiveSettings,
  readConfigFile,
  SyncOptions,
} from "./conf.ts";
import { deepEqual, Repository, selectRepository } from "./utils.ts";

// Constants for git-sync fields to avoid duplication
const GIT_SYNC_FIELDS = [
  "includes",
  "excludes",
  "extraIncludes",
  "skipScripts",
  "skipFlows",
  "skipApps",
  "skipFolders",
  "skipVariables",
  "skipResources",
  "skipResourceTypes",
  "skipSecrets",
  "includeSchedules",
  "includeTriggers",
  "includeUsers",
  "includeGroups",
  "includeSettings",
  "includeKey",
] as const;

// Helper to normalize repository path by removing $res: prefix
function normalizeRepoPath(path: string): string {
  return path.replace(/^\$res:/, "");
}

// Helper to get typed field value from SyncOptions
function getFieldValue(opts: SyncOptions, field: string): any {
  return (opts as any)[field];
}

// Construct override key using the single format: baseUrl:workspaceId:repo
function constructOverrideKey(
  baseUrl: string,
  workspaceId: string,
  repoPath: string,
  workspaceLevel = false,
): string {
  // Validate that components don't contain colons to avoid key collisions
  if (baseUrl.includes(':') && !baseUrl.startsWith('http')) {
    throw new Error(`Invalid baseUrl contains colon: ${baseUrl}`);
  }
  if (workspaceId.includes(':')) {
    throw new Error(`Invalid workspaceId contains colon: ${workspaceId}`);
  }
  if (repoPath.includes(':') && !repoPath.startsWith('$res:')) {
    throw new Error(`Invalid repoPath contains colon: ${repoPath}`);
  }

  if (workspaceLevel) {
    return `${baseUrl}:${workspaceId}:*`;
  }
  return `${baseUrl}:${workspaceId}:${repoPath}`;
}

// Helper to compare string arrays (used for includes/excludes/extraIncludes)
function arraysEqual(arr1: string[], arr2: string[]): boolean {
  if (arr1.length !== arr2.length) {
    return false;
  }
  const sorted1 = [...arr1].sort();
  const sorted2 = [...arr2].sort();
  return sorted1.every((item, index) => item === sorted2[index]);
}

// Normalize SyncOptions for semantic comparison - treat undefined arrays as empty arrays
function normalizeSyncOptions(opts: SyncOptions): SyncOptions {
  return {
    ...opts,
    includes: opts.includes || [],
    excludes: opts.excludes || [],
    extraIncludes: opts.extraIncludes || [],
  };
}

// Extract only git-sync relevant fields for comparison
function extractGitSyncFields(opts: SyncOptions): Partial<SyncOptions> {
  return {
    includes: opts.includes || [],
    excludes: opts.excludes || [],
    extraIncludes: opts.extraIncludes || [],
    skipScripts: opts.skipScripts,
    skipFlows: opts.skipFlows,
    skipApps: opts.skipApps,
    skipFolders: opts.skipFolders,
    skipVariables: opts.skipVariables,
    skipResources: opts.skipResources,
    skipResourceTypes: opts.skipResourceTypes,
    skipSecrets: opts.skipSecrets,
    includeSchedules: opts.includeSchedules,
    includeTriggers: opts.includeTriggers,
    includeUsers: opts.includeUsers,
    includeGroups: opts.includeGroups,
    includeSettings: opts.includeSettings,
    includeKey: opts.includeKey,
  };
}

// Helper function to determine current settings based on write mode and conflicts
function getCurrentSettings(
  localConfig: SyncOptions,
  writeMode: "override" | "replace",
  overrideKey?: string,
): SyncOptions {
  if (
    writeMode === "override" &&
    overrideKey &&
    localConfig.overrides?.[overrideKey]
  ) {
    return {
      ...DEFAULT_SYNC_OPTIONS,
      ...localConfig,
      ...localConfig.overrides[overrideKey],
    };
  } else {
    // For "replace" mode, exclude overrides since they're never accessed
    const { overrides, ...configWithoutOverrides } = localConfig;
    return { ...DEFAULT_SYNC_OPTIONS, ...configWithoutOverrides };
  }
}

// Interface for git-sync repository settings from backend
interface GitSyncRepository extends Repository {
  settings: {
    include_path: string[];
    include_type: string[];
    exclude_path?: string[];
    extra_include_path?: string[];
  };
}

// Convert backend include_type array to SyncOptions boolean flags
function includeTypeToSyncOptions(
  includeTypes: string[],
): Partial<SyncOptions> {
  return {
    skipScripts: !includeTypes.includes("script"),
    skipFlows: !includeTypes.includes("flow"),
    skipApps: !includeTypes.includes("app"),
    skipFolders: !includeTypes.includes("folder"),
    skipVariables: !includeTypes.includes("variable"),
    skipResources: !includeTypes.includes("resource"),
    skipResourceTypes: !includeTypes.includes("resourcetype"),
    skipSecrets: !includeTypes.includes("secret"),
    includeSchedules: includeTypes.includes("schedule"),
    includeTriggers: includeTypes.includes("trigger"),
    includeUsers: includeTypes.includes("user"),
    includeGroups: includeTypes.includes("group"),
    includeSettings: includeTypes.includes("settings"),
    includeKey: includeTypes.includes("key"),
  };
}

// Convert SyncOptions boolean flags to backend include_type array
function syncOptionsToIncludeType(opts: SyncOptions): string[] {
  const includeTypes: string[] = [];

  if (!opts.skipScripts) includeTypes.push("script");
  if (!opts.skipFlows) includeTypes.push("flow");
  if (!opts.skipApps) includeTypes.push("app");
  if (!opts.skipFolders) includeTypes.push("folder");
  if (!opts.skipVariables) includeTypes.push("variable");
  if (!opts.skipResources) includeTypes.push("resource");
  if (!opts.skipResourceTypes) includeTypes.push("resourcetype");
  if (!opts.skipSecrets) includeTypes.push("secret");
  if (opts.includeSchedules) includeTypes.push("schedule");
  if (opts.includeTriggers) includeTypes.push("trigger");
  if (opts.includeUsers) includeTypes.push("user");
  if (opts.includeGroups) includeTypes.push("group");
  if (opts.includeSettings) includeTypes.push("settings");
  if (opts.includeKey) includeTypes.push("key");

  return includeTypes;
}

// Convert SyncOptions to backend format used by both Windmill backend and UI
function syncOptionsToBackendFormat(opts: SyncOptions): {
  include_path: string[];
  exclude_path: string[];
  extra_include_path: string[];
  include_type: string[];
} {
  return {
    include_path: opts.includes || [],
    exclude_path: opts.excludes || [],
    extra_include_path: opts.extraIncludes || [],
    include_type: syncOptionsToIncludeType(opts),
  };
}

// Select repository interactively if multiple exist
// Generate structured diff showing field changes
function generateStructuredDiff(
  current: any,
  backend: any,
): { [key: string]: { from: any; to: any } } {
  const diff: { [key: string]: { from: any; to: any } } = {};

  // Get all unique keys from both objects
  const allKeys = new Set([...Object.keys(current), ...Object.keys(backend)]);

  for (const key of allKeys) {
    const currentValue = current[key];
    const backendValue = backend[key];

    if (!deepEqual(currentValue, backendValue)) {
      diff[key] = {
        from: currentValue,
        to: backendValue,
      };
    }
  }

  return diff;
}

// Helper to generate changes between two normalized SyncOptions objects
function generateChanges(
  normalizedCurrent: SyncOptions,
  normalizedNew: SyncOptions,
): { [key: string]: { from: any; to: any } } {
  const changes: { [key: string]: { from: any; to: any } } = {};

  for (const field of GIT_SYNC_FIELDS) {
    const currentValue = getFieldValue(normalizedCurrent, field);
    const newValue = getFieldValue(normalizedNew, field);
    if (!deepEqual(currentValue, newValue)) {
      changes[field] = {
        from: currentValue,
        to: newValue,
      };
    }
  }

  return changes;
}

// Helper to display changes in human-readable format
function displayChanges(
  changes: { [key: string]: { from: any; to: any } },
): void {
  for (const [field, change] of Object.entries(changes)) {
    if (
      Array.isArray(change.from) ||
      Array.isArray(change.to)
    ) {
      console.log(colors.yellow(`  ${field}:`));
      console.log(
        colors.red(
          `    - ${JSON.stringify(change.from)}`,
        ),
      );
      console.log(
        colors.green(
          `    + ${JSON.stringify(change.to)}`,
        ),
      );
    } else {
      console.log(
        colors.yellow(`  ${field}: `) +
          colors.red(`${change.from}`) +
          " → " +
          colors.green(`${change.to}`),
      );
    }
  }
}

async function pullGitSyncSettings(
  opts: GlobalOptions & {
    repository?: string;
    workspaceLevel?: boolean;
    default?: boolean;
    diff?: boolean;
    jsonOutput?: boolean;
    replace?: boolean;
    override?: boolean;
    withBackendSettings?: string;
  },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  try {
    // Parse and validate --with-backend-settings if provided
    let settings: any;
    if (opts.withBackendSettings) {
      try {
        const parsedSettings = JSON.parse(opts.withBackendSettings);

        // Validate the structure matches expected test format (raw settings object)
        if (
          !parsedSettings.include_path ||
          !Array.isArray(parsedSettings.include_path)
        ) {
          throw new Error(
            "Invalid settings format. Expected include_path array",
          );
        }
        if (
          !parsedSettings.include_type ||
          !Array.isArray(parsedSettings.include_type)
        ) {
          throw new Error(
            "Invalid settings format. Expected include_type array",
          );
        }

        // Create mock backend response with single repository using provided settings
        const mockRepositoryPath = opts.repository || "u/mock/repo";
        settings = {
          git_sync: {
            repositories: [{
              git_repo_resource_path: mockRepositoryPath,
              settings: {
                include_path: parsedSettings.include_path,
                include_type: parsedSettings.include_type,
                exclude_path: parsedSettings.exclude_path || [],
                extra_include_path: parsedSettings.extra_include_path || [],
              },
            }],
          },
        };
      } catch (parseError) {
        const errorMessage = parseError instanceof Error ? parseError.message : String(parseError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error:
                `Failed to parse --with-backend-settings JSON: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to parse --with-backend-settings JSON: ${errorMessage}`,
            ),
          );
        }
        return;
      }
    } else {
      // Fetch workspace settings to get git-sync configuration
      try {
        settings = await wmill.getSettings({
          workspace: workspace.workspaceId,
        });
      } catch (apiError) {
        const errorMessage = apiError instanceof Error ? apiError.message : String(apiError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error: `Failed to fetch workspace settings: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to fetch workspace settings: ${errorMessage}`,
            ),
          );
        }
        return;
      }
    }

    if (
      !settings.git_sync?.repositories ||
      settings.git_sync.repositories.length === 0
    ) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: false,
            error: "No git-sync repositories configured",
          }),
        );
      } else {
        log.error(
          colors.red(
            "No git-sync repositories configured in workspace",
          ),
        );
      }
      return;
    }

    // Find the repository to work with
    let selectedRepo: GitSyncRepository;
    if (opts.repository) {
      const found = settings.git_sync.repositories.find(
        (r: GitSyncRepository) =>
          r.git_repo_resource_path === opts.repository ||
          r.git_repo_resource_path === `$res:${opts.repository}`,
      );
      if (!found) {
        throw new Error(`Repository ${opts.repository} not found`);
      }
      selectedRepo = found;
    } else {
      selectedRepo = await selectRepository(
        settings.git_sync.repositories,
      );
    }

    // Convert backend settings to SyncOptions format
    const backendSyncOptions: SyncOptions = {
      includes: selectedRepo.settings.include_path || [],
      excludes: selectedRepo.settings.exclude_path || [],
      extraIncludes: selectedRepo.settings.extra_include_path || [],
      ...includeTypeToSyncOptions(
        selectedRepo.settings.include_type || [],
      ),
    };

    // Check if wmill.yaml exists - require it for git-sync settings commands
    try {
      await Deno.stat("wmill.yaml");
    } catch (error) {
      log.error(
        colors.red(
          "No wmill.yaml file found. Please run 'wmill init' first to create the configuration file.",
        ),
      );
      Deno.exit(1);
    }

    // Read current local configuration
    const localConfig = await readConfigFile();

    // Determine where to write the settings for diff display
    let overrideKey: string | undefined;
    let writeMode: "override" | "replace" = "replace";

    // For diff mode, determine what the write mode would be without interactive prompts
    if (opts.default) {
      writeMode = "replace";
    } else if (opts.replace) {
      writeMode = "replace";
    } else if (opts.override || opts.workspaceLevel) {
      writeMode = "override";
      if (opts.workspaceLevel) {
        overrideKey = constructOverrideKey(
          workspace.remote,
          workspace.workspaceId,
          "",
          true,
        );
      } else {
        const repoPath = normalizeRepoPath(selectedRepo.git_repo_resource_path);
        overrideKey = constructOverrideKey(
          workspace.remote,
          workspace.workspaceId,
          repoPath,
        );
      }
    } else {
      // Default behavior for existing files with no explicit flags
      // Use same logic as diff to determine if there's a real conflict
      const currentSettings = getCurrentSettings(
        localConfig,
        "replace", // Check against replace mode
        undefined,
      );

      const gitSyncBackend = extractGitSyncFields(
        normalizeSyncOptions(backendSyncOptions),
      );
      const gitSyncCurrent = extractGitSyncFields(
        normalizeSyncOptions(currentSettings),
      );
      const hasConflict = !deepEqual(gitSyncBackend, gitSyncCurrent);

      if (hasConflict) {
        // For diff mode, show what override would look like
        writeMode = "override";
        const repoPath = normalizeRepoPath(selectedRepo.git_repo_resource_path);
        overrideKey = constructOverrideKey(
          workspace.remote,
          workspace.workspaceId,
          repoPath,
        );
      } else {
        writeMode = "replace";
      }
    }

    if (opts.diff) {
      // Show differences between local and backend
      const currentSettings = getCurrentSettings(
        localConfig,
        writeMode,
        overrideKey,
      );

      const normalizedCurrent = normalizeSyncOptions(currentSettings);
      const normalizedBackend = normalizeSyncOptions(backendSyncOptions);
      const gitSyncCurrent = extractGitSyncFields(normalizedCurrent);
      const gitSyncBackend = extractGitSyncFields(normalizedBackend);
      const hasChanges = !deepEqual(gitSyncBackend, gitSyncCurrent);

      if (opts.jsonOutput) {
        const repoPath = normalizeRepoPath(selectedRepo.git_repo_resource_path);

        // Generate structured diff using the same normalized objects
        const structuredDiff = hasChanges
          ? generateStructuredDiff(gitSyncCurrent, gitSyncBackend)
          : {};

        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: syncOptionsToBackendFormat(normalizedCurrent),
            backend: syncOptionsToBackendFormat(normalizedBackend),
            repository: selectedRepo.git_repo_resource_path,
            diff: structuredDiff,
          }),
        );
      } else {
        if (hasChanges) {
          log.info("Changes that would be made:");
          const changes = generateChanges(normalizedCurrent, normalizedBackend);

          if (Object.keys(changes).length === 0) {
            log.info(colors.green("No differences found"));
          } else {
            displayChanges(changes);
          }
        } else {
          log.info(colors.green("No differences found"));
        }
      }
      return;
    }

    // For non-diff mode, handle interactive logic if needed
    // Only show interactive prompts for existing files with conflicts
    if (
      !opts.diff &&
      !opts.default &&
      !opts.replace &&
      !opts.override &&
      !opts.workspaceLevel
    ) {
      // Use the same logic as diff to determine current settings
      const currentSettings = getCurrentSettings(
        localConfig,
        writeMode,
        overrideKey,
      );

      const gitSyncBackend = extractGitSyncFields(
        normalizeSyncOptions(backendSyncOptions),
      );
      const gitSyncCurrent = extractGitSyncFields(
        normalizeSyncOptions(currentSettings),
      );
      const hasConflict = !deepEqual(gitSyncBackend, gitSyncCurrent);

      if (hasConflict && Deno.stdin.isTerminal()) {
        // Interactive mode - ask user
        const { Select } = await import("./deps.ts");
        const choice = await Select.prompt({
          message: "Settings conflict detected. How would you like to proceed?",
          options: [
            {
              name: "Replace existing settings",
              value: "replace",
            },
            {
              name: "Add repository-specific override",
              value: "override",
            },
            { name: "Cancel", value: "cancel" },
          ],
        });

        if (choice === "cancel") {
          log.info("Operation cancelled");
          return;
        }

        writeMode = choice as "replace" | "override";
        if (writeMode === "override") {
          const repoPath = normalizeRepoPath(
            selectedRepo.git_repo_resource_path,
          );
          overrideKey = constructOverrideKey(
            workspace.remote,
            workspace.workspaceId,
            repoPath,
          );
        }
      } else if (hasConflict) {
        // Non-interactive mode with conflicts - show message and exit
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error:
                "Settings conflict detected. Use --replace or --override flags to resolve.",
              hasConflict: true,
            }),
          );
        } else {
          log.error(colors.red("Settings conflict detected."));
          log.info(
            "Use --replace to overwrite existing settings or --override to add repository-specific override.",
          );
        }
        return;
      }
    }

    // Check if there are actually any changes before writing
    const currentSettingsForCheck = getCurrentSettings(
      localConfig,
      writeMode,
      overrideKey,
    );
    const gitSyncBackend = extractGitSyncFields(
      normalizeSyncOptions(backendSyncOptions),
    );
    const gitSyncCurrent = extractGitSyncFields(
      normalizeSyncOptions(currentSettingsForCheck),
    );
    const hasActualChanges = !deepEqual(gitSyncBackend, gitSyncCurrent);

    if (!hasActualChanges) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            message: "No changes needed",
            repository: selectedRepo.git_repo_resource_path,
          }),
        );
      } else {
        log.info(
          colors.green(
            "No changes needed - settings are already up to date",
          ),
        );
      }
      return;
    }

    // Apply the settings based on write mode
    let updatedConfig: SyncOptions;

    if (writeMode === "replace") {
      // Preserve existing local config and update only git-sync fields
      updatedConfig = { ...localConfig };
      // Remove overrides since we're in replace mode
      delete updatedConfig.overrides;
      // Update with backend git-sync settings
      Object.assign(updatedConfig, backendSyncOptions);
    } else if (writeMode === "override" && overrideKey) {
      // Add repository-specific override
      updatedConfig = { ...localConfig };
      if (!updatedConfig.overrides) {
        updatedConfig.overrides = {};
      }

      // Only store the delta - settings that differ from current effective settings
      const currentEffective = getCurrentSettings(localConfig, "replace");
      const deltaSettings: Partial<SyncOptions> = {};

      // Compare each setting and only include differences
      for (const [key, value] of Object.entries(backendSyncOptions)) {
        if (key === "overrides") continue; // Skip overrides field

        const currentValue = (currentEffective as any)[key];
        const newValue = value;

        // Compare arrays by content, primitives by value
        const isDifferent =
          Array.isArray(currentValue) && Array.isArray(newValue)
            ? !arraysEqual(currentValue, newValue)
            : currentValue !== newValue;

        if (isDifferent) {
          (deltaSettings as any)[key] = newValue;
        }
      }

      updatedConfig.overrides[overrideKey] = deltaSettings;
    } else {
      // Replace top-level settings
      updatedConfig = { ...localConfig };
      // Copy all backend settings to top level, excluding overrides
      const { overrides, ...topLevelSettings } = backendSyncOptions;
      Object.assign(updatedConfig, topLevelSettings);
    }

    // Write updated configuration
    await Deno.writeTextFile("wmill.yaml", yamlStringify(updatedConfig));

    if (opts.jsonOutput) {
      console.log(
        JSON.stringify({
          success: true,
          message: `Git-sync settings pulled successfully`,
          repository: selectedRepo.git_repo_resource_path,
          overrideKey,
        }),
      );
    } else {
      log.info(
        colors.green(
          `Git-sync settings pulled successfully from ${selectedRepo.git_repo_resource_path}`,
        ),
      );
      if (writeMode === "override" && overrideKey) {
        log.info(
          colors.gray(
            `Settings written to override key: ${overrideKey}`,
          ),
        );
      } else if (writeMode === "replace") {
        log.info(colors.gray(`Settings written as simple configuration`));
      } else {
        log.info(colors.gray(`Settings written to top-level defaults`));
      }
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (opts.jsonOutput) {
      console.log(
        JSON.stringify({ success: false, error: errorMessage }),
      );
    } else {
      log.error(
        colors.red(
          `Failed to pull git-sync settings: ${errorMessage}`,
        ),
      );
    }
  }
}

async function pushGitSyncSettings(
  opts: GlobalOptions & {
    repository?: string;
    diff?: boolean;
    jsonOutput?: boolean;
    withBackendSettings?: string;
  },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  try {
    // Check if wmill.yaml exists - require it for git-sync settings commands
    try {
      await Deno.stat("wmill.yaml");
    } catch (error) {
      log.error(
        colors.red(
          "No wmill.yaml file found. Please run 'wmill init' first to create the configuration file.",
        ),
      );
      Deno.exit(1);
    }

    // Read local configuration
    const localConfig = await readConfigFile();

    // Parse and validate --with-backend-settings if provided, otherwise fetch from backend
    let settings: any;
    if (opts.withBackendSettings) {
      try {
        const parsedSettings = JSON.parse(opts.withBackendSettings);

        // Validate the structure matches expected test format (raw settings object)
        if (
          !parsedSettings.include_path ||
          !Array.isArray(parsedSettings.include_path)
        ) {
          throw new Error(
            "Invalid settings format. Expected include_path array",
          );
        }
        if (
          !parsedSettings.include_type ||
          !Array.isArray(parsedSettings.include_type)
        ) {
          throw new Error(
            "Invalid settings format. Expected include_type array",
          );
        }

        // Create mock backend response with single repository using provided settings
        const mockRepositoryPath = opts.repository || "u/mock/repo";
        settings = {
          git_sync: {
            repositories: [{
              git_repo_resource_path: mockRepositoryPath,
              settings: {
                include_path: parsedSettings.include_path,
                include_type: parsedSettings.include_type,
                exclude_path: parsedSettings.exclude_path || [],
                extra_include_path: parsedSettings.extra_include_path || [],
              },
            }],
          },
        };
      } catch (parseError) {
        const errorMessage = parseError instanceof Error ? parseError.message : String(parseError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error:
                `Failed to parse --with-backend-settings JSON: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to parse --with-backend-settings JSON: ${errorMessage}`,
            ),
          );
        }
        return;
      }
    } else {
      // Fetch current backend settings
      try {
        settings = await wmill.getSettings({
          workspace: workspace.workspaceId,
        });
      } catch (apiError) {
        const errorMessage = apiError instanceof Error ? apiError.message : String(apiError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error: `Failed to fetch workspace settings: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to fetch workspace settings: ${errorMessage}`,
            ),
          );
        }
        return;
      }
    }

    if (
      !settings.git_sync?.repositories ||
      settings.git_sync.repositories.length === 0
    ) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: false,
            error: "No git-sync repositories configured",
          }),
        );
      } else {
        log.error(
          colors.red(
            "No git-sync repositories configured in workspace",
          ),
        );
      }
      return;
    }

    // Find the repository to work with
    let selectedRepo: GitSyncRepository;
    if (opts.repository) {
      const found = settings.git_sync.repositories.find(
        (r: GitSyncRepository) =>
          r.git_repo_resource_path === opts.repository ||
          r.git_repo_resource_path === `$res:${opts.repository}`,
      );
      if (!found) {
        throw new Error(`Repository ${opts.repository} not found`);
      }
      selectedRepo = found;
    } else {
      selectedRepo = await selectRepository(
        settings.git_sync.repositories,
      );
    }

    // Get effective settings for this workspace/repo
    const repoPath = normalizeRepoPath(selectedRepo.git_repo_resource_path);
    const effectiveSettings = getEffectiveSettings(
      localConfig,
      workspace.remote,
      workspace.workspaceId,
      repoPath,
    );

    // Convert to backend format
    const backendFormat = {
      include_path: effectiveSettings.includes || [],
      include_type: syncOptionsToIncludeType(effectiveSettings),
      exclude_path: effectiveSettings.excludes || [],
      extra_include_path: effectiveSettings.extraIncludes || [],
    };

    if (opts.diff) {
      // Show what would be pushed
      const currentBackend = selectedRepo.settings;

      // Convert current backend settings to SyncOptions for user-friendly display
      const currentSyncOptions: SyncOptions = {
        includes: currentBackend.include_path || [],
        excludes: currentBackend.exclude_path || [],
        extraIncludes: currentBackend.extra_include_path || [],
        ...includeTypeToSyncOptions(currentBackend.include_type || []),
      };

      const normalizedCurrent = normalizeSyncOptions(currentSyncOptions);
      const normalizedEffective = normalizeSyncOptions(effectiveSettings);
      const gitSyncCurrent = extractGitSyncFields(normalizedCurrent);
      const gitSyncEffective = extractGitSyncFields(normalizedEffective);
      const hasChanges = !deepEqual(gitSyncEffective, gitSyncCurrent);

      if (opts.jsonOutput) {
        // Generate structured diff using the same normalized objects
        const structuredDiff = hasChanges
          ? generateStructuredDiff(gitSyncCurrent, gitSyncEffective)
          : {};

        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: syncOptionsToBackendFormat(normalizedEffective),
            backend: syncOptionsToBackendFormat(normalizedCurrent),
            repository: selectedRepo.git_repo_resource_path,
            diff: structuredDiff,
          }),
        );
      } else {
        if (hasChanges) {
          log.info("Changes that would be pushed:");
          const changes = generateChanges(
            normalizedCurrent,
            normalizedEffective,
          );

          if (Object.keys(changes).length === 0) {
            log.info(colors.green("No changes to push"));
          } else {
            displayChanges(changes);
          }
        } else {
          log.info(colors.green("No changes to push"));
        }
      }
      return;
    }

    if (opts.withBackendSettings) {
      // Skip backend update when using simulated settings
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            message:
              `Git-sync settings push simulated (--with-backend-settings used)`,
            repository: selectedRepo.git_repo_resource_path,
            simulated: true,
          }),
        );
      } else {
        log.info(
          colors.green(
            `Git-sync settings push simulated for ${selectedRepo.git_repo_resource_path} (--with-backend-settings used)`,
          ),
        );
      }
    } else {
      // Update the specific repository settings
      const updatedRepos = settings.git_sync.repositories.map(
        (repo: GitSyncRepository) => {
          if (
            repo.git_repo_resource_path ===
              selectedRepo.git_repo_resource_path
          ) {
            return {
              ...repo,
              settings: backendFormat,
            };
          }
          return repo;
        },
      );

      // Push updated settings to backend
      try {
        await wmill.editWorkspaceGitSyncConfig({
          workspace: workspace.workspaceId,
          requestBody: {
            git_sync_settings: {
              repositories: updatedRepos,
            },
          },
        });
      } catch (apiError) {
        const errorMessage = apiError instanceof Error ? apiError.message : String(apiError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error: `Failed to update workspace git-sync config: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to update workspace git-sync config: ${errorMessage}`,
            ),
          );
        }
        return;
      }

      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            message: `Git-sync settings pushed successfully`,
            repository: selectedRepo.git_repo_resource_path,
          }),
        );
      } else {
        log.info(
          colors.green(
            `Git-sync settings pushed successfully to ${selectedRepo.git_repo_resource_path}`,
          ),
        );
      }
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (opts.jsonOutput) {
      console.log(
        JSON.stringify({ success: false, error: errorMessage }),
      );
    } else {
      log.error(
        colors.red(
          `Failed to push git-sync settings: ${errorMessage}`,
        ),
      );
    }
  }
}

const command = new Command()
  .description(
    "Manage git-sync settings between local wmill.yaml and Windmill backend",
  )
  .command("pull")
  .description(
    "Pull git-sync settings from Windmill backend to local wmill.yaml",
  )
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo)",
  )
  .option(
    "--workspace-level",
    "Write settings to workspace:* override instead of workspace:repo",
  )
  .option(
    "--default",
    "Write settings to top-level defaults instead of overrides",
  )
  .option("--replace", "Replace existing settings (non-interactive mode)")
  .option(
    "--override",
    "Add repository-specific override (non-interactive mode)",
  )
  .option("--diff", "Show differences without applying changes")
  .option("--json-output", "Output in JSON format")
  .option(
    "--with-backend-settings <json:string>",
    "Use provided JSON settings instead of querying backend (for testing)",
  )
  .action(pullGitSyncSettings as any)
  .command("push")
  .description(
    "Push git-sync settings from local wmill.yaml to Windmill backend",
  )
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo)",
  )
  .option("--diff", "Show what would be pushed without applying changes")
  .option("--json-output", "Output in JSON format")
  .option(
    "--with-backend-settings <json:string>",
    "Use provided JSON settings instead of querying backend (for testing)",
  )
  .action(pushGitSyncSettings as any);

export { pullGitSyncSettings, pushGitSyncSettings };
export default command;
