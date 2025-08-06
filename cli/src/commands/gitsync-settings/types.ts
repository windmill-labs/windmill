import { Repository } from "../../utils/utils.ts";

// Interface for git-sync repository settings from backend
export interface GitSyncRepository extends Repository {
  settings: {
    include_path: string[];
    include_type: string[];
    exclude_path?: string[];
    extra_include_path?: string[];
  };
}

// Backend format for git-sync settings
export interface BackendGitSyncSettings {
  include_path: string[];
  include_type: string[];
  exclude_path?: string[];
  extra_include_path?: string[];
}

// Constants for git-sync fields
export const GIT_SYNC_FIELDS = [
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

export type GitSyncField = typeof GIT_SYNC_FIELDS[number];

// Type mappings for backend include_type values
export const INCLUDE_TYPE_MAPPINGS = {
  script: "skipScripts",
  flow: "skipFlows",
  app: "skipApps",
  folder: "skipFolders",
  variable: "skipVariables",
  resource: "skipResources",
  resourcetype: "skipResourceTypes",
  secret: "skipSecrets",
  schedule: "includeSchedules",
  trigger: "includeTriggers",
  user: "includeUsers",
  group: "includeGroups",
  settings: "includeSettings",
  key: "includeKey",
} as const;

// Write mode for branch-based configuration
export type WriteMode = "override" | "replace";