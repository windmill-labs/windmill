/**
 * Generates a commented wmill.yaml template with all available options
 * documented inline, making it easy for users and AI agents to discover
 * and configure settings.
 */
export function generateCommentedTemplate(branchName?: string): string {
  const branch = branchName ?? "main";

  return `# wmill.yaml — Windmill CLI configuration
# Full reference: run "wmill config reference"

# Default TypeScript runtime: "bun" or "deno"
defaultTs: bun

# Glob patterns for files to sync (relative to repo root)
includes:
  - "f/**"

# Additional glob patterns merged with includes (useful for branch overrides)
# extraIncludes: []

# Glob patterns to exclude from sync
excludes: []

# --- What to sync ---------------------------------------------------------
# "skip" options default to false (synced), "include" options default to false (not synced)

skipVariables: false
skipResources: false
skipResourceTypes: false
skipSecrets: true               # true by default — secrets are not synced for security
skipScripts: false
skipFlows: false
skipApps: false
skipFolders: false
skipWorkspaceDependencies: false

# Uncomment to include these (excluded by default):
# includeSchedules: true        # sync schedules
# includeTriggers: true         # sync triggers (http, websocket, kafka, etc.)
# includeUsers: true            # sync workspace users
# includeGroups: true           # sync workspace groups
# includeSettings: true         # sync workspace settings
# includeKey: true              # sync encryption key

# --- Sync behavior ---------------------------------------------------------

# Number of parallel operations during sync
# parallel: 4

# Require lock files for all scripts (fail sync if missing)
# locksRequired: true

# Run linting before push
# lint: true

# Handle secrets as plain text (not recommended for production)
# plainSecrets: false

# Branch name to use promotion overrides from during sync
# promotion: staging

# Skip validation that current git branch matches a configured branch
# skipBranchValidation: false

# Use non-dotted path convention: __flow, __app, __raw_app instead of .flow, .app, .raw_app
nonDottedPaths: true

# --- Codebase bundling (shared libraries) ----------------------------------
# Bundle TypeScript/JavaScript codebases that scripts import from.
# Each entry is bundled and uploaded so scripts can import shared code.

codebases: []
# codebases:
#   - relative_path: ./shared          # path to the codebase
#     includes: ["**/*.ts"]            # files to include in bundle
#     excludes: ["node_modules/**"]    # files to exclude
#     format: esm                      # bundle format: "cjs" or "esm"
#     external: ["pg", "axios"]        # dependencies to leave unbundled
#     assets:                          # static files to copy into bundle
#       - from: ./static
#         to: ./dist
#     # customBundler: ./build.ts      # custom bundler script (replaces esbuild)
#     # inject: ["./polyfills.ts"]     # files to inject into every entry point
#     # define:                        # compile-time constants
#     #   API_URL: '"https://api.example.com"'
#     # banner:                        # text prepended to output files
#     #   js: "/* bundled by windmill */"
#     # loader:                        # esbuild loader overrides
#     #   ".png": "dataurl"

# --- Git branch / environment bindings -------------------------------------
# Map git branches to Windmill workspaces and override settings per branch.
# Use "environments" as an alias if you prefer environment-based terminology.

gitBranches:
  ${branch}:
    overrides: {}
    # baseUrl: https://app.windmill.dev    # Windmill instance URL for this branch
    # workspaceId: my-workspace            # workspace to sync with
    # promotionOverrides:                  # overrides applied during --promotion
    #   skipSecrets: false
    # specificItems:                       # only sync these specific items
    #   variables: ["f/my_folder/my_var"]
    #   resources: ["f/my_folder/my_res"]
    #   triggers: ["f/my_folder/my_trigger"]
    #   folders: ["my_folder"]
    #   settings: true

  # Example: staging branch bound to a different workspace
  # staging:
  #   baseUrl: https://staging.windmill.dev
  #   workspaceId: staging-workspace
  #   overrides:
  #     skipSecrets: false
  #     includeSchedules: true

  # Items shared across ALL branches
  # commonSpecificItems:
  #   variables: ["f/shared/api_key"]
  #   resources: ["f/shared/db_conn"]
  #   folders: ["shared"]
`;
}

/**
 * Configuration option descriptor used by the config reference command.
 */
export interface ConfigOption {
  name: string;
  type: string;
  default: string;
  description: string;
}

/**
 * All wmill.yaml configuration options with their types, defaults, and descriptions.
 * This is the single source of truth used by both the template and the reference command.
 */
export const CONFIG_REFERENCE: ConfigOption[] = [
  // Core
  { name: "defaultTs", type: '"bun" | "deno"', default: "bun", description: "Default TypeScript runtime for new scripts" },
  { name: "includes", type: "string[]", default: '["f/**"]', description: "Glob patterns for files to include in sync" },
  { name: "extraIncludes", type: "string[]", default: "[]", description: "Additional glob patterns merged with includes (useful in branch overrides)" },
  { name: "excludes", type: "string[]", default: "[]", description: "Glob patterns for files to exclude from sync" },
  { name: "nonDottedPaths", type: "boolean", default: "true", description: "Use __flow/__app/__raw_app suffixes instead of .flow/.app/.raw_app" },

  // Skip flags
  { name: "skipVariables", type: "boolean", default: "false", description: "Skip syncing variables" },
  { name: "skipResources", type: "boolean", default: "false", description: "Skip syncing resources" },
  { name: "skipResourceTypes", type: "boolean", default: "false", description: "Skip syncing resource types" },
  { name: "skipSecrets", type: "boolean", default: "true", description: "Skip syncing secrets (true by default for security)" },
  { name: "skipScripts", type: "boolean", default: "false", description: "Skip syncing scripts" },
  { name: "skipFlows", type: "boolean", default: "false", description: "Skip syncing flows" },
  { name: "skipApps", type: "boolean", default: "false", description: "Skip syncing apps" },
  { name: "skipFolders", type: "boolean", default: "false", description: "Skip syncing folders" },
  { name: "skipWorkspaceDependencies", type: "boolean", default: "false", description: "Skip syncing workspace dependencies" },

  // Include flags
  { name: "includeSchedules", type: "boolean", default: "false", description: "Include schedules in sync" },
  { name: "includeTriggers", type: "boolean", default: "false", description: "Include triggers (http, websocket, kafka, etc.) in sync" },
  { name: "includeUsers", type: "boolean", default: "false", description: "Include workspace users in sync" },
  { name: "includeGroups", type: "boolean", default: "false", description: "Include workspace groups in sync" },
  { name: "includeSettings", type: "boolean", default: "false", description: "Include workspace settings in sync" },
  { name: "includeKey", type: "boolean", default: "false", description: "Include encryption key in sync" },

  // Behavior
  { name: "parallel", type: "number", default: "(unset)", description: "Number of parallel operations during sync" },
  { name: "locksRequired", type: "boolean", default: "false", description: "Require lock files for all scripts" },
  { name: "lint", type: "boolean", default: "false", description: "Run linting before push" },
  { name: "plainSecrets", type: "boolean", default: "false", description: "Handle secrets as plain text (not recommended)" },
  { name: "message", type: "string", default: "(unset)", description: "Default commit message for sync operations" },
  { name: "promotion", type: "string", default: "(unset)", description: "Branch name to use promotion overrides from during sync" },
  { name: "skipBranchValidation", type: "boolean", default: "false", description: "Skip validation that current git branch matches a configured branch" },

  // Codebases
  { name: "codebases", type: "Codebase[]", default: "[]", description: "Codebase bundling configurations for shared libraries" },
  { name: "codebases[].relative_path", type: "string", default: "(required)", description: "Path to the codebase directory" },
  { name: "codebases[].includes", type: "string[]", default: "(all files)", description: "Glob patterns for files to include in bundle" },
  { name: "codebases[].excludes", type: "string[]", default: "[]", description: "Glob patterns for files to exclude from bundle" },
  { name: "codebases[].format", type: '"cjs" | "esm"', default: "(unset)", description: "Bundle output format" },
  { name: "codebases[].external", type: "string[]", default: "[]", description: "Dependencies to leave unbundled (externals)" },
  { name: "codebases[].assets", type: "{from, to}[]", default: "[]", description: "Static files to copy into the bundle" },
  { name: "codebases[].customBundler", type: "string", default: "(unset)", description: "Path to a custom bundler script (replaces esbuild)" },
  { name: "codebases[].inject", type: "string[]", default: "[]", description: "Files to inject into every entry point" },
  { name: "codebases[].define", type: "Record<string, string>", default: "{}", description: "Compile-time constant definitions" },
  { name: "codebases[].banner", type: "Record<string, string>", default: "{}", description: "Text to prepend to output files by type" },
  { name: "codebases[].loader", type: "Record<string, string>", default: "{}", description: "esbuild loader overrides by extension" },

  // Git branches
  { name: "gitBranches", type: "object", default: "{}", description: "Map git branches to workspaces and per-branch sync overrides" },
  { name: "gitBranches.<branch>.baseUrl", type: "string", default: "(unset)", description: "Windmill instance URL for this branch" },
  { name: "gitBranches.<branch>.workspaceId", type: "string", default: "(unset)", description: "Workspace ID to sync with for this branch" },
  { name: "gitBranches.<branch>.overrides", type: "Partial<SyncOptions>", default: "{}", description: "Override any top-level sync option for this branch" },
  { name: "gitBranches.<branch>.promotionOverrides", type: "Partial<SyncOptions>", default: "{}", description: "Overrides applied when using --promotion flag" },
  { name: "gitBranches.<branch>.specificItems", type: "object", default: "(unset)", description: "Only sync specific items on this branch" },
  { name: "gitBranches.<branch>.specificItems.variables", type: "string[]", default: "[]", description: "Specific variable paths to sync" },
  { name: "gitBranches.<branch>.specificItems.resources", type: "string[]", default: "[]", description: "Specific resource paths to sync" },
  { name: "gitBranches.<branch>.specificItems.triggers", type: "string[]", default: "[]", description: "Specific trigger paths to sync" },
  { name: "gitBranches.<branch>.specificItems.folders", type: "string[]", default: "[]", description: "Specific folder paths to sync" },
  { name: "gitBranches.<branch>.specificItems.settings", type: "boolean", default: "false", description: "Whether to sync settings for this branch" },
  { name: "gitBranches.commonSpecificItems", type: "object", default: "(unset)", description: "Specific items shared across all branches" },
  { name: "gitBranches.commonSpecificItems.variables", type: "string[]", default: "[]", description: "Variable paths shared across all branches" },
  { name: "gitBranches.commonSpecificItems.resources", type: "string[]", default: "[]", description: "Resource paths shared across all branches" },
  { name: "gitBranches.commonSpecificItems.triggers", type: "string[]", default: "[]", description: "Trigger paths shared across all branches" },
  { name: "gitBranches.commonSpecificItems.folders", type: "string[]", default: "[]", description: "Folder paths shared across all branches" },
  { name: "gitBranches.commonSpecificItems.settings", type: "boolean", default: "false", description: "Whether to sync settings across all branches" },
  { name: "environments", type: "(alias)", default: "-", description: "Alias for gitBranches — use if you prefer environment-based terminology" },
];

/**
 * Format the config reference as a human-readable table for terminal output.
 */
export function formatConfigReference(): string {
  const nameWidth = 48;
  const typeWidth = 24;
  const defaultWidth = 14;

  const header = [
    "OPTION".padEnd(nameWidth),
    "TYPE".padEnd(typeWidth),
    "DEFAULT".padEnd(defaultWidth),
    "DESCRIPTION",
  ].join("  ");

  const separator = "-".repeat(header.length + 20);

  const rows = CONFIG_REFERENCE.map((opt) =>
    [
      opt.name.padEnd(nameWidth),
      opt.type.padEnd(typeWidth),
      opt.default.padEnd(defaultWidth),
      opt.description,
    ].join("  ")
  );

  return [
    "wmill.yaml — Configuration Reference",
    "",
    "Full documentation: https://www.windmill.dev/docs/advanced/cli",
    "",
    separator,
    header,
    separator,
    ...rows,
    separator,
    "",
    'Run "wmill init" to generate a wmill.yaml with commented examples.',
  ].join("\n");
}

/**
 * Format the config reference as JSON for programmatic consumption.
 */
export function formatConfigReferenceJson(): string {
  return JSON.stringify(CONFIG_REFERENCE, null, 2);
}
