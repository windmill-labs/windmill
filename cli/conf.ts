import { log, yamlParseFile } from "./deps.ts";
import { Workspace } from "./workspace.ts";

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
  overrides?: { [key: string]: Partial<SyncOptions> };
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
    if (conf?.defaultTs == undefined) {
      log.warn(
        "No defaultTs defined in your wmill.yaml. Using 'bun' as default."
      );
    }
    return typeof conf == "object" ? conf : ({} as SyncOptions);
  } catch (e) {
    log.warn(
      "No wmill.yaml found. Use 'wmill init' to bootstrap it. Using 'bun' as default typescript runtime."
    );
    return {};
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
  skipSecrets: false,
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

// Get effective settings by merging defaults, top-level settings, and overrides
export function getEffectiveSettings(
  config: SyncOptions,
  workspace: string,
  repo: string,
  workspaceObj?: Workspace
): SyncOptions {
  // Start with defaults
  let effective = { ...DEFAULT_SYNC_OPTIONS };
  
  // Merge top-level settings from config
  Object.keys(config).forEach(key => {
    if (key !== 'overrides' && config[key as keyof SyncOptions] !== undefined) {
      (effective as any)[key] = config[key as keyof SyncOptions];
    }
  });
  
  // Generate possible override keys in order of preference
  const possibleWorkspaceKeys = workspaceObj ? [
    `${workspaceObj.name}:*`,                          // Primary: "localhost_test:*"
    `${workspaceObj.remote}:${workspace}:*`,           // Disambiguated: "http://localhost:8000/:test:*"
    `${workspace}:*`,                                  // Legacy: "test:*"
  ] : [
    `${workspace}:*`,                                  // Legacy only
  ];

  const possibleRepoKeys = workspaceObj ? [
    `${workspaceObj.name}:${repo}`,                    // Primary: "localhost_test:u/user/repo"
    `${workspaceObj.remote}:${workspace}:${repo}`,     // Disambiguated: "http://localhost:8000/:test:u/user/repo"
    `${workspace}:${repo}`,                            // Legacy: "test:u/user/repo"
  ] : [
    `${workspace}:${repo}`,                            // Legacy only
  ];

  // Track which keys were found for validation
  let workspaceKeyFound = false;
  let repoKeyFound = false;
  const foundWorkspaceKeys: string[] = [];
  const foundRepoKeys: string[] = [];

  // Apply workspace-level overrides (first match wins)
  for (const key of possibleWorkspaceKeys) {
    if (config.overrides?.[key]) {
      if (!workspaceKeyFound) {
        Object.assign(effective, config.overrides[key]);
        workspaceKeyFound = true;
      }
      foundWorkspaceKeys.push(key);
    }
  }
  
  // Apply repository-specific overrides (first match wins, overrides workspace-level)
  for (const key of possibleRepoKeys) {
    if (config.overrides?.[key]) {
      if (!repoKeyFound) {
        Object.assign(effective, config.overrides[key]);
        repoKeyFound = true;
      }
      foundRepoKeys.push(key);
    }
  }

  // Validation and helpful error messages
  if (config.overrides && Object.keys(config.overrides).length > 0) {
    // Check for potential mismatched keys that contain this repo
    if (!repoKeyFound && repo) {
      const existingKeys = Object.keys(config.overrides);
      const repoMatches = existingKeys.filter(key => key.includes(repo));
      const workspaceMatches = existingKeys.filter(key => 
        workspaceObj ? 
          (key.includes(workspaceObj.name) || key.includes(workspace)) :
          key.includes(workspace)
      );

      if (repoMatches.length > 0 || workspaceMatches.length > 0) {
        const suggestions = [...new Set([...repoMatches, ...workspaceMatches])];
        console.warn(`⚠️  Override key not found for repository "${repo}".`);
        if (workspaceObj) {
          console.warn(`   Current workspace: "${workspaceObj.name}" (ID: ${workspace})`);
          console.warn(`   Expected key format: "${workspaceObj.name}:${repo}"`);
        } else {
          console.warn(`   Current workspace ID: "${workspace}"`);
          console.warn(`   Expected key format: "${workspace}:${repo}"`);
        }
        if (suggestions.length > 0) {
          console.warn(`   Found similar keys: ${suggestions.join(', ')}`);
          console.warn(`   Did you mean one of these keys?`);
        }
      }
    }

    // Warn about multiple matching keys
    if (foundWorkspaceKeys.length > 1) {
      console.warn(`⚠️  Multiple workspace-level override keys found: ${foundWorkspaceKeys.join(', ')}`);
      console.warn(`   Using: ${foundWorkspaceKeys[0]}`);
    }
    if (foundRepoKeys.length > 1) {
      console.warn(`⚠️  Multiple repository override keys found: ${foundRepoKeys.join(', ')}`);
      console.warn(`   Using: ${foundRepoKeys[0]}`);
    }

    // Check for potential instance ambiguity (multiple remote:workspaceId keys)
    if (workspaceObj && config.overrides) {
      const disambiguatedKeys = Object.keys(config.overrides).filter(key => 
        key.includes(':') && key.split(':').length === 3 && key.includes(workspace)
      );
      const uniqueRemotes = new Set(
        disambiguatedKeys.map(key => key.split(':')[0]).filter(remote => remote.startsWith('http'))
      );
      
      if (uniqueRemotes.size > 1) {
        console.warn(`⚠️  Multiple instances detected with same workspace ID "${workspace}"`);
        console.warn(`   Found instances: ${Array.from(uniqueRemotes).join(', ')}`);
        console.warn(`   Current instance: ${workspaceObj.remote}`);
        console.warn(`   Consider using disambiguated format: "${workspaceObj.remote}:${workspace}:${repo}"`);
      }
    }
  }
  
  return effective;
}
