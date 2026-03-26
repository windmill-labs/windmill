/**
 * Configuration option descriptor — drives both the reference table and the YAML template.
 */
export interface ConfigOption {
  name: string;
  type: string;
  default: string;
  description: string;

  // --- Template rendering hints (ignored by the reference table) ---

  /** Start a new section with this header (e.g., "What to sync") */
  section?: string;
  /** Note printed below the section header */
  sectionNote?: string;
  /** Render as commented out in template (default: false) */
  commented?: boolean;
  /** YAML value to show in template (defaults to `default` field) */
  templateValue?: string;
  /** Multi-line commented example block appended after this option's line */
  example?: string;
  /** Don't render in template — sub-fields shown via parent's example block */
  hidden?: boolean;
  /** Extra inline comment after the value (e.g., "true by default — secrets are not synced") */
  inlineComment?: string;
  /** Note printed above commented-out options group (e.g., "Uncomment to include these:") */
  groupNote?: string;
}

/**
 * All wmill.yaml configuration options.
 *
 * To add a new option:
 *   - For simple top-level options: add an entry with the right section/commented flags
 *   - For sub-fields of complex types (codebases[], gitBranches.<branch>.*):
 *     set `hidden: true` and add the field to the parent's `example` block
 */
export const CONFIG_REFERENCE: ConfigOption[] = [
  // ── Core ──────────────────────────────────────────────────────────────
  { name: "defaultTs", type: '"bun" | "deno"', default: "bun", description: "Default TypeScript runtime for new scripts" },
  { name: "includes", type: "string[]", default: '["f/**"]', description: "Glob patterns for files to include in sync",
    templateValue: '\n  - "f/**"' },
  { name: "extraIncludes", type: "string[]", default: "[]", description: "Additional glob patterns merged with includes (useful in branch overrides)",
    commented: true },
  { name: "excludes", type: "string[]", default: "[]", description: "Glob patterns for files to exclude from sync" },

  // ── What to sync ──────────────────────────────────────────────────────
  { name: "skipVariables", type: "boolean", default: "false", description: "Skip syncing variables",
    section: "What to sync", sectionNote: '"skip" options default to false (synced), "include" options default to false (not synced)' },
  { name: "skipResources", type: "boolean", default: "false", description: "Skip syncing resources" },
  { name: "skipResourceTypes", type: "boolean", default: "false", description: "Skip syncing resource types" },
  { name: "skipSecrets", type: "boolean", default: "true", description: "Skip syncing secrets (true by default for security)",
    inlineComment: "true by default — secrets are not synced for security" },
  { name: "skipScripts", type: "boolean", default: "false", description: "Skip syncing scripts" },
  { name: "skipFlows", type: "boolean", default: "false", description: "Skip syncing flows" },
  { name: "skipApps", type: "boolean", default: "false", description: "Skip syncing apps" },
  { name: "skipFolders", type: "boolean", default: "false", description: "Skip syncing folders" },
  { name: "skipWorkspaceDependencies", type: "boolean", default: "false", description: "Skip syncing workspace dependencies" },

  { name: "includeSchedules", type: "boolean", default: "false", description: "Include schedules in sync",
    commented: true, templateValue: "true", groupNote: "Uncomment to include these (excluded by default):" },
  { name: "includeTriggers", type: "boolean", default: "false", description: "Include triggers (http, websocket, kafka, etc.) in sync",
    commented: true, templateValue: "true" },
  { name: "includeUsers", type: "boolean", default: "false", description: "Include workspace users in sync",
    commented: true, templateValue: "true" },
  { name: "includeGroups", type: "boolean", default: "false", description: "Include workspace groups in sync",
    commented: true, templateValue: "true" },
  { name: "includeSettings", type: "boolean", default: "false", description: "Include workspace settings in sync",
    commented: true, templateValue: "true" },
  { name: "includeKey", type: "boolean", default: "false", description: "Include encryption key in sync",
    commented: true, templateValue: "true" },

  // ── Sync behavior ─────────────────────────────────────────────────────
  { name: "parallel", type: "number", default: "(unset)", description: "Number of parallel operations during sync",
    section: "Sync behavior", commented: true, templateValue: "4" },
  { name: "locksRequired", type: "boolean", default: "false", description: "Require lock files for all scripts",
    commented: true, templateValue: "true" },
  { name: "lint", type: "boolean", default: "false", description: "Run linting before push",
    commented: true, templateValue: "true" },
  { name: "plainSecrets", type: "boolean", default: "false", description: "Handle secrets as plain text (not recommended)",
    commented: true },
  { name: "message", type: "string", default: "(unset)", description: "Default commit message for sync operations",
    commented: true, templateValue: '"my commit message"' },
  { name: "promotion", type: "string", default: "(unset)", description: "Branch name to use promotion overrides from during sync",
    commented: true, templateValue: "staging" },
  { name: "skipBranchValidation", type: "boolean", default: "false", description: "Skip validation that current git branch matches a configured branch",
    commented: true },
  { name: "nonDottedPaths", type: "boolean", default: "true", description: "Use __flow/__app/__raw_app suffixes instead of .flow/.app/.raw_app" },

  // ── Codebase bundling ─────────────────────────────────────────────────
  { name: "codebases", type: "Codebase[]", default: "[]", description: "Codebase bundling configurations for shared libraries",
    section: "Codebase bundling (shared libraries)",
    sectionNote: "Bundle TypeScript/JavaScript codebases that scripts import from.\nEach entry is bundled and uploaded so scripts can import shared code.",
    example: [
      "# codebases:",
      '#   - relative_path: ./shared          # path to the codebase',
      '#     includes: ["**/*.ts"]            # files to include in bundle',
      '#     excludes: ["node_modules/**"]    # files to exclude',
      '#     format: esm                      # bundle format: "cjs" or "esm"',
      '#     external: ["pg", "axios"]        # dependencies to leave unbundled',
      "#     assets:                          # static files to copy into bundle",
      "#       - from: ./static",
      "#         to: ./dist",
      "#     # customBundler: ./build.ts      # custom bundler script (replaces esbuild)",
      '#     # inject: ["./polyfills.ts"]     # files to inject into every entry point',
      "#     # define:                        # compile-time constants",
      "#     #   API_URL: '\"https://api.example.com\"'",
      "#     # banner:                        # text prepended to output files",
      '#     #   js: "/* bundled by windmill */"',
      "#     # loader:                        # esbuild loader overrides",
      '#     #   ".png": "dataurl"',
    ].join("\n"),
  },
  // Sub-fields shown via codebases example above
  { name: "codebases[].relative_path", type: "string", default: "(required)", description: "Path to the codebase directory", hidden: true },
  { name: "codebases[].includes", type: "string[]", default: "(all files)", description: "Glob patterns for files to include in bundle", hidden: true },
  { name: "codebases[].excludes", type: "string[]", default: "[]", description: "Glob patterns for files to exclude from bundle", hidden: true },
  { name: "codebases[].format", type: '"cjs" | "esm"', default: "(unset)", description: "Bundle output format", hidden: true },
  { name: "codebases[].external", type: "string[]", default: "[]", description: "Dependencies to leave unbundled (externals)", hidden: true },
  { name: "codebases[].assets", type: "{from, to}[]", default: "[]", description: "Static files to copy into the bundle", hidden: true },
  { name: "codebases[].customBundler", type: "string", default: "(unset)", description: "Path to a custom bundler script (replaces esbuild)", hidden: true },
  { name: "codebases[].inject", type: "string[]", default: "[]", description: "Files to inject into every entry point", hidden: true },
  { name: "codebases[].define", type: "Record<string, string>", default: "{}", description: "Compile-time constant definitions", hidden: true },
  { name: "codebases[].banner", type: "Record<string, string>", default: "{}", description: "Text to prepend to output files by type", hidden: true },
  { name: "codebases[].loader", type: "Record<string, string>", default: "{}", description: "esbuild loader overrides by extension", hidden: true },

  // ── Git branches ──────────────────────────────────────────────────────
  { name: "gitBranches", type: "object", default: "{}", description: "Map git branches to workspaces and per-branch sync overrides",
    section: "Git branch / environment bindings",
    sectionNote: "Map git branches to Windmill workspaces and override settings per branch.\nUse \"environments\" as an alias if you prefer environment-based terminology.",
    // The template for gitBranches is special — it uses {{BRANCH}} placeholder
    templateValue: "\n  {{BRANCH}}:\n    overrides: {}",
    example: [
      "    # baseUrl: https://app.windmill.dev    # Windmill instance URL for this branch",
      "    # workspaceId: my-workspace            # workspace to sync with",
      "    # promotionOverrides:                  # overrides applied during --promotion",
      "    #   skipSecrets: false",
      "    # specificItems:                       # only sync these specific items",
      '    #   variables: ["f/my_folder/my_var"]',
      '    #   resources: ["f/my_folder/my_res"]',
      '    #   triggers: ["f/my_folder/my_trigger"]',
      '    #   folders: ["my_folder"]',
      "    #   settings: true",
      "",
      "  # Example: staging branch bound to a different workspace",
      "  # staging:",
      "  #   baseUrl: https://staging.windmill.dev",
      "  #   workspaceId: staging-workspace",
      "  #   overrides:",
      "  #     skipSecrets: false",
      "  #     includeSchedules: true",
      "",
      "  # Items shared across ALL branches",
      "  # commonSpecificItems:",
      '  #   variables: ["f/shared/api_key"]',
      '  #   resources: ["f/shared/db_conn"]',
      '  #   folders: ["shared"]',
    ].join("\n"),
  },
  // Sub-fields shown via gitBranches example above
  { name: "gitBranches.<branch>.baseUrl", type: "string", default: "(unset)", description: "Windmill instance URL for this branch", hidden: true },
  { name: "gitBranches.<branch>.workspaceId", type: "string", default: "(unset)", description: "Workspace ID to sync with for this branch", hidden: true },
  { name: "gitBranches.<branch>.overrides", type: "Partial<SyncOptions>", default: "{}", description: "Override any top-level sync option for this branch", hidden: true },
  { name: "gitBranches.<branch>.promotionOverrides", type: "Partial<SyncOptions>", default: "{}", description: "Overrides applied when using --promotion flag", hidden: true },
  { name: "gitBranches.<branch>.specificItems", type: "object", default: "(unset)", description: "Only sync specific items on this branch", hidden: true },
  { name: "gitBranches.<branch>.specificItems.variables", type: "string[]", default: "[]", description: "Specific variable paths to sync", hidden: true },
  { name: "gitBranches.<branch>.specificItems.resources", type: "string[]", default: "[]", description: "Specific resource paths to sync", hidden: true },
  { name: "gitBranches.<branch>.specificItems.triggers", type: "string[]", default: "[]", description: "Specific trigger paths to sync", hidden: true },
  { name: "gitBranches.<branch>.specificItems.folders", type: "string[]", default: "[]", description: "Specific folder paths to sync", hidden: true },
  { name: "gitBranches.<branch>.specificItems.settings", type: "boolean", default: "false", description: "Whether to sync settings for this branch", hidden: true },
  { name: "gitBranches.commonSpecificItems", type: "object", default: "(unset)", description: "Specific items shared across all branches", hidden: true },
  { name: "gitBranches.commonSpecificItems.variables", type: "string[]", default: "[]", description: "Variable paths shared across all branches", hidden: true },
  { name: "gitBranches.commonSpecificItems.resources", type: "string[]", default: "[]", description: "Resource paths shared across all branches", hidden: true },
  { name: "gitBranches.commonSpecificItems.triggers", type: "string[]", default: "[]", description: "Trigger paths shared across all branches", hidden: true },
  { name: "gitBranches.commonSpecificItems.folders", type: "string[]", default: "[]", description: "Folder paths shared across all branches", hidden: true },
  { name: "gitBranches.commonSpecificItems.settings", type: "boolean", default: "false", description: "Whether to sync settings across all branches", hidden: true },
  { name: "environments", type: "(alias)", default: "-", description: "Alias for gitBranches — use if you prefer environment-based terminology", hidden: true },
];

// ─── Template generator ─────────────────────────────────────────────────────

/**
 * Generates a commented wmill.yaml template from CONFIG_REFERENCE.
 *
 * Each ConfigOption is rendered as either an active line (`key: value`)
 * or a commented-out line (`# key: value`), with description comments
 * and optional example blocks.
 */
export function generateCommentedTemplate(branchName?: string): string {
  const branch = branchName ?? "main";
  const lines: string[] = [
    "# yaml-language-server: $schema=wmill.schema.json",
    "# wmill.yaml — Windmill CLI configuration",
    '# Full reference: run "wmill config"',
    "",
  ];

  for (const opt of CONFIG_REFERENCE) {
    if (opt.hidden) continue;

    // Section header
    if (opt.section) {
      const ruler = "-".repeat(Math.max(0, 65 - opt.section.length));
      lines.push(`# --- ${opt.section} ${ruler}`);
      if (opt.sectionNote) {
        for (const noteLine of opt.sectionNote.split("\n")) {
          lines.push(`# ${noteLine}`);
        }
      }
      lines.push("");
    }

    // Group note (e.g., "Uncomment to include these:")
    if (opt.groupNote) {
      lines.push(`# ${opt.groupNote}`);
    }

    const value = opt.templateValue ?? opt.default;
    const resolvedValue = value.replace("{{BRANCH}}", branch);

    if (opt.commented) {
      // Commented-out option: # description  →  # key: value
      lines.push(`# ${opt.description}`);
      lines.push(`# ${opt.name}: ${resolvedValue}`);
    } else {
      // Active option: # description  →  key: value
      lines.push(`# ${opt.description}`);
      if (opt.inlineComment) {
        // Value with inline trailing comment
        const base = `${opt.name}: ${resolvedValue}`;
        const pad = " ".repeat(Math.max(1, 32 - base.length));
        lines.push(`${base}${pad}# ${opt.inlineComment}`);
      } else {
        lines.push(`${opt.name}: ${resolvedValue}`);
      }
    }

    // Example block
    if (opt.example) {
      const resolvedExample = opt.example.replace(/\{\{BRANCH\}\}/g, branch);
      for (const exLine of resolvedExample.split("\n")) {
        lines.push(exLine);
      }
    }

    lines.push("");
  }

  return lines.join("\n");
}

// ─── Reference formatters ───────────────────────────────────────────────────

/**
 * Format the config reference as a human-readable table for terminal output.
 */
export function formatConfigReference(): string {
  const nameWidth = 48;
  const descWidth = 70;

  const header = [
    "OPTION".padEnd(nameWidth),
    "DESCRIPTION".padEnd(descWidth),
    "DEFAULT",
  ].join("  ");

  const separator = "-".repeat(header.length + 10);

  const rows = CONFIG_REFERENCE.map((opt) =>
    [
      opt.name.padEnd(nameWidth),
      opt.description.padEnd(descWidth),
      opt.default,
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
  // Strip template-only fields from JSON output
  const clean = CONFIG_REFERENCE.map(({ name, type, default: def, description }) => ({
    name, type, default: def, description,
  }));
  return JSON.stringify(clean, null, 2);
}

// ─── JSON Schema generator ──────────────────────────────────────────────────

/** Map CONFIG_REFERENCE type strings to JSON Schema type descriptors. */
function typeToJsonSchema(type: string): Record<string, any> {
  // Exact matches
  if (type === "boolean") return { type: "boolean" };
  if (type === "number") return { type: "integer" };
  if (type === "string") return { type: "string" };
  if (type === "string[]") return { type: "array", items: { type: "string" } };
  if (type === '"bun" | "deno"') return { type: "string", enum: ["bun", "deno"] };
  if (type === '"cjs" | "esm"') return { type: "string", enum: ["cjs", "esm"] };
  if (type === "Record<string, string>") return { type: "object", additionalProperties: { type: "string" } };
  if (type === "{from, to}[]") return {
    type: "array",
    items: {
      type: "object",
      properties: {
        from: { type: "string", description: "Source path" },
        to: { type: "string", description: "Destination path" },
      },
      required: ["from", "to"],
    },
  };
  // Fallback
  return { type: "object" };
}

/** Build the specificItems sub-schema from gitBranches.<branch>.specificItems.* entries. */
function specificItemsSchema(): Record<string, any> {
  const prefix = "gitBranches.<branch>.specificItems.";
  const props: Record<string, any> = {};
  for (const opt of CONFIG_REFERENCE) {
    if (!opt.name.startsWith(prefix)) continue;
    const key = opt.name.slice(prefix.length);
    props[key] = { ...typeToJsonSchema(opt.type), description: opt.description };
  }
  return {
    type: "object",
    description: "Sync only specific items",
    properties: props,
    additionalProperties: false,
  };
}

/** Build the Codebase item schema from codebases[].* entries in CONFIG_REFERENCE. */
function codebaseItemSchema(): Record<string, any> {
  const props: Record<string, any> = {};
  for (const opt of CONFIG_REFERENCE) {
    if (!opt.name.startsWith("codebases[].")) continue;
    const key = opt.name.replace("codebases[].", "");
    props[key] = { ...typeToJsonSchema(opt.type), description: opt.description };
  }
  return {
    type: "object",
    properties: props,
    required: ["relative_path"],
    additionalProperties: false,
  };
}

/** Build the per-branch config schema. It extends the top-level SyncOptions. */
function branchConfigSchema(topLevelProps: Record<string, any>): Record<string, any> {
  return {
    type: "object",
    properties: {
      baseUrl: { type: "string", description: "Windmill instance URL for this branch" },
      workspaceId: { type: "string", description: "Workspace ID to sync with for this branch" },
      overrides: {
        type: "object",
        description: "Override any top-level sync option for this branch",
        properties: topLevelProps,
        additionalProperties: false,
      },
      promotionOverrides: {
        type: "object",
        description: "Overrides applied when using --promotion flag",
        properties: topLevelProps,
        additionalProperties: false,
      },
      specificItems: specificItemsSchema(),
    },
    additionalProperties: false,
  };
}

/**
 * Generate a JSON Schema for wmill.yaml, derived from CONFIG_REFERENCE.
 *
 * The schema enables editor autocomplete, inline documentation, and validation
 * when referenced via a `# yaml-language-server: $schema=...` directive.
 */
export function generateJsonSchema(): Record<string, any> {
  // Collect top-level simple properties (not codebases, not gitBranches, not sub-fields)
  const topLevelProps: Record<string, any> = {};
  for (const opt of CONFIG_REFERENCE) {
    // Skip sub-fields, complex types, and aliases
    if (opt.name.includes(".") || opt.name.includes("[")) continue;
    if (opt.name === "codebases" || opt.name === "gitBranches" || opt.name === "environments") continue;
    topLevelProps[opt.name] = { ...typeToJsonSchema(opt.type), description: opt.description };
  }

  const branchSchema = branchConfigSchema(topLevelProps);

  const schema: Record<string, any> = {
    $schema: "http://json-schema.org/draft-07/schema#",
    title: "wmill.yaml",
    description: "Windmill CLI configuration file. Full reference: wmill config",
    type: "object",
    properties: {
      ...topLevelProps,
      codebases: {
        type: "array",
        description: "Codebase bundling configurations for shared libraries",
        items: codebaseItemSchema(),
      },
      gitBranches: {
        type: "object",
        description: "Map git branches to workspaces and per-branch sync overrides",
        properties: {
          commonSpecificItems: specificItemsSchema(),
        },
        additionalProperties: branchSchema,
      },
      environments: {
        type: "object",
        description: "Alias for gitBranches — use if you prefer environment-based terminology",
        properties: {
          commonSpecificItems: specificItemsSchema(),
        },
        additionalProperties: branchSchema,
      },
    },
    additionalProperties: false,
  };

  return schema;
}
