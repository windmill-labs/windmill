import type { SyncOptions } from "./conf.ts";

// Interface for structured responses
export interface UIState {
  include_path: string[];
  include_type: string[];
}

export function uiStateToSyncOptions(uiState: UIState): SyncOptions {
  return {
    defaultTs: 'bun',
    includes: uiState.include_path.length > 0 ? uiState.include_path : ['f/**'],
    excludes: [],
    codebases: [],

    // Convert include_type array to skip/include flags
    skipVariables: !uiState.include_type.includes('variable'),
    skipResources: !uiState.include_type.includes('resource'),
    skipResourceTypes: !uiState.include_type.includes('resourcetype'),
    skipSecrets: !uiState.include_type.includes('secret'),

    includeSchedules: uiState.include_type.includes('schedule'),
    includeTriggers: uiState.include_type.includes('trigger'),
    includeUsers: uiState.include_type.includes('user'),
    includeGroups: uiState.include_type.includes('group'),
    includeSettings: uiState.include_type.includes('settings'),
    includeKey: uiState.include_type.includes('key')
  };
}

export function syncOptionsToUIState(syncOptions: SyncOptions): UIState {
  const include_type: string[] = ['script', 'flow', 'app', 'folder']; // Always included

  // Add types based on skip flags (default to included if not specified)
  if (syncOptions.skipVariables !== true) include_type.push('variable');
  if (syncOptions.skipResources !== true) include_type.push('resource');
  if (syncOptions.skipResourceTypes !== true) include_type.push('resourcetype');
  if (syncOptions.skipSecrets !== true) include_type.push('secret');

  // Add types based on include flags (default to excluded if not specified)
  if (syncOptions.includeSchedules === true) include_type.push('schedule');
  if (syncOptions.includeTriggers === true) include_type.push('trigger');
  if (syncOptions.includeUsers === true) include_type.push('user');
  if (syncOptions.includeGroups === true) include_type.push('group');
  if (syncOptions.includeSettings === true) include_type.push('settings');
  if (syncOptions.includeKey === true) include_type.push('key');

  return {
    include_path: syncOptions.includes || ['f/**'],
    include_type
  };
}

// Helper function to handle JSON input parsing
export function parseJsonInput(jsonInput: string): UIState {
  try {
    return JSON.parse(jsonInput);
  } catch (e) {
    throw new Error("Invalid JSON in settings parameter: " + (e as Error).message);
  }
}