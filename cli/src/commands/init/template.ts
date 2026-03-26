/**
 * Configuration option descriptor — each entry IS a JSON Schema property
 * with extra metadata for template rendering and reference table display.
 *
 * To generate the JSON Schema: iterate entries, strip NON_SCHEMA_KEYS, done.
 * Sub-fields of complex types (codebases items, gitBranches branch config)
 * are defined inline in the parent's schema — no duplicate entries needed.
 * The reference table auto-expands nested schemas into rows.
 *
 * Adding a new option:
 *   1. Add an entry to CONFIG_REFERENCE with JSON Schema type fields + description
 *   2. Add template rendering hints (section, commented, templateValue, etc.)
 *   3. `wmill init` (YAML template), `wmill config` (table), and wmill.schema.json all update automatically
 */
export interface ConfigOption {
  // --- JSON Schema fields (kept when generating schema) ---
  type: string;
  description: string;
  enum?: string[];
  items?: Record<string, any>;
  properties?: Record<string, any>;
  additionalProperties?: Record<string, any> | boolean;
  required?: string[];

  // --- Non-schema metadata (stripped when generating schema) ---
  name: string;
  default: string;

  // --- Template rendering hints (also stripped) ---
  section?: string;
  sectionNote?: string;
  commented?: boolean;
  templateValue?: string;
  example?: string;
  inlineComment?: string;
  groupNote?: string;
}

/** Keys to strip from ConfigOption entries when generating JSON Schema. */
const NON_SCHEMA_KEYS = new Set([
  "name", "default",
  "section", "sectionNote", "commented", "templateValue",
  "example", "inlineComment", "groupNote",
]);

// Reusable sub-schemas for nested types
const SPECIFIC_ITEMS_SCHEMA = {
  type: "object",
  description: "Sync only specific items",
  properties: {
    variables: { type: "array", items: { type: "string" }, description: "Specific variable paths to sync" },
    resources: { type: "array", items: { type: "string" }, description: "Specific resource paths to sync" },
    triggers: { type: "array", items: { type: "string" }, description: "Specific trigger paths to sync" },
    folders: { type: "array", items: { type: "string" }, description: "Specific folder paths to sync" },
    settings: { type: "boolean", description: "Whether to sync settings" },
  },
  additionalProperties: false,
} as const;

const BRANCH_CONFIG_SCHEMA = {
  type: "object",
  properties: {
    baseUrl: { type: "string", description: "Windmill instance URL for this branch" },
    workspaceId: { type: "string", description: "Workspace ID to sync with for this branch" },
    overrides: { type: "object", description: "Override any top-level sync option for this branch" },
    promotionOverrides: { type: "object", description: "Overrides applied when using --promotion flag" },
    specificItems: SPECIFIC_ITEMS_SCHEMA,
  },
  additionalProperties: false,
} as const;

/**
 * All wmill.yaml configuration options — single source of truth.
 * Each entry is a JSON Schema property with extra metadata.
 */
export const CONFIG_REFERENCE: ConfigOption[] = [
  // ── Core ──────────────────────────────────────────────────────────────
  { name: "defaultTs", type: "string", enum: ["bun", "deno"], default: "bun", description: "Default TypeScript runtime for new scripts" },
  { name: "includes", type: "array", items: { type: "string" }, default: '["f/**"]', description: "Glob patterns for files to include in sync",
    templateValue: '\n  - "f/**"' },
  { name: "extraIncludes", type: "array", items: { type: "string" }, default: "[]", description: "Additional glob patterns merged with includes (useful in branch overrides)",
    commented: true },
  { name: "excludes", type: "array", items: { type: "string" }, default: "[]", description: "Glob patterns for files to exclude from sync" },

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
  { name: "parallel", type: "integer", default: "(unset)", description: "Number of parallel operations during sync",
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
  { name: "codebases", type: "array", default: "[]", description: "Codebase bundling configurations for shared libraries",
    items: {
      type: "object",
      properties: {
        relative_path: { type: "string", description: "Path to the codebase directory" },
        includes: { type: "array", items: { type: "string" }, description: "Glob patterns for files to include in bundle" },
        excludes: { type: "array", items: { type: "string" }, description: "Glob patterns for files to exclude from bundle" },
        format: { type: "string", enum: ["cjs", "esm"], description: "Bundle output format" },
        external: { type: "array", items: { type: "string" }, description: "Dependencies to leave unbundled (externals)" },
        assets: { type: "array", items: { type: "object", properties: { from: { type: "string" }, to: { type: "string" } }, required: ["from", "to"] }, description: "Static files to copy into the bundle" },
        customBundler: { type: "string", description: "Path to a custom bundler script (replaces esbuild)" },
        inject: { type: "array", items: { type: "string" }, description: "Files to inject into every entry point" },
        define: { type: "object", additionalProperties: { type: "string" }, description: "Compile-time constant definitions" },
        banner: { type: "object", additionalProperties: { type: "string" }, description: "Text to prepend to output files by type" },
        loader: { type: "object", additionalProperties: { type: "string" }, description: "esbuild loader overrides by extension" },
      },
      required: ["relative_path"],
      additionalProperties: false,
    },
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

  // ── Git branches ──────────────────────────────────────────────────────
  { name: "gitBranches", type: "object", default: "{}", description: "Map git branches to workspaces and per-branch sync overrides",
    properties: { commonSpecificItems: SPECIFIC_ITEMS_SCHEMA },
    additionalProperties: BRANCH_CONFIG_SCHEMA,
    section: "Git branch / environment bindings",
    sectionNote: "Map git branches to Windmill workspaces and override settings per branch.\nUse \"environments\" as an alias if you prefer environment-based terminology.",
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

  { name: "environments", type: "object", default: "{}", description: "Alias for gitBranches — use if you prefer environment-based terminology",
    properties: { commonSpecificItems: SPECIFIC_ITEMS_SCHEMA },
    additionalProperties: BRANCH_CONFIG_SCHEMA,
    commented: true },
];

// ─── Template generator ─────────────────────────────────────────────────────

/** Quote a string for use as a YAML key if it contains special characters. */
function yamlKey(s: string): string {
  return /^[a-zA-Z0-9_/.@-]+$/.test(s) ? s : `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

export function generateCommentedTemplate(branchName?: string): string {
  const branch = yamlKey(branchName ?? "main");
  const lines: string[] = [
    "# yaml-language-server: $schema=wmill.schema.json",
    "# wmill.yaml — Windmill CLI configuration",
    '# Full reference: run "wmill config"',
    "",
  ];

  for (const opt of CONFIG_REFERENCE) {
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

    if (opt.groupNote) {
      lines.push(`# ${opt.groupNote}`);
    }

    const value = opt.templateValue ?? opt.default;
    const resolvedValue = value.replace("{{BRANCH}}", branch);

    if (opt.commented) {
      lines.push(`# ${opt.description}`);
      lines.push(`# ${opt.name}: ${resolvedValue}`);
    } else {
      lines.push(`# ${opt.description}`);
      if (opt.inlineComment) {
        const base = `${opt.name}: ${resolvedValue}`;
        const pad = " ".repeat(Math.max(1, 32 - base.length));
        lines.push(`${base}${pad}# ${opt.inlineComment}`);
      } else {
        lines.push(`${opt.name}: ${resolvedValue}`);
      }
    }

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

/** Recursively expand a schema's properties into flat reference rows. */
function expandSchema(
  prefix: string,
  schema: Record<string, any>,
  rows: { name: string; description: string; default: string }[]
): void {
  if (schema.properties) {
    for (const [key, prop] of Object.entries(schema.properties) as [string, Record<string, any>][]) {
      const name = prefix ? `${prefix}.${key}` : key;
      rows.push({ name, description: prop.description ?? "", default: "" });
      // Recurse into nested object properties (e.g., specificItems)
      if (prop.properties && prop.type === "object") {
        expandSchema(name, prop, rows);
      }
    }
  }
}

export function formatConfigReference(): string {
  const nameWidth = 48;
  const descWidth = 70;

  const header = [
    "OPTION".padEnd(nameWidth),
    "DESCRIPTION".padEnd(descWidth),
    "DEFAULT",
  ].join("  ");

  const separator = "-".repeat(header.length + 10);

  const allRows: { name: string; description: string; default: string }[] = [];
  for (const opt of CONFIG_REFERENCE) {
    allRows.push({ name: opt.name, description: opt.description, default: opt.default });

    // Auto-expand array item properties (e.g., codebases[].*)
    if (opt.items?.properties) {
      expandSchema(`${opt.name}[]`, opt.items, allRows);
    }
    // Auto-expand additionalProperties (e.g., gitBranches.<branch>.*)
    if (opt.additionalProperties && typeof opt.additionalProperties === "object" && opt.additionalProperties.properties) {
      expandSchema(`${opt.name}.<branch>`, opt.additionalProperties as Record<string, any>, allRows);
    }
    // Auto-expand named properties (e.g., gitBranches.commonSpecificItems)
    if (opt.properties) {
      expandSchema(opt.name, opt, allRows);
    }
  }

  const rows = allRows.map((r) =>
    [r.name.padEnd(nameWidth), r.description.padEnd(descWidth), r.default].join("  ")
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

export function formatConfigReferenceJson(): string {
  const clean = CONFIG_REFERENCE.map((opt) => ({
    name: opt.name, type: opt.type, default: opt.default, description: opt.description,
  }));
  return JSON.stringify(clean, null, 2);
}

// ─── JSON Schema generator ──────────────────────────────────────────────────

/**
 * Generate a JSON Schema for wmill.yaml by stripping non-schema keys from CONFIG_REFERENCE.
 */
export function generateJsonSchema(): Record<string, any> {
  const properties: Record<string, any> = {};
  for (const opt of CONFIG_REFERENCE) {
    const entry: Record<string, any> = {};
    for (const [k, v] of Object.entries(opt)) {
      if (!NON_SCHEMA_KEYS.has(k) && k !== "name") {
        entry[k] = v;
      }
    }
    properties[opt.name] = entry;
  }
  return {
    $schema: "http://json-schema.org/draft-07/schema#",
    title: "wmill.yaml",
    description: "Windmill CLI configuration file. Full reference: wmill config",
    type: "object",
    properties,
    additionalProperties: false,
  };
}
