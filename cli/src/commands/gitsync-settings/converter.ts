import { SyncOptions } from "../../core/conf.ts";
import { BackendGitSyncSettings, GIT_SYNC_FIELDS, INCLUDE_TYPE_MAPPINGS } from "./types.ts";

// Helper to compare string arrays (used for includes/excludes/extraIncludes)
function arraysEqual(arr1: string[], arr2: string[]): boolean {
  if (arr1.length !== arr2.length) {
    return false;
  }
  const sorted1 = [...arr1].sort();
  const sorted2 = [...arr2].sort();
  return sorted1.every((item, index) => item === sorted2[index]);
}

export class GitSyncSettingsConverter {
  // Convert backend include_type array to SyncOptions boolean flags
  static fromBackendFormat(settings: BackendGitSyncSettings): SyncOptions {
    const includeTypes = settings.include_type || [];

    const result: SyncOptions = {
      includes: settings.include_path || [],
      excludes: settings.exclude_path || [],
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

    // Only include extraIncludes if it has content
    if (settings.extra_include_path && settings.extra_include_path.length > 0) {
      result.extraIncludes = settings.extra_include_path;
    }

    return result;
  }

  // Convert SyncOptions boolean flags to backend include_type array
  static toBackendFormat(opts: SyncOptions): BackendGitSyncSettings {
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

    const result: BackendGitSyncSettings = {
      include_path: opts.includes || [],
      include_type: includeTypes,
    };

    // Only include optional arrays if they have content
    if (opts.excludes && opts.excludes.length > 0) {
      result.exclude_path = opts.excludes;
    }
    if (opts.extraIncludes && opts.extraIncludes.length > 0) {
      result.extra_include_path = opts.extraIncludes;
    }

    return result;
  }

  // Normalize SyncOptions for semantic comparison - treat undefined values as their defaults
  static normalize(opts: SyncOptions): SyncOptions {
    return {
      ...opts,
      includes: opts.includes || [],
      excludes: opts.excludes || [],
      extraIncludes: opts.extraIncludes || [],
      skipVariables: opts.skipVariables ?? false,
      skipResources: opts.skipResources ?? false,
      skipResourceTypes: opts.skipResourceTypes ?? false,
      skipSecrets: opts.skipSecrets ?? true,
      skipScripts: opts.skipScripts ?? false,
      skipFlows: opts.skipFlows ?? false,
      skipApps: opts.skipApps ?? false,
      skipFolders: opts.skipFolders ?? false,
      includeSchedules: opts.includeSchedules ?? false,
      includeTriggers: opts.includeTriggers ?? false,
      includeUsers: opts.includeUsers ?? false,
      includeGroups: opts.includeGroups ?? false,
      includeSettings: opts.includeSettings ?? false,
      includeKey: opts.includeKey ?? false,
    };
  }

  // Extract only git-sync relevant fields for comparison
  static extractGitSyncFields(opts: SyncOptions): Partial<SyncOptions> {
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

  // Check if two values are different, with special handling for arrays
  static isDifferent(value1: any, value2: any): boolean {
    if (Array.isArray(value1) && Array.isArray(value2)) {
      return !arraysEqual(value1 as string[], value2 as string[]);
    }
    return value1 !== value2;
  }
}
