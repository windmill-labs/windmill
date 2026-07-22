import { requireLogin } from "../../core/auth.ts";
import { fetchVersion, resolveWorkspace } from "../../core/context.ts";
import { writeFile, readdir, stat, rm, copyFile, mkdir } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import * as path from "node:path";
import { sep as SEP } from "node:path";
import { stringify as yamlStringify, type DocumentOptions, type SchemaOptions, type CreateNodeOptions, type ToStringOptions } from "yaml";
import JSZip from "jszip";
import { minimatch } from "minimatch";
import { yamlParseContent } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";

import {
  getTypeStrFromPath,
  GlobalOptions,
  parseFromPath,
  pushObj,
  removeType,
  showConflict,
  showDiff,
  extractNativeTriggerInfo,
  redactEncryptionKey,
  isDatatableMigrationPath,
  parseDatatableMigrationPath,
} from "../../types.ts";
import { downloadZip } from "./pull.ts";
import { runLint, printReport, checkMissingLocks } from "../lint/lint.ts";
import { pullSharedUi, pushSharedUi } from "../shared_ui.ts";
import {
  pushMigrationFromDisk,
  offerToRunNewMigrations,
  validateLocalMigrations,
} from "../datatable_migrations.ts";

import {
  exts,
  findContentFile,
  findResourceFile,
  handleScriptMetadata,
  removeExtensionToPath,
  filePathExtensionFromContentType,
} from "../script/script.ts";

import { handleFile } from "../script/script.ts";
import {
  deepEqual,
  fetchRemoteVersion,
  getHeaders,
  isFileResource,
  isFilesetResource,
  isRawAppFile,
  isWorkspaceDependencies,
  readTextFile,
} from "../../utils/utils.ts";
import {
  getEffectiveSettings,
  getWorkspaceNames,
  mergeConfigWithConfigFile,
  parseSyncBehavior,
  SyncOptions,
  validateBranchConfiguration,
  findWorkspaceByGitBranch,
  WorkspaceEntryConfig,
} from "../../core/conf.ts";
import type { PermissionedAsContext } from "../../core/permissioned_as.ts";
import { preCheckPermissionedAs } from "../../core/permissioned_as.ts";
import {
  fromWorkspaceSpecificPath,
  getWorkspaceSpecificPath,
  getSpecificItemsForCurrentBranch,
  isWorkspaceSpecificFile,
  isCurrentWorkspaceFile,
  isItemTypeConfigured,
  isSpecificItem,
  SpecificItemsConfig,
} from "../../core/specific_items.ts";
import {
  getCurrentGitBranch,
  isGitRepository,
  computeGitSyncDeployBranch,
  checkoutGitSyncDeployBranch,
  gitSyncDeployPush,
  deriveGitSyncDeployIncludes,
  isForkWorkspace,
  type GitSyncDeployItem,
} from "../../utils/git.ts";
import { Workspace } from "../workspace/workspace.ts";
import { removePathPrefix } from "../../types.ts";
import { listSyncCodebases, SyncCodebase } from "../../utils/codebase.ts";
import {
  generateScriptMetadataInternal,
  getRawWorkspaceDependencies,
  readLockfile,
  UnknownLockVersionError,
  MalformedLockfileError,
  workspaceDependenciesPathToLanguageAndFilename,
} from "../../utils/metadata.ts";
import { DoubleLinkedDependencyTree, uploadScripts } from "../../utils/dependency_tree.ts";
import { OpenFlow, NativeServiceName, ScriptModule } from "../../../gen/types.gen.ts";
import { pushResource } from "../resource/resource.ts";
import {
  newPathAssigner,
  newRawAppPathAssigner,
  PathAssigner,
} from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";
import { extractInlineScripts as extractInlineScriptsForFlows, extractCurrentMapping } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";
import { generateFlowLockInternal } from "../flow/flow_metadata.ts";
import { isExecutionModeAnonymous } from "../app/app.ts";
import {
  APP_BACKEND_FOLDER,
  generateAppLocksInternal,
} from "../app/app_metadata.ts";
import {
  isFlowPath,
  isAppPath,
  isRawAppPath,
  extractFolderPath,
  extractResourceName,
  isFlowMetadataFile,
  isAppMetadataFile,
  isRawAppMetadataFile,
  isRawAppFolderMetadataFile,
  isAppFolderMetadataFile,
  isFlowFolderMetadataFile,
  getDeleteSuffix,
  transformJsonPathToDir,
  getFolderSuffix,
  getFolderSuffixWithSep,
  getNonDottedPaths,
  isScriptModulePath,
  getModuleFolderSuffix,
  isModuleEntryPoint,
  getScriptBasePathFromModulePath,
  hasWrongFormatSuffix,
} from "../../utils/resource_folders.ts";

let branchDeprecationWarned = false;

// Map a ws_specific `item_kind` returned by the backend to its corresponding
// SpecificItemsConfig array. The backend currently only emits `resource` and
// `variable`, but the mapping rule (kind → its plural — `_trigger` kinds all
// fold into `triggers`) is generic so a future kind doesn't require a change
// here. Returns null for kinds we have no place to store (e.g. `settings`,
// which is a single boolean, not a list).
function configKeyForItemKind(
  kind: string,
): keyof Omit<SpecificItemsConfig, "settings"> | null {
  switch (kind) {
    case "resource":
      return "resources";
    case "variable":
      return "variables";
      // case "schedule":
      //   return "schedules";
      // default:
      //   return kind.endsWith("_trigger") ? "triggers" : null;
    }
    return null
}

// Fetch ws_specific items from the server and merge their paths into specificItems.
// Each ws_specific entry (item_kind + path) is appended as an exact file-path pattern
// to the corresponding array in the config, so the existing pattern-matching logic picks them up.
// Returns the merged config plus the raw server list (or null if the server list could
// not be fetched) — callers performing push-side reconciliation need to know which
// (kind, path) pairs are *not yet* ws_specific on the server.
async function mergeWsSpecificFromServer(
  workspaceId: string,
  specificItems: SpecificItemsConfig | undefined,
): Promise<{
  merged: SpecificItemsConfig | undefined;
  serverItems: Array<{ item_kind: string; path: string }> | null;
}> {
  let wsSpecificItems: Array<{ item_kind: string; path: string }>;
  try {
    wsSpecificItems = await wmill.listWsSpecific({ workspace: workspaceId });
  } catch (err) {
    // 404 = endpoint not present on an older server: expected, log at debug.
    // Anything else (401/403/network) is a real failure that produces an
    // incomplete sync — surface it so the user notices.
    const isApiError = err && typeof err === "object" &&
      "name" in err && (err as { name: unknown }).name === "ApiError";
    const status = isApiError ? (err as { status?: number }).status : undefined;
    if (status === 404) {
      log.debug("listWsSpecific endpoint not available on server, skipping");
    } else {
      const msg = err instanceof Error ? err.message : String(err);
      log.warn(
        `Could not fetch ws_specific items from server (${status ?? "no status"}): ${msg}. ` +
          `Sync will proceed without server-side ws_specific items.`,
      );
    }
    return { merged: specificItems, serverItems: null };
  }

  if (wsSpecificItems.length === 0) {
    return { merged: specificItems, serverItems: wsSpecificItems };
  }

  const merged: SpecificItemsConfig = specificItems ? { ...specificItems } : {};

  for (const item of wsSpecificItems) {
    const configKey = configKeyForItemKind(item.item_kind);
    if (!configKey) continue;
    if (!merged[configKey]) {
      merged[configKey] = [];
    }
    // Patterns are .yaml regardless of opts.json — isSpecificItem normalizes
    // .json file paths to .yaml before matching, so a single .yaml pattern
    // covers both extensions for the same item.
    merged[configKey]!.push(`${item.path}.${item.item_kind}.yaml`);
  }

  return { merged, serverItems: wsSpecificItems };
}

// Compute (kind, serverPath) pairs that are flagged ws_specific by the local
// config (specificItems) but are *not* marked ws_specific on the server. These
// represent flag-only changes that the file-content diff misses (because the
// flag isn't part of the YAML body), so push needs to handle them explicitly.
//
// Generic over item kind (uses getTypeStrFromPath + removeType so .yaml/.json
// work the same), gated by configKeyForItemKind so only kinds the backend can
// mark ws_specific make it through. As of writing the backend only emits
// `resource` and `variable`, but the gating handles future kinds without
// touching this function.
export function computeWsSpecificFlagOnlyPushes(
  localMap: Record<string, string>,
  localSpecificItems: SpecificItemsConfig | undefined,
  serverItems: Array<{ item_kind: string; path: string }> | null,
): Array<{ kind: string; serverPath: string; filePath: string }> {
  if (!localSpecificItems || serverItems === null) return [];

  const serverSet = new Set(
    serverItems.map((i) => `${i.item_kind}:${i.path}`),
  );

  const out: Array<{ kind: string; serverPath: string; filePath: string }> = [];
  for (const filePath of Object.keys(localMap)) {
    let kind: string;
    try {
      kind = getTypeStrFromPath(filePath);
    } catch {
      continue;
    }
    if (configKeyForItemKind(kind) === null) continue;

    if (!isSpecificItem(filePath, localSpecificItems)) continue;

    const serverPath = removeType(filePath, kind);
    if (serverSet.has(`${kind}:${serverPath}`)) continue;

    out.push({ kind, serverPath, filePath });
  }
  return out;
}

// Resolve workspace name from a --branch override (git branch → workspace name).
// Falls back to using the branch value as-is (backward compat: old key = branch name).
function resolveWsNameFromBranch(opts: SyncOptions, branchName: string): string {
  const match = findWorkspaceByGitBranch(opts.workspaces, branchName);
  return match ? match[0] : branchName;
}

// Resolve wsNameForConfig from CLI flags. Prefers --branch → matching config key,
// then --workspace → matching config key (incl. when --base-url is set). Returns
// undefined when no flag-based resolution applies; callers then fall back to
// inferWsNameFromProfile on the resolved workspace profile.
export function resolveWsNameForConfigFromFlags(
  opts: SyncOptions & { branch?: string; workspace?: string },
): string | undefined {
  if (opts.branch) {
    return resolveWsNameFromBranch(opts, opts.branch);
  }
  if (opts.workspace) {
    // Use getWorkspaceNames so reserved keys (e.g. commonSpecificItems) are filtered out,
    // matching the behavior of findWorkspaceByGitBranch / inferWsNameFromProfile.
    const validKeys = getWorkspaceNames(opts.workspaces);
    if (validKeys.includes(opts.workspace)) {
      return opts.workspace;
    }
  }
  return undefined;
}

// Warn if --workspace overrides auto-detected branch or if workspace not in config.
function warnWorkspaceOverride(opts: SyncOptions, wsNameForConfig: string | undefined): void {
  if (!wsNameForConfig || !opts.workspaces) return;

  // Check if workspace exists in config
  const wsEntry = (opts.workspaces as any)?.[wsNameForConfig] as WorkspaceEntryConfig | undefined;
  if (!wsEntry) {
    const wsNames = Object.keys(opts.workspaces).filter((k) => k !== "commonSpecificItems");
    if (wsNames.length > 0) {
      log.warn(
        `⚠️  Workspace '${wsNameForConfig}' is not defined in the 'workspaces' section of wmill.yaml.\n` +
        `   No workspace-specific overrides will be applied. Available workspaces: ${wsNames.join(", ")}`
      );
    }
    return;
  }

  // Check if current git branch maps to a different workspace
  if (isGitRepository()) {
    const currentBranch = getCurrentGitBranch();
    if (currentBranch) {
      const autoMatch = findWorkspaceByGitBranch(opts.workspaces, currentBranch);
      if (autoMatch && autoMatch[0] !== wsNameForConfig) {
        log.info(
          `Current git branch '${currentBranch}' maps to workspace '${autoMatch[0]}', ` +
          `but --workspace overrides to '${wsNameForConfig}'.`
        );
      }
    }
  }
}

// The workspace name is used as the file suffix for workspace-specific files.
// This is a pass-through — the workspace name (config key) IS the suffix.
function resolveWsNameForFiles(_opts: SyncOptions, wsName: string): string {
  return wsName;
}

// After resolveWorkspace, infer the workspace config name from the resolved profile
// by matching baseUrl + workspaceId against the workspaces config entries.
function inferWsNameFromProfile(opts: SyncOptions, profile: { remote: string; workspaceId: string }): string | undefined {
  if (!opts.workspaces) return undefined;
  const wsNames = Object.keys(opts.workspaces).filter((k) => k !== "commonSpecificItems");
  for (const name of wsNames) {
    const entry = (opts.workspaces as any)[name] as WorkspaceEntryConfig;
    if (!entry?.baseUrl) continue;
    try {
      const entryUrl = new URL(entry.baseUrl).toString();
      const profileUrl = new URL(profile.remote).toString();
      const entryWsId = entry.workspaceId ?? name;
      if (entryUrl === profileUrl && entryWsId === profile.workspaceId) {
        return name;
      }
    } catch {
      continue;
    }
  }
  return undefined;
}

// Merge CLI options with effective settings, preserving CLI flags as overrides
function mergeCliWithEffectiveOptions<
  T extends GlobalOptions & SyncOptions & { repository?: string },
>(cliOpts: T, effectiveOpts: SyncOptions): T {
  // overlay CLI options on top (undefined cliOpts won't override effectiveOpts)
  return Object.assign({}, effectiveOpts, cliOpts) as T;
}

// Resolve effective sync options using workspace-based configuration
async function resolveEffectiveSyncOptions(
  workspace: Workspace,
  localConfig: SyncOptions,
  promotion?: string,
  workspaceNameOverride?: string,
): Promise<SyncOptions> {
  return await getEffectiveSettings(localConfig, promotion, false, false, workspaceNameOverride);
}

type DynFSElement = {
  isDirectory: boolean;
  path: string;
  whitelistedExt?: boolean;
  // getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<DynFSElement>;
};

export function findCodebase(
  path: string,
  codebases: SyncCodebase[],
): SyncCodebase | undefined {
  if (!path.endsWith(".ts")) {
    return;
  }
  for (const c of codebases) {
    let included = false;
    let excluded = false;
    if (c.includes == undefined || c.includes == null) {
      included = true;
    }
    if (typeof c.includes == "string") {
      c.includes = [c.includes];
    }
    for (const r of c.includes ?? []) {
      if (included) {
        break;
      }
      if (minimatch(path, r)) {
        included = true;
      }
    }
    if (typeof c.excludes == "string") {
      c.excludes = [c.excludes];
    }
    for (const r of c.excludes ?? []) {
      if (minimatch(path, r)) {
        excluded = true;
      }
    }
    if (included && !excluded) {
      return c;
    }
  }
}

async function addCodebaseDigestIfRelevant(
  path: string,
  content: string,
  codebases: SyncCodebase[],
  ignoreCodebaseChanges: boolean,
): Promise<string> {
  const isScript = path.endsWith(".script.yaml");
  if (!isScript) {
    return content;
  }
  let isTs = true;
  const replacedPath = path.replace(".script.yaml", ".ts");
  try {
    await stat(replacedPath);
  } catch {
    isTs = false;
  }
  if (!isTs) {
    return content;
  }
  if (isTs) {
    const c = findCodebase(replacedPath, codebases);
    if (c) {
      let parsed: any;
      try {
        parsed = yamlParseContent(path, content);
      } catch (error) {
        log.error(
          `Failed to parse YAML content for codebase digest at path: ${path}`,
        );
        throw error;
      }
      if (parsed && typeof parsed == "object") {
        if (ignoreCodebaseChanges) {
          parsed["codebase"] = undefined;
        } else {
          parsed["codebase"] = await c.getDigest();
        }
        parsed["lock"] = "";
        return yamlStringify(parsed, yamlOptions);
      } else {
        throw Error(
          `Expected local yaml ${path} to be an object, found: ${content} instead`,
        );
      }
    }
  }
  return content;
}

export async function FSFSElement(
  p: string,
  codebases: SyncCodebase[],
  ignoreCodebaseChanges: boolean,
): Promise<DynFSElement> {
  function _internal_element(
    localP: string,
    isDir: boolean,
    codebases: SyncCodebase[],
  ): DynFSElement {
    return {
      isDirectory: isDir,
      path: localP.substring(p.length + 1),
      async *getChildren(): AsyncIterable<DynFSElement> {
        if (!isDir) return [];
        try {
          const entries = await readdir(localP, { withFileTypes: true });
          for (const e of entries) {
            yield _internal_element(
              path.join(localP, e.name),
              e.isDirectory(),
              codebases,
            );
          }
        } catch (e) {
          log.warn(`Error reading dir: ${localP}, ${e}`);
        }
      },
      async getContentText(): Promise<string> {
        const content = await readTextFile(localP);
        const itemPath = localP.substring(p.length + 1);
        const r = await addCodebaseDigestIfRelevant(
          itemPath,
          content,
          codebases,
          ignoreCodebaseChanges,
        );
        return r;
      },
    };
  }
  return _internal_element(p, (await stat(p)).isDirectory(), codebases);
}

function prioritizeName(name: string): string {
  if (name == "version") return "aaa";
  if (name == "id") return "aa";
  if (name == "type") return "ab";
  if (name == "summary") return "ad";
  if (name == "name") return "ae";
  if (name == "display_name") return "af";
  if (name == "description") return "ag";
  if (name == "value") return "ah";
  if (name == "content") return "ai";
  if (name == "modules") return "aj";
  if (name == "failure_module") return "ak";
  if (name == "input_transforms") return "al";
  if (name == "lock") return "az";
  if (name == "locks") return "azz";

  return name;
}

export const yamlOptions: DocumentOptions & SchemaOptions & CreateNodeOptions & ToStringOptions = {
  sortMapEntries: (a, b) => {
    return prioritizeName(String(a.key)).localeCompare(prioritizeName(String(b.key)));
  },
  aliasDuplicateObjects: false,
  singleQuote: true,
};

/**
 * Iterate object/array entries in the same order they will appear in the
 * YAML output. Arrays preserve their index order; plain objects are sorted
 * by the same comparator as `yamlOptions.sortMapEntries`.
 *
 * Use this whenever traversal order influences a side effect that the YAML
 * representation also expresses (e.g. auto-numbered inline-script paths in
 * `extractInlineScriptsForApps`). Without it, the path-assigner walks the
 * in-memory key order returned by `JSON.parse` (server insertion order),
 * but the YAML serializer reorders keys alphabetically — so identically
 * named scripts get numbers that don't line up with their position on disk
 * and shuffle between pulls when the server returns keys in a different order.
 */
export function yamlSortedEntries(rec: any): [string, any][] {
  const entries = Object.entries(rec);
  if (Array.isArray(rec)) {
    return entries;
  }
  entries.sort(([a], [b]) =>
    prioritizeName(a).localeCompare(prioritizeName(b)),
  );
  return entries;
}

export interface InlineScript {
  path: string;
  content: string;
}

function extractFields(fields: Record<string, any>) {
  Object.entries(fields).forEach(([k, v]) => {
    if (typeof v == "object") {
      if (v.type == "static") {
        fields[k] = { value: v.value };
      } else if (v.type == "javascript") {
        fields[k] = { expr: v.expr, allowUserResources: v.allowUserResources };
      } else if (v.type == "user") {
        fields[k] = undefined;
      }
    }
  });
}

export function extractFieldsForRawApps(runnables: Record<string, any>) {
  Object.values(runnables).forEach((v) => {
    if (typeof v == "object") {
      if (v.fields !== undefined) {
        extractFields(v.fields);
      }
    }
  });
}

/**
 * Generates AGENTS.md - app-specific configuration for AI agents working with raw apps.
 * References the raw-app skill for complete documentation and includes instance-specific
 * data configuration (datatable, schema, whitelisted tables).
 */
export function generateAgentsDocumentation(data: {
  tables?: string[];
  datatable?: string;
  schema?: string;
} | undefined): string {
  const tables = data?.tables ?? [];
  const defaultDatatable = data?.datatable;
  const defaultSchema = data?.schema;

  return `# AI Agent Instructions

For complete raw app documentation (app structure, backend runnables, datatables, SQL migrations), use the \`raw-app\` skill.

This file contains **app-specific configuration** for this raw app instance.

---

## Data Configuration

${defaultDatatable
  ? `**Default Datatable:** \`${defaultDatatable}\`${defaultSchema ? ` | **Default Schema:** \`${defaultSchema}\`` : ''}`
  : '**No default datatable configured.** Set \`data.datatable\` in \`raw_app.yaml\` to enable database access.'}

### Whitelisted Tables

${tables.length > 0
  ? `These tables are accessible to this app:\n\n${tables.map(t => `- \`${t}\``).join('\n')}`
  : `**No tables whitelisted.** Add tables to \`data.tables\` in \`raw_app.yaml\`.`}

### Adding a Table

Edit \`raw_app.yaml\`:

\`\`\`yaml
data:
  datatable: ${defaultDatatable || 'main'}
  ${defaultSchema ? `schema: ${defaultSchema}\n  ` : ''}tables:
${tables.length > 0 ? tables.map(t => `    - ${t}`).join('\n') : '    # Add tables here'}
    - ${defaultDatatable || 'main'}/${defaultSchema ? defaultSchema + ':' : ''}new_table  # ← Add like this
\`\`\`

**Table reference formats:**
- \`<datatable>/<table>\` - Table in public schema
- \`<datatable>/<schema>:<table>\` - Table in specific schema

---

## Quick Reference

**Backend runnable:** Add \`backend/<name>.ts\` (or .py, etc.), then run \`wmill generate-metadata\`

**Call from frontend:**
\`\`\`typescript
import { backend } from './wmill';
const result = await backend.<name>({ arg: 'value' });
\`\`\`

**Query datatable (TypeScript):**
\`\`\`typescript
const sql = wmill.datatable();
const rows = await sql\`SELECT * FROM table WHERE id = \${id}\`.fetch();
\`\`\`

**SQL migrations:** Add \`.sql\` files to \`sql_to_apply/\`, run \`wmill app dev\`, then whitelist tables

---
*Run \`wmill app generate-agents\` to refresh. See \`.claude/skills/raw-app\` skill for full documentation.*
`;
}

/**
 * Generates a simple DATATABLES.md with just the current configuration summary.
 * The detailed schema information is generated by generate_datatables.ts command.
 */
export function generateDatatablesDocumentation(data: {
  tables?: string[];
  datatable?: string;
  schema?: string;
} | undefined): string {
  const tables = data?.tables ?? [];
  const defaultDatatable = data?.datatable;
  const defaultSchema = data?.schema;

  return `# Data Tables

This file contains the database schema information for this app.
Run \`wmill app generate-agents\` to refresh with current workspace schemas.

**For full instructions, see \`AGENTS.md\`.**

## Current Configuration

${defaultDatatable
  ? `**Default Datatable:** \`${defaultDatatable}\`${defaultSchema ? ` | **Default Schema:** \`${defaultSchema}\`` : ''}`
  : '**No default datatable configured.**'}

## Whitelisted Tables

${tables.length > 0
  ? `${tables.map(t => `- \`${t}\``).join('\n')}`
  : `*No tables whitelisted. Add tables to \`data.tables\` in \`raw_app.yaml\`.*`}

---

## Schema Information

Run \`wmill app generate-agents\` to populate this section with detailed schema information
from the workspace datatables.

---
*Auto-generated. Run \`wmill app generate-agents\` to refresh schemas.*
`;
}
export function extractInlineScriptsForApps(
  key: string | undefined,
  rec: any,
  pathAssigner: PathAssigner,
  toId: (key: string, val: any) => string,
  removeSchema: boolean,
): InlineScript[] {
  if (!rec) {
    return [];
  }
  if (typeof rec == "object") {
    // Iterate in YAML output order so that auto-numbered names assigned by
    // the path-assigner line up with the position they will appear in the
    // serialized YAML — and stay stable across pulls regardless of the key
    // order the server returns. See yamlSortedEntries above.
    return yamlSortedEntries(rec).flatMap(([k, v]) => {
      if (k == "inlineScript" && v != null && typeof v == "object") {
        rec["type"] = undefined;
        const o: Record<string, any> = v as any;
        const name = toId(key ?? "", rec);
        const [basePathO, ext] = pathAssigner.assignPath(name, o["language"]);
        const basePath = basePathO.replaceAll(SEP, "/");
        const r = [];
        if (o["content"]) {
          const content = o["content"];
          o["content"] = "!inline " + basePath.replaceAll(SEP, "/") + ext;
          r.push({
            path: basePath + ext,
            content: content,
          });
        }
        if (o["lock"] && o["lock"] != "") {
          const lock = o["lock"];
          o["lock"] = "!inline " + basePath.replaceAll(SEP, "/") + "lock";
          r.push({
            path: basePath + "lock",
            content: lock,
          });
        }
        if (removeSchema) {
          o.schema = undefined;
        }
        return r;
      } else {
        return extractInlineScriptsForApps(
          k,
          v,
          pathAssigner,
          toId,
          removeSchema,
        );
      }
    });
  }
  return [];
}

type FileResourceTypeInfo = { format_extension: string | null; is_fileset: boolean };

function parseFileResourceTypeMap(
  raw: Record<string, string | FileResourceTypeInfo>,
): { formatExtMap: Record<string, string>; filesetMap: Record<string, boolean> } {
  const formatExtMap: Record<string, string> = {};
  const filesetMap: Record<string, boolean> = {};
  for (const [k, v] of Object.entries(raw)) {
    if (typeof v === "string") {
      formatExtMap[k] = v;
      filesetMap[k] = false;
    } else {
      if (v.format_extension) {
        formatExtMap[k] = v.format_extension;
      }
      filesetMap[k] = v.is_fileset ?? false;
    }
  }
  return { formatExtMap, filesetMap };
}

async function findFilesetResourceFile(changePath: string): Promise<string> {
  // Extract the base path before .fileset/
  const filesetIdx = changePath.indexOf(".fileset" + SEP);
  if (filesetIdx === -1) {
    throw new Error(`Not a fileset resource path: ${changePath}`);
  }
  const basePath = changePath.substring(0, filesetIdx);
  const candidates = [basePath + ".resource.json", basePath + ".resource.yaml"];

  for (const candidate of candidates) {
    try {
      const s = await stat(candidate);
      if (s.isFile()) return candidate;
    } catch {
      // not found, try next
    }
  }
  throw new Error(`No resource metadata file found for fileset resource: ${changePath}`);
}

type FilesetPushResult =
  | { status: "pushed"; resourceFilePath: string }
  | { status: "already-synced"; resourceFilePath: string }
  | { status: "parent-missing" };

async function pushFilesetParentResource(
  childPath: string,
  workspaceId: string,
  alreadySynced: string[],
  cachedWsName: string | null,
  specificItems?: SpecificItemsConfig,
): Promise<FilesetPushResult> {
  let resourceFilePath: string;
  try {
    resourceFilePath = await findFilesetResourceFile(childPath);
  } catch {
    return { status: "parent-missing" };
  }
  if (alreadySynced.includes(resourceFilePath)) {
    return { status: "already-synced", resourceFilePath };
  }
  alreadySynced.push(resourceFilePath);

  const newObj = parseFromPath(
    resourceFilePath,
    await readTextFile(resourceFilePath),
  );

  let serverPath = resourceFilePath;
  let wsSpecific = false;
  if (cachedWsName && isWorkspaceSpecificFile(resourceFilePath)) {
    serverPath = fromWorkspaceSpecificPath(resourceFilePath, cachedWsName);
    wsSpecific = true;
  } else if (specificItems && isSpecificItem(childPath, specificItems)) {
    wsSpecific = true;
  }

  await pushResource(
    workspaceId,
    serverPath,
    undefined,
    newObj,
    resourceFilePath,
    wsSpecific ? true : undefined,
  );
  return { status: "pushed", resourceFilePath };
}

function ZipFSElement(
  zip: JSZip,
  useYaml: boolean,
  defaultTs: "bun" | "deno",
  resourceTypeToFormatExtension: Record<string, string>,
  resourceTypeToIsFileset: Record<string, boolean>,
  ignoreCodebaseChanges: boolean,
  stripOnBehalfOf: boolean,
): DynFSElement {
  // Pre-scan: find zip base paths of scripts that have modules.
  // These scripts use the folder layout: {basePath}__mod/script.{ext}
  let _moduleScriptPaths: Set<string> | null = null;
  async function getModuleScriptPaths(): Promise<Set<string>> {
    if (_moduleScriptPaths === null) {
      _moduleScriptPaths = new Set();
      for (const filename in zip.files) {
        if (filename.endsWith(".script.json") && !zip.files[filename].dir) {
          try {
            const content = await zip.files[filename].async("text");
            const parsed = JSON.parse(content);
            if (parsed.modules && Object.keys(parsed.modules).length > 0) {
              _moduleScriptPaths.add(
                filename.slice(0, -".script.json".length)
              );
            }
          } catch {}
        }
      }
    }
    return _moduleScriptPaths;
  }

  async function _internal_file(
    p: string,
    f: JSZip.JSZipObject,
  ): Promise<DynFSElement[]> {
    const kind:
      | "flow"
      | "app"
      | "script"
      | "resource"
      | "other"
      | "raw_app"
      | "dependencies" = isFlowMetadataFile(p)
      ? "flow"
      : isAppMetadataFile(p)
        ? "app"
        : isRawAppMetadataFile(p)
          ? "raw_app"
          : p.endsWith(".script.json")
            ? "script"
            : p.endsWith(".resource.json")
              ? "resource"
              : p.startsWith("dependencies/")
                ? "dependencies"
                : "other";

    const isJson = p.endsWith(".json");

    function transformPath() {
      if (kind == "flow") {
        return transformJsonPathToDir(p, "flow");
      } else if (kind == "app") {
        return transformJsonPathToDir(p, "app");
      } else if (kind == "raw_app") {
        return transformJsonPathToDir(p, "raw_app");
      } else if (kind == "dependencies") {
        return p;
      } else {
        return useYaml && isJson ? p.replaceAll(".json", ".yaml") : p;
      }
    }

    let finalPath = transformPath();

    // Redirect content files for scripts with modules into __mod/ folder
    if (kind == "other" && exts.some((ext) => p.endsWith(ext))) {
      const normalizedP = p.replace(/^\.[\\/]/, "");
      const moduleScripts = await getModuleScriptPaths();
      for (const basePath of moduleScripts) {
        if (normalizedP.startsWith(basePath + ".")) {
          const ext = normalizedP.slice(basePath.length); // e.g., ".ts", ".py"
          const dir = path.dirname(finalPath);
          const base = path.basename(basePath);
          finalPath = path.join(dir, base + getModuleFolderSuffix(), "script" + ext);
          break;
        }
      }
    }

    const r = [
      {
        isDirectory: kind == "flow" || kind == "app" || kind == "raw_app",
        path: finalPath,
        async *getChildren(): AsyncIterable<DynFSElement> {
          if (kind == "flow") {
            let flow: OpenFlow;
            try {
              flow = JSON.parse(await f.async("text"));
            } catch (error) {
              log.error(`Failed to parse flow.yaml at path: ${p}`);
              throw error;
            }
            let inlineScripts;
            try {
              const assigner = newPathAssigner(defaultTs, { skipInlineScriptSuffix: getNonDottedPaths() });
              // Preserve original !inline filenames from the flow to avoid phantom renames
              const inlineMapping = extractCurrentMapping(
                flow.value.modules as any,
                {},
                flow.value.failure_module,
                flow.value.preprocessor_module,
              );
              inlineScripts = extractInlineScriptsForFlows(
                flow.value.modules as any,
                inlineMapping,
                SEP,
                defaultTs,
                assigner,
                { skipInlineScriptSuffix: getNonDottedPaths(), failOnInlineDirective: true },
              );
              if (flow.value.failure_module) {
                inlineScripts.push(...extractInlineScriptsForFlows(
                  [flow.value.failure_module],
                  inlineMapping,
                  SEP,
                  defaultTs,
                  assigner,
                  { skipInlineScriptSuffix: getNonDottedPaths(), failOnInlineDirective: true },
                ));
              }
              if (flow.value.preprocessor_module) {
                inlineScripts.push(...extractInlineScriptsForFlows(
                  [flow.value.preprocessor_module],
                  inlineMapping,
                  SEP,
                  defaultTs,
                  assigner,
                  { skipInlineScriptSuffix: getNonDottedPaths(), failOnInlineDirective: true },
                ));
              }
            } catch (error) {
              log.error(
                `Failed to extract inline scripts for flow at path: ${p}`,
              );
              throw error;
            }
            for (const s of inlineScripts) {
              yield {
                isDirectory: false,
                path: path.join(finalPath, s.path),
                async *getChildren() {},
                async getContentText() {
                  return s.content;
                },
              };
            }

            if (stripOnBehalfOf) {
              // Only emit the flag when set; a `false` here is the default and
              // would produce a spurious diff for every ownerless flow.
              if ((flow as any).on_behalf_of_email) {
                (flow as any).has_on_behalf_of = true;
              }
              delete (flow as any).on_behalf_of_email;
            }

            yield {
              isDirectory: false,
              path: path.join(finalPath, "flow.yaml"),
              async *getChildren() {},
              async getContentText() {
                return yamlStringify(flow, yamlOptions);
              },
            };
          } else if (kind == "app") {
            let app;
            try {
              app = JSON.parse(await f.async("text"));
            } catch (error) {
              log.error(`Failed to parse app.yaml at path: ${p}`);
              throw error;
            }
            let inlineScripts;
            try {
              inlineScripts = extractInlineScriptsForApps(
                undefined,
                app?.["value"],
                newPathAssigner(defaultTs, { skipInlineScriptSuffix: getNonDottedPaths() }),
                (_, val) => val["name"],
                false,
              );
            } catch (error) {
              log.error(
                `Failed to extract inline scripts for app at path: ${p}`,
              );
              throw error;
            }

            for (const s of inlineScripts) {
              yield {
                isDirectory: false,
                path: path.join(finalPath, s.path),
                async *getChildren() {},
                async getContentText() {
                  return s.content;
                },
              };
            }

            if (isExecutionModeAnonymous(app)) {
              app.public = true;
            }
            app.policy = undefined;
            yield {
              isDirectory: false,
              path: path.join(finalPath, "app.yaml"),
              async *getChildren() {},
              async getContentText() {
                return yamlStringify(app, yamlOptions);
              },
            };
          } else if (kind == "raw_app") {
            let rawApp;
            try {
              rawApp = JSON.parse(await f.async("text"));
            } catch (error) {
              log.error(`Failed to parse app.yaml at path: ${p}`);
              throw error;
            }
            if (rawApp?.["policy"]?.["execution_mode"] == "anonymous") {
              rawApp.public = true;
            }
            // console.log("rawApp", rawApp);
            rawApp.policy = undefined;
            // custom_path is derived from the file path, don't store it
            delete rawApp?.["custom_path"];
            let inlineScripts;
            const value = rawApp?.["value"];
            const runnables = value?.["runnables"] ?? {};
            // console.log("FOOB", value?.["runnables"])
            extractFieldsForRawApps(runnables);
            try {
              inlineScripts = extractInlineScriptsForApps(
                undefined,
                value,
                newRawAppPathAssigner(defaultTs),
                (key, val_) => key,
                true,
              );
            } catch (error) {
              log.error(
                `Failed to extract inline scripts for raw app at path: ${p}`,
              );
              throw error;
            }

            try {
              for (const [filePath, content] of Object.entries(
                value?.["files"] ?? [],
              )) {
                // Skip generated/dev-only files and folders
                if (
                  filePath.startsWith("/sql_to_apply/") ||
                  filePath === "/wmill.d.ts" ||
                  filePath === "/AGENTS.md" ||
                  filePath === "/DATATABLES.md"
                ) {
                  continue;
                }
                yield {
                  isDirectory: false,
                  path: path.join(finalPath, filePath.substring(1)),
                  async *getChildren() {},
                    async getContentText() {
                    if (typeof content !== "string") {
                      throw new Error(
                        `Content of raw app file ${filePath} is not a string`,
                      );
                    }
                    return content as string;
                  },
                };
              }
            } catch (error) {
              log.error(`Failed to extract files for raw app at path: ${p}`);
              throw error;
            }

            // Yield inline script content and lock files
            for (const s of inlineScripts) {
              yield {
                isDirectory: false,
                path: path.join(finalPath, APP_BACKEND_FOLDER, s.path),
                async *getChildren() {},
                async getContentText() {
                  return s.content;
                },
              };
            }

            // Helper to simplify fields for YAML output
            // { type: 'static', value: X } -> { value: X }
            // { type: 'ctx', ctx: X } -> { ctx: X }
            function simplifyFields(fields: Record<string, any> | undefined) {
              if (!fields) return undefined;
              const simplified: Record<string, any> = {};
              for (const [k, v] of Object.entries(fields)) {
                if (typeof v === "object" && v !== null) {
                  if (v.type === "static" && v.value !== undefined) {
                    simplified[k] = { value: v.value };
                  } else if (v.type === "ctx" && v.ctx !== undefined) {
                    simplified[k] = { ctx: v.ctx };
                  } else {
                    // Keep other field types as-is
                    simplified[k] = v;
                  }
                } else {
                  simplified[k] = v;
                }
              }
              return simplified;
            }

            // Yield each runnable as a separate YAML file in the backend folder
            // For inline scripts, simplify the YAML - inlineScript is not needed since
            // content/lock/language can be derived from sibling files
            for (const [runnableId, runnable] of Object.entries(runnables)) {
              const runnableObj = runnable as Record<string, any>;
              let simplifiedRunnable: Record<string, any>;

              if (runnableObj.inlineScript) {
                // For inline scripts, remove inlineScript and just keep type: 'inline'
                // plus any other metadata (name, fields, etc.)
                simplifiedRunnable = { type: "inline" };

                // Copy over any other fields that aren't inlineScript or type
                for (const [key, value] of Object.entries(runnableObj)) {
                  if (key !== "inlineScript" && key !== "type") {
                    simplifiedRunnable[key] = value;
                  }
                }
              } else if (runnableObj.type === "path" && runnableObj.runType) {
                // For path-based runnables, convert from API format to file format
                // { type: "path", runType: "script" } -> { type: "script" }
                // Also remove schema field
                const {
                  type: _type,
                  runType,
                  schema: _schema,
                  ...rest
                } = runnableObj;
                simplifiedRunnable = {
                  type: runType,
                  ...rest,
                };
              } else {
                // For other runnables, keep as-is
                simplifiedRunnable = runnableObj;
              }

              // Simplify fields for cleaner YAML output
              if (simplifiedRunnable.fields) {
                simplifiedRunnable.fields = simplifyFields(simplifiedRunnable.fields);
              }

              yield {
                isDirectory: false,
                path: path.join(
                  finalPath,
                  APP_BACKEND_FOLDER,
                  `${runnableId}.yaml`,
                ),
                async *getChildren() {},
                async getContentText() {
                  return yamlStringify(simplifiedRunnable, yamlOptions);
                },
              };
            }

            // Extract data field from value before deleting it
            const data = value?.["data"];
            if (data) {
              rawApp.data = data;
            }

            // Remove runnables and value from raw_app.yaml - they are now in separate files
            delete rawApp?.["value"];
            // Don't include runnables in raw_app.yaml anymore

            yield {
              isDirectory: false,
              path: path.join(finalPath, "raw_app.yaml"),
              async *getChildren() {},
              async getContentText() {
                return yamlStringify(rawApp, yamlOptions);
              },
            };

            // Yield DATATABLES.md documentation file for AI agents
            yield {
              isDirectory: false,
              path: path.join(finalPath, "DATATABLES.md"),
              async *getChildren() {},
              async getContentText() {
                return generateDatatablesDocumentation(data);
              },
            };
          }
        },

        async getContentText(): Promise<string> {
          const content = await f.async("text");

          if (kind == "script") {
            let parsed;
            try {
              parsed = JSON.parse(content);
            } catch (error) {
              log.error(`Failed to parse script.yaml at path: ${p}`);
              throw error;
            }
            const hasModules = parsed["modules"] && Object.keys(parsed["modules"]).length > 0;
            if (
              parsed["lock"] &&
              parsed["lock"] != "" &&
              parsed["codebase"] == undefined
            ) {
              if (hasModules) {
                // Lock lives inside __mod/ folder as script.lock
                const scriptBase = removeSuffix(removeSuffix(p.replaceAll(SEP, "/"), ".json"), ".script");
                parsed["lock"] =
                  "!inline " + scriptBase + getModuleFolderSuffix() + "/script.lock";
              } else {
                parsed["lock"] =
                  "!inline " +
                  removeSuffix(p.replaceAll(SEP, "/"), ".json") +
                  ".lock";
              }
            } else if (parsed["lock"] == "") {
              parsed["lock"] = "";
            } else {
              parsed["lock"] = undefined;
            }
            if (ignoreCodebaseChanges && parsed["codebase"]) {
              parsed["codebase"] = undefined;
            }
            if (stripOnBehalfOf) {
              // Only emit the flag when set; a `false` here is the default and
              // would produce a spurious diff for every ownerless script.
              if (parsed["on_behalf_of_email"]) {
                parsed["has_on_behalf_of"] = true;
              }
              delete parsed["on_behalf_of_email"];
            }
            // Modules are stored as files in __mod/ folder, not in metadata
            delete parsed["modules"];
            return useYaml
              ? yamlStringify(parsed, yamlOptions)
              : JSON.stringify(parsed, null, 2);
          }

          if (kind == "resource") {
            const content = await f.async("text");
            let parsed;
            try {
              parsed = JSON.parse(content);
            } catch (error) {
              log.error(`Failed to parse resource.yaml at path: ${p}`);
              throw error;
            }
            const resourceType = parsed["resource_type"];
            const formatExtension =
              resourceTypeToFormatExtension[resourceType];
            const isFileset = resourceTypeToIsFileset[resourceType] ?? false;

            if (isFileset) {
              parsed["value"] =
                "!inline_fileset " +
                removeSuffix(p.replaceAll(SEP, "/"), ".resource.json") +
                ".fileset";
            } else if (formatExtension) {
              parsed["value"]["content"] =
                "!inline " +
                removeSuffix(p.replaceAll(SEP, "/"), ".resource.json") +
                ".resource.file." +
                formatExtension;
            }
            return useYaml
              ? yamlStringify(parsed, yamlOptions)
              : JSON.stringify(parsed, null, 2);
          }

          if (isJson && kind != "dependencies") {
            try {
              const parsed = JSON.parse(content);
              if (stripOnBehalfOf) {
                const isSchedule = p.endsWith(".schedule.json");
                const isTrigger = p.endsWith("_trigger.json");
                // Only emit the flag when set; a `false` here is the default and
                // would produce a spurious diff for every ownerless schedule/trigger.
                if (isSchedule) {
                  if (parsed["permissioned_as"]) {
                    parsed["has_permissioned_as"] = true;
                  }
                  delete parsed["permissioned_as"];
                  delete parsed["email"];
                  delete parsed["edited_by"];
                } else if (isTrigger) {
                  if (parsed["permissioned_as"]) {
                    parsed["has_permissioned_as"] = true;
                  }
                  delete parsed["permissioned_as"];
                  delete parsed["edited_by"];
                }
              }
              return useYaml
                ? yamlStringify(parsed, yamlOptions)
                : JSON.stringify(parsed, null, 2);
            } catch (error) {
              log.error(`Failed to parse JSON content at path: ${p}`);
              throw error;
            }
          }
          return content;
        },
      },
    ];
    if (kind == "script") {
      const content = await f.async("text");
      let parsed;
      try {
        parsed = JSON.parse(content);
      } catch (error) {
        log.error(`Failed to parse script lock content at path: ${p}`);
        throw error;
      }
      const lock = parsed["lock"];
      const scriptModules: Record<string, ScriptModule> | undefined = parsed["modules"];
      const hasModules = scriptModules && Object.keys(scriptModules).length > 0;

      // Compute base path and module folder
      const metaExt = useYaml ? ".yaml" : ".json";
      const scriptBasePath = removeSuffix(
        removeSuffix(finalPath, metaExt),
        ".script"
      );
      const moduleFolderPath = scriptBasePath + getModuleFolderSuffix();

      if (hasModules) {
        // Redirect metadata into __mod/script.yaml
        r[0].path = path.join(moduleFolderPath, "script" + metaExt);
      }

      if (lock && lock != "") {
        r.push({
          isDirectory: false,
          path: hasModules
            ? path.join(moduleFolderPath, "script.lock")
            : removeSuffix(finalPath, metaExt) + ".lock",
          async *getChildren() {},
          async getContentText() {
            return lock;
          },
        });
      }

      // Extract script modules into __mod/ folder
      if (hasModules) {
        r.push({
          isDirectory: true,
          path: moduleFolderPath,
          async *getChildren() {
            for (const [relPath, mod] of Object.entries(scriptModules!)) {
              // Yield the module content file
              yield {
                isDirectory: false,
                path: path.join(moduleFolderPath, relPath),
                async *getChildren() {},
                async getContentText() {
                  return mod.content;
                },
              };

              // Yield the module lock file if present
              if (mod.lock) {
                const baseName = relPath.replace(/\.[^.]+$/, '');
                yield {
                  isDirectory: false,
                  path: path.join(moduleFolderPath, baseName + ".lock"),
                  async *getChildren() {},
                  async getContentText() {
                    return mod.lock!;
                  },
                };
              }
            }
          },
          async getContentText() {
            throw new Error("Cannot get content of directory");
          },
        });
      }
    }
    if (kind == "resource") {
      const content = await f.async("text");
      let parsed;
      try {
        parsed = JSON.parse(content);
      } catch (error) {
        log.error(`Failed to parse resource file content at path: ${p}`);
        throw error;
      }
      const resourceType = parsed["resource_type"];
      const formatExtension =
        resourceTypeToFormatExtension[resourceType];
      const isFileset = resourceTypeToIsFileset[resourceType] ?? false;

      if (isFileset && typeof parsed["value"] === "object" && parsed["value"] !== null) {
        const filesetBasePath =
          removeSuffix(finalPath, ".resource.json") + ".fileset";
        // Push directory entry for the fileset
        r.push({
          isDirectory: true,
          path: filesetBasePath,
          async *getChildren() {
            for (const [relPath, fileContent] of Object.entries(parsed["value"])) {
              if (typeof fileContent === "string") {
                yield {
                  isDirectory: false,
                  path: path.join(filesetBasePath, relPath),
                  async *getChildren() {},
                  async getContentText() {
                    return fileContent;
                  },
                };
              }
            }
          },
          async getContentText() {
            throw new Error("Cannot get content of directory");
          },
        });
      } else if (formatExtension) {
        const fileContent: string = parsed["value"]["content"];
        if (typeof fileContent === "string") {
          r.push({
            isDirectory: false,
            path:
              removeSuffix(finalPath, ".resource.json") +
              ".resource.file." +
              formatExtension,
            async *getChildren() {},
            async getContentText() {
              return fileContent;
            },
          });
        }
      }
    }
    return r;
  }
  function _internal_folder(p: string, zip: JSZip): DynFSElement {
    return {
      isDirectory: true,
      path: p,
      async *getChildren(): AsyncIterable<DynFSElement> {
        for (const filename in zip.files) {
          const file = zip.files[filename];
          const totalPath = path.join(p, filename);
          if (file.dir) {
            const e = zip.folder(file.name)!;
            yield _internal_folder(totalPath, e);
          } else {
            const fs = await _internal_file(totalPath, file);
            for (const f of fs) {
              yield f;
            }
          }
        }
      },
      async getContentText(): Promise<string> {
        throw new Error("Cannot get content of folder");
      },
    };
  }
  return _internal_folder("." + SEP, zip);
}

export async function* readDirRecursiveWithIgnore(
  ignore: (path: string, isDirectory: boolean) => boolean,
  root: DynFSElement,
): AsyncGenerator<{
  path: string;
  ignored: boolean;
  isDirectory: boolean;
  // getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
}> {
  const stack: {
    path: string;
    isDirectory: boolean;
    ignored: boolean;
    c(): AsyncIterable<DynFSElement>;
    // getContentBytes(): Promise<Uint8Array>;
    getContentText(): Promise<string>;
  }[] = [
    {
      path: root.path,
      ignored: ignore(root.path, root.isDirectory),
      isDirectory: root.isDirectory,
      c: root.getChildren,
      // getContentBytes(): Promise<Uint8Array> {
      //   throw undefined;
      // },
      getContentText(): Promise<string> {
        throw undefined;
      },
    },
  ];

  while (stack.length > 0) {
    const e = stack.pop()!;
    yield e;
    for await (const e2 of e.c()) {
      if (e2.isDirectory) {
        const dirName = e2.path.split(SEP).pop();
        if (
          dirName == "node_modules" ||
          dirName == ".claude" ||
          dirName?.startsWith(".")
        ) {
          continue;
        }
      }
      stack.push({
        path: e2.path,
        ignored: e.ignored || ignore(e2.path, e2.isDirectory),
        isDirectory: e2.isDirectory,
        // getContentBytes: e2.getContentBytes,
        getContentText: e2.getContentText,
        c: e2.getChildren,
      });
    }
  }
}

type Added = { name: "added"; path: string; content: string };
type Deleted = { name: "deleted"; path: string };
type Edit = {
  name: "edited";
  path: string;
  before: string;
  after: string;
  codebase?: string;
};
// Flag-only change: the file content matches the server but the local
// specificItems config flags the item as ws_specific while the server
// does not (or vice-versa). Emitted only by computeWsSpecificFlagOnlyPushes
// and routed through its own apply branch — do NOT model this as
// Edit{before === after}, because any future "skip identical edits"
// optimization would silently drop these.
type WsSpecificFlag = {
  name: "ws_specific_flag";
  path: string;
  kind: string;
  wsSpecific: boolean;
};

type Change = Added | Deleted | Edit | WsSpecificFlag;

export async function elementsToMap(
  els: DynFSElement,
  ignore: (path: string, isDirectory: boolean) => boolean,
  json: boolean,
  skips: Skips,
  specificItems?: SpecificItemsConfig,
  branchOverride?: string,
  isRemote?: boolean,
): Promise<{ [key: string]: string }> {
  const map: { [key: string]: string } = {};
  const processedBasePaths = new Set<string>();
  const wrongFormatPaths: string[] = [];
  // Cache git branch at the start to avoid repeated execSync calls per file
  const cachedWsName = branchOverride ?? getCurrentGitBranch() ?? undefined;
  for await (const entry of readDirRecursiveWithIgnore(ignore, els)) {
    // console.log("FOO", entry.path, entry.ignored, entry.isDirectory)
    if (entry.isDirectory) {
      // Check for folder suffix format mismatch (only for local paths)
      if (!isRemote) {
        const dirName = entry.path.split(SEP).pop() ?? "";
        if (hasWrongFormatSuffix(dirName)) {
          wrongFormatPaths.push(entry.path);
        }
      }
      continue;
    }
    if (entry.ignored) {
      continue;
    }
    const path = entry.path;
    // Include module files in the map so they're compared for changes,
    // but they're pushed as part of their parent script via handleFile
    if (isScriptModulePath(path)) {
      map[path] = await entry.getContentText();
      continue;
    }
    if (
      !isFileResource(path) &&
      !isFilesetResource(path) &&
      !isRawAppFile(path) &&
      !isWorkspaceDependencies(path)
    ) {
      if (json && path.endsWith(".yaml")) continue;
      if (!json && path.endsWith(".json")) continue;

      if (
        ![
          "json",
          "yaml",
          "go",
          "sh",
          "ts",
          "py",
          "sql",
          "gql",
          "ps1",
          "php",
          "js",
          "lock",
          "rs",
          "cs",
          "yml",
          "nu",
          "java",
          "rb",
          "r",
          // for related places search: ADD_NEW_LANG
        ].includes(path.split(".").pop() ?? "")
      ) {
        continue;
      }
    }

    if (isRawAppFile(path)) {
      const suffix = path.split(getFolderSuffix("raw_app") + SEP).pop();
      if (
        suffix?.startsWith("dist/") ||
        suffix == "wmill.d.ts" ||
        suffix == "package-lock.json" ||
        suffix == "DATATABLES.md"
      ) {
        continue;
      }
    }

    if (skips.skipResources && (isFileResource(path) || isFilesetResource(path))) continue;

    const ext = json ? ".json" : ".yaml";
    if (!skips.includeSchedules && path.endsWith(".schedule" + ext)) continue;
    if (
      !skips.includeTriggers &&
      (path.endsWith(".http_trigger" + ext) ||
        path.endsWith(".websocket_trigger" + ext) ||
        path.endsWith(".kafka_trigger" + ext) ||
        path.endsWith(".nats_trigger" + ext) ||
        path.endsWith(".postgres_trigger" + ext) ||
        path.endsWith(".mqtt_trigger" + ext) ||
        path.endsWith(".amqp_trigger" + ext) ||
        path.endsWith(".sqs_trigger" + ext) ||
        path.endsWith(".gcp_trigger" + ext) ||
        path.endsWith(".azure_trigger" + ext) ||
        path.endsWith(".email_trigger" + ext) ||
        path.endsWith("_native_trigger" + ext))
    ) {
      continue;
    }
    if (!skips.includeUsers && path.endsWith(".user" + ext)) continue;
    if (!skips.includeGroups && path.endsWith(".group" + ext)) continue;
    if (!skips.includeSettings && path === "settings" + ext) continue;
    if (!skips.includeKey && path === "encryption_key") continue;
    if (skips.skipResources && path.endsWith(".resource" + ext)) continue;
    if (skips.skipResourceTypes && path.endsWith(".resource-type" + ext)) {
      continue;
    }

    // Use getTypeStrFromPath for consistent type detection
    try {
      const fileType = getTypeStrFromPath(path);
      if (skips.skipVariables && fileType === "variable") continue;
      if (skips.skipScripts && fileType === "script") continue;
      if (skips.skipFlows && fileType === "flow") continue;
      if (skips.skipApps && fileType === "app") continue;
      if (skips.skipFolders && fileType === "folder") continue;
      if (
        skips.skipWorkspaceDependencies &&
        fileType === "workspace_dependencies"
      )
        continue;
    } catch {
      // If getTypeStrFromPath can't determine the type, continue processing the file
    }

    // Handle workspace-specific files - skip files for other branches
    if (specificItems && isWorkspaceSpecificFile(path)) {
      if (!isCurrentWorkspaceFile(path, cachedWsName)) {
        // Skip workspace-specific files for other branches
        continue;
      }
    }

    const content = await entry.getContentText();

    if (skips.skipSecrets && path.endsWith(".variable" + ext)) {
      try {
        let o;
        if (json) {
          try {
            o = JSON.parse(content);
          } catch (error) {
            log.error(`Failed to parse JSON variable content at path: ${path}`);
            throw error;
          }
        } else {
          try {
            o = yamlParseContent(path, content);
          } catch (error) {
            log.error(`Failed to parse YAML variable content at path: ${path}`);
            throw error;
          }
        }
        if (o["is_secret"]) {
          continue;
        }
      } catch (e) {
        log.warn(`Error reading variable ${path} to check for secrets`);
      }
    }

    // Handle workspace-specific path mapping after all filtering
    if (cachedWsName && isCurrentWorkspaceFile(path, cachedWsName)) {
      // This is a workspace-specific file for current branch
      const currentBranch = cachedWsName;
      const basePath = fromWorkspaceSpecificPath(path, currentBranch);

      // Only use workspace-specific files if the item type IS configured as branch-specific
      // AND matches the pattern. Otherwise, skip and use base file instead.
      if (!isItemTypeConfigured(basePath, specificItems)) {
        // Type not configured as branch-specific - skip, use base file instead
        continue;
      }
      if (!isSpecificItem(basePath, specificItems)) {
        // Type configured but doesn't match pattern - skip
        continue;
      }

      // Type configured AND matches - map to base path
      map[basePath] = content;
      processedBasePaths.add(basePath);
    } else if (!isWorkspaceSpecificFile(path)) {
      // This is a regular base file
      if (processedBasePaths.has(path)) {
        // Skip base file, we already processed workspace-specific version
        continue;
      }
      // Skip base file if it's configured as branch-specific (expect branch version)
      // Only for LOCAL files - remote workspace only has base paths
      if (!isRemote && isSpecificItem(path, specificItems)) {
        continue;
      }
      map[path] = content;
    }
    // Note: workspace-specific files for other branches are already filtered out earlier
  }

  if (wrongFormatPaths.length > 0) {
    const isNonDotted = getNonDottedPaths();
    const foundFormat = isNonDotted ? ".flow/.app/.raw_app" : "__flow/__app/__raw_app";
    const expectedFormat = isNonDotted ? "__flow/__app/__raw_app" : ".flow/.app/.raw_app";
    const configHint = isNonDotted
      ? "Either remove 'nonDottedPaths: true' from wmill.yaml, or rename these directories to use __flow/__app/__raw_app format."
      : "Either add 'nonDottedPaths: true' to wmill.yaml, or rename these directories to use .flow/.app/.raw_app format.";
    const pathList = wrongFormatPaths.map((p) => `  ${p}`).join("\n");
    throw new Error(
      `Found ${wrongFormatPaths.length} directory(ies) using ${foundFormat} format, but wmill.yaml expects ${expectedFormat}:\n${pathList}\n${configHint}`
    );
  }

  return map;
}

export interface Skips {
  skipVariables?: boolean | undefined;
  skipResources?: boolean | undefined;
  skipResourceTypes?: boolean | undefined;
  skipSecrets?: boolean | undefined;
  skipScripts?: boolean | undefined;
  skipFlows?: boolean | undefined;
  skipApps?: boolean | undefined;
  skipFolders?: boolean | undefined;
  skipWorkspaceDependencies?: boolean | undefined;
  skipScriptsMetadata?: boolean | undefined;
  includeSchedules?: boolean | undefined;
  includeTriggers?: boolean | undefined;
  includeUsers?: boolean | undefined;
  includeGroups?: boolean | undefined;
  includeSettings?: boolean | undefined;
  includeKey?: boolean | undefined;
}

// Detect paths (and their parent directories) within a single set that differ
// only by letter case — e.g. a remote workspace that genuinely holds both
// f/Caps/a and f/caps/b. On a case-insensitive filesystem (Windows, default
// macOS) these cannot be represented as two distinct files/directories at all,
// so we can only warn. Returns one group per collision, each listing the
// distinct casings sorted for stable output.
//
// Only the shallowest clash is reported: when two case-variant folders also
// contain same-named files (f/Caps/main.ts + f/caps/main.ts), the folder clash
// is the root cause, so the nested per-file group is suppressed rather than
// inflating the count with one entry per duplicated leaf.
export function findCaseInsensitiveCollisions(
  paths: Iterable<string>,
): string[][] {
  // lowercased prefix -> set of distinct original casings observed
  const byLower = new Map<string, Set<string>>();
  for (const full of paths) {
    // Compare on normalized forward-slash prefixes so a Windows-style "\" map
    // key and a remote "/" key collapse to the same prefix.
    const segs = full.split(/[\\/]/).filter((s) => s.length > 0);
    let acc = "";
    for (let i = 0; i < segs.length; i++) {
      // Accumulate every directory prefix as well as the full file path, so a
      // "f/Caps" vs "f/caps" folder clash is caught even when the leaf files
      // (e.g. a.ts vs b.ts) don't themselves collide.
      acc = i === 0 ? segs[i] : `${acc}/${segs[i]}`;
      const lower = acc.toLowerCase();
      let set = byLower.get(lower);
      if (!set) {
        set = new Set();
        byLower.set(lower, set);
      }
      set.add(acc);
    }
  }
  const collidingLowers = new Set<string>();
  for (const [lower, set] of byLower) {
    if (set.size > 1) collidingLowers.add(lower);
  }
  const collisions: string[][] = [];
  for (const lower of collidingLowers) {
    // Drop this group if any ancestor prefix is itself a collision — the
    // shallower folder clash already names the root cause.
    const parts = lower.split("/");
    let hasCollidingAncestor = false;
    for (let i = 1; i < parts.length; i++) {
      if (collidingLowers.has(parts.slice(0, i).join("/"))) {
        hasCollidingAncestor = true;
        break;
      }
    }
    if (!hasCollidingAncestor) {
      collisions.push([...byLower.get(lower)!].sort());
    }
  }
  return collisions;
}

type CaseTrieNode = {
  // lowercased segment -> child, recording the canonical (server) casing and
  // whether the server holds more than one casing of that segment (ambiguous).
  children: Map<
    string,
    { canonical: string; ambiguous: boolean; node: CaseTrieNode }
  >;
};

// Rewrite `localMap` keys to the canonical casing recorded on the server
// (`remoteMap`) when they differ only by letter case. This is the core
// WIN-2020 fix: on a case-insensitive filesystem a folder such as `f/Caps`
// can have its on-disk casing silently drift (e.g. to `f/caps`) — Windows
// stores whatever case the directory was first created with and reports that
// from readdir, regardless of the server's path. Without this, the diff sees
// the drifted local path as an entirely different item and emits a destructive
// "delete f/Caps + add f/caps" pair, so a single capitalized folder appears to
// vanish and a lowercase clone shows up out of nowhere. Adopting the server
// casing collapses that phantom and leaves the canonical path on the server
// untouched.
//
// Canonicalization is segment-by-segment against a trie of remote paths, so it
// also applies the server's folder casing to brand-new local files that have no
// exact remote match (e.g. adding f/caps/New.ts under a drifted f/Caps folder
// becomes f/Caps/New.ts) — otherwise the push would recreate the case-only
// collision the fix is meant to prevent. A segment is only adopted when the
// server casing is unambiguous; at the first ambiguous or unknown segment the
// remainder of the path keeps its local casing.
//
// Returns the rewritten map, the per-key rewrites, and any genuinely ambiguous
// server-side groups (two distinct remote paths differing only by case) — those
// can't be canonicalized to a single target and are left for the caller to warn
// about.
export function canonicalizeCaseInsensitiveKeys(
  localMap: Record<string, string>,
  remoteMap: Record<string, string>,
): {
  map: Record<string, string>;
  ambiguous: string[][];
  rewritten: { from: string; to: string }[];
} {
  const root: CaseTrieNode = { children: new Map() };
  for (const k of Object.keys(remoteMap)) {
    let node = root;
    for (const seg of k.split(/[\\/]/)) {
      if (seg.length === 0) continue;
      const lk = seg.toLowerCase();
      let entry = node.children.get(lk);
      if (!entry) {
        entry = { canonical: seg, ambiguous: false, node: { children: new Map() } };
        node.children.set(lk, entry);
      } else if (entry.canonical !== seg) {
        entry.ambiguous = true;
      }
      node = entry.node;
    }
  }

  const out: Record<string, string> = {};
  const rewritten: { from: string; to: string }[] = [];
  for (const [k, v] of Object.entries(localMap)) {
    // Preserve the key's own separator style so the rewritten key still matches
    // the rest of the map (and round-trips through push) on every platform.
    const sep = k.includes("\\") ? "\\" : "/";
    const segs = k.split(/[\\/]/);
    const canonSegs: string[] = [];
    let node: CaseTrieNode | undefined = root;
    let changed = false;
    for (const seg of segs) {
      if (seg.length === 0) {
        canonSegs.push(seg);
        continue;
      }
      const entry = node?.children.get(seg.toLowerCase());
      if (entry && !entry.ambiguous) {
        if (entry.canonical !== seg) changed = true;
        canonSegs.push(entry.canonical);
        node = entry.node;
      } else {
        // No unambiguous server guidance for this segment: keep the local
        // casing here and below (deeper server structure is unknown).
        canonSegs.push(seg);
        node = undefined;
      }
    }
    const canonKey = canonSegs.join(sep);
    if (changed && canonKey !== k) {
      out[canonKey] = v;
      rewritten.push({ from: k, to: canonKey });
    } else {
      out[k] = v;
    }
  }

  return {
    map: out,
    ambiguous: findCaseInsensitiveCollisions(Object.keys(remoteMap)),
    rewritten,
  };
}

// Summarize case-only key rewrites by their differing path prefix (typically a
// folder such as f/caps -> f/Caps) so a folder whose casing drifted is reported
// once instead of once per contained file.
export function summarizeCaseRewrites(
  rewritten: { from: string; to: string }[],
): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const { from, to } of rewritten) {
    const fromSegs = from.split(/[\\/]/);
    const toSegs = to.split(/[\\/]/);
    // Find the shortest prefix at which the two casings first differ; that is
    // the folder (or file) whose casing actually changed.
    let i = 0;
    while (
      i < fromSegs.length &&
      i < toSegs.length &&
      fromSegs[i] === toSegs[i]
    ) {
      i++;
    }
    const fromPrefix = fromSegs.slice(0, i + 1).join("/");
    const toPrefix = toSegs.slice(0, i + 1).join("/");
    const key = `${fromPrefix} -> ${toPrefix}`;
    if (!seen.has(key)) {
      seen.add(key);
      out.push(key);
    }
  }
  return out;
}

// Emit a single grouped warning for case-only collisions that cannot be
// represented on a case-insensitive filesystem (two distinct server paths
// differing only by case). Unlike the drift handled by
// canonicalizeCaseInsensitiveKeys, these require the user to rename one side.
function warnUnrepresentableCaseCollisions(collisions: string[][]): void {
  if (collisions.length === 0) return;
  const groups = collisions.map((g) => `  - ${g.join("  <->  ")}`).join("\n");
  log.warn(
    `Found ${collisions.length} path(s) that differ only by letter case:\n` +
      `${groups}\n` +
      `On case-insensitive filesystems (Windows, default macOS) these collapse ` +
      `into a single file/directory and cannot both be synced. Rename one side ` +
      `to a distinct path to make the tree sync reliably across platforms.`,
  );
}

// Probe (and cache) whether `dir` lives on a case-insensitive filesystem.
// Auto-detected by round-tripping a probe file under two casings, with an
// explicit WMILL_CASE_INSENSITIVE_FS=true/false override so Windows behaviour
// can be forced (or emulated for cross-platform repos / tests) on any host.
let _caseInsensitiveFsCache: boolean | undefined;
export async function isCaseInsensitiveFilesystem(
  dir: string,
): Promise<boolean> {
  const override = (process.env.WMILL_CASE_INSENSITIVE_FS ?? "")
    .trim()
    .toLowerCase();
  if (override === "true" || override === "1") return true;
  if (override === "false" || override === "0") return false;
  if (_caseInsensitiveFsCache !== undefined) return _caseInsensitiveFsCache;
  let result = false;
  try {
    const upper = path.join(dir, `.wmill-CASEPROBE-${process.pid}.tmp`);
    const lower = path.join(dir, `.wmill-caseprobe-${process.pid}.tmp`);
    await writeFile(upper, "", "utf-8");
    try {
      await stat(lower);
      result = true; // lowercase name resolves to the file we wrote uppercase
    } catch {
      result = false;
    }
    await rm(upper).catch(() => {});
    await rm(lower).catch(() => {});
  } catch {
    result = false;
  }
  _caseInsensitiveFsCache = result;
  return result;
}

// A script's `lock` is NULL on the remote only while the server is still
// (re)generating it — e.g. an importer relock is in flight after a dependent
// relative-import module changed, or the script's own first lock job has not
// settled yet. NULL means "lock pending", NOT "this script has no lock": a
// genuinely lock-free script (no dependencies, or a codebase script) serializes
// `lock: ''` (empty string), which keeps its metadata key. The git-sync deploy
// mirror reads the workspace inside that window and would otherwise mirror the
// transient NULL as a deletion of the committed `.script.lock` (and strip the
// `lock: '!inline …'` line from the metadata), corrupting the mirror until the
// relock writes the identical lock back seconds later (issue #9588).
//
// When pulling (remote -> local), if the remote reports a pending (NULL) lock
// for a script whose committed lock still exists locally, carry the local lock
// onto the remote map so the diff is a no-op for both the lock file and the
// metadata `lock` line. An empty-string lock ('') is left untouched, so a real
// "dependencies removed" transition still deletes the obsolete lock.
export function preservePendingScriptLocks(
  remote: Record<string, string>,
  local: Record<string, string>,
): void {
  // A multi-module script keeps its metadata in the folder layout
  // `…__mod/script.{yaml,json}` instead of `….script.{yaml,json}`.
  // Map keys are always forward-slash normalized, on every platform.
  const modMeta = getModuleFolderSuffix() + "/script";
  for (const metaKey of Object.keys(remote)) {
    const isYaml =
      metaKey.endsWith(".script.yaml") || metaKey.endsWith(modMeta + ".yaml");
    const isJson =
      metaKey.endsWith(".script.json") || metaKey.endsWith(modMeta + ".json");
    if (!isYaml && !isJson) continue;

    const localMeta = local[metaKey];
    if (localMeta === undefined) continue; // script not committed locally

    let remoteParsed: any;
    let localParsed: any;
    try {
      remoteParsed = isYaml
        ? yamlParseContent(metaKey, remote[metaKey])
        : JSON.parse(remote[metaKey]);
      localParsed = isYaml ? yamlParseContent(metaKey, localMeta) : JSON.parse(localMeta);
    } catch {
      continue;
    }
    if (typeof remoteParsed !== "object" || remoteParsed === null) continue;
    if (typeof localParsed !== "object" || localParsed === null) continue;

    // Only a NULL/absent remote lock is "pending". An empty-string lock ('') is
    // a real "no dependencies" state and must still propagate as a deletion.
    const remoteLock = remoteParsed["lock"];
    if (remoteLock !== undefined && remoteLock !== null) continue;

    // The local side must reference an inline lock backed by a committed file.
    const localLock = localParsed["lock"];
    if (typeof localLock !== "string" || !localLock.startsWith("!inline ")) continue;

    // Derive the lock-file key from the `!inline` reference itself, not from the
    // metadata path: a multi-module script keeps its lock at `…__mod/script.lock`,
    // which a `.script.yaml -> .script.lock` rewrite would miss. The reference and
    // the map keys are both forward-slash, so no separator rewrite is needed.
    const lockKey = localLock.slice("!inline ".length);
    if (local[lockKey] === undefined) continue; // committed lock already gone

    remoteParsed["lock"] = localLock;
    remote[metaKey] = isYaml
      ? yamlStringify(remoteParsed, yamlOptions)
      : JSON.stringify(remoteParsed, null, 2);
    remote[lockKey] = local[lockKey];
  }
}

async function compareDynFSElement(
  els1: DynFSElement,
  els2: DynFSElement | undefined,
  ignore: (path: string, isDirectory: boolean) => boolean,
  json: boolean,
  skips: Skips,
  ignoreMetadataDeletion: boolean,
  codebases: SyncCodebase[],
  ignoreCodebaseChanges: boolean,
  specificItems?: SpecificItemsConfig,
  branchOverride?: string,
  isEls1Remote?: boolean,
  caseInsensitiveFs?: boolean,
): Promise<{ changes: Change[]; localMap: Record<string, string> }> {
  let [m1, m2] = els2
    ? await Promise.all([
        elementsToMap(els1, ignore, json, skips, specificItems, branchOverride, isEls1Remote),
        elementsToMap(els2, ignore, json, skips, specificItems, branchOverride, !isEls1Remote),
      ])
    : [await elementsToMap(els1, ignore, json, skips, specificItems, branchOverride, isEls1Remote), {}];

  // Reconcile letter-case differences between the local tree and the
  // authoritative server casing. Only meaningful for an actual two-sided diff
  // (els2 defined) where we know which side is the remote.
  if (els2 && isEls1Remote !== undefined) {
    const remoteMap = isEls1Remote ? m1 : m2;

    // Always warn about server paths that differ only by case (e.g. f/Caps and
    // f/caps as two distinct items). These cannot coexist on a case-insensitive
    // filesystem, so flag them on every platform — a Linux author needs to know
    // their tree won't round-trip for a Windows/macOS teammate.
    warnUnrepresentableCaseCollisions(
      findCaseInsensitiveCollisions(Object.keys(remoteMap)),
    );

    // On a case-insensitive filesystem, the local on-disk casing of a folder
    // can drift from the server's (Windows reports the case the directory was
    // first created with). Rewrite those drifted local keys to the server
    // casing so the diff treats them as the same item instead of a destructive
    // delete+add pair. This is the WIN-2020 fix.
    if (caseInsensitiveFs) {
      const { map, rewritten } = canonicalizeCaseInsensitiveKeys(
        isEls1Remote ? m2 : m1,
        remoteMap,
      );
      if (isEls1Remote) {
        m2 = map;
      } else {
        m1 = map;
      }
      const summary = summarizeCaseRewrites(rewritten);
      if (summary.length > 0) {
        log.info(
          `Reconciled ${summary.length} local path(s) to the server's casing ` +
            `(case-insensitive filesystem):\n` +
            summary.map((s) => `  ${s}`).join("\n"),
        );
      }
    }
  }

  // Pull only (remote is els1): keep a committed `.script.lock` when the remote
  // lock is transiently NULL (a relock is mid-flight). See #9588.
  if (isEls1Remote === true) {
    preservePendingScriptLocks(m1, m2);
  }

  const changes: Change[] = [];

  function parseYaml(k: string, v: string) {
    if (k.endsWith(".script.yaml")) {
      let o: any;
      try {
        o = yamlParseContent(k, v);
      } catch (error) {
        log.error(`Failed to parse script YAML content at path: ${k}`);
        throw error;
      }
      if (typeof o == "object") {
        if (Array.isArray(o?.["lock"])) {
          o["lock"] = o["lock"].join("\n");
        }
        if (o["is_template"] != undefined) {
          delete o["is_template"];
        }
        // no_main_func is a legacy field — replaced by auto_kind and
        // auto-detected from script content at deploy time. auto_kind is
        // intentionally never serialized to disk. Strip both so pre-migration
        // local metadata does not produce phantom diffs.
        if (o["no_main_func"] != undefined) {
          delete o["no_main_func"];
        }
        if (o["auto_kind"] != undefined) {
          delete o["auto_kind"];
        }
      }
      return o;
    } else {
      try {
        return yamlParseContent(k, v);
      } catch (error) {
        log.error(`Failed to parse YAML content at path: ${k}`);
        throw error;
      }
    }
  }

  const codebaseChanges: Record<string, string> = {};

  for (let [k, v] of Object.entries(m1)) {
    const isScriptMetadata =
      k.endsWith(".script.yaml") || k.endsWith(".script.json");
    const skipMetadata = skips.skipScriptsMetadata && isScriptMetadata;

    if (m2[k] === undefined) {
      if (skipMetadata) {
        continue;
      }
      if (k.startsWith("dependencies/")) {
        if (!workspaceDependenciesPathToLanguageAndFilename(k)) {
          log.warn(`Skipping unrecognized workspace dependencies file: ${k}`);
          continue;
        }
        log.info(`Adding workspace dependencies file: ${k}`);
      }
      changes.push({ name: "added", path: k, content: v });
    } else {
      if (m2[k] == v) {
        continue;
      } else if (k.endsWith(".json") && !isWorkspaceDependencies(k)) {
        let parsedV, parsedM2;
        try {
          parsedV = JSON.parse(v);
        } catch (error) {
          log.error(
            `Failed to parse new JSON content for comparison at path: ${k}`,
          );
          throw error;
        }
        try {
          parsedM2 = JSON.parse(m2[k]);
        } catch (error) {
          log.error(
            `Failed to parse existing JSON content for comparison at path: ${k}`,
          );
          throw error;
        }
        if (deepEqual(parsedV, parsedM2)) {
          continue;
        }
      } else if (k.endsWith(".yaml")) {
        const before = parseYaml(k, m2[k]);
        const after = parseYaml(k, v);
        if (deepEqual(before, after)) {
          continue;
        }
        if (!ignoreCodebaseChanges) {
          const beforeCodebase = before?.codebase;
          const afterCodebase = after?.codebase;
          if (before?.codebase != undefined) {
            delete before.codebase;
            m2[k] = yamlStringify(before, yamlOptions);
          }
          if (after?.codebase != undefined) {
            delete after.codebase;
            v = yamlStringify(after, yamlOptions);
          }
          if (beforeCodebase != afterCodebase) {
            codebaseChanges[k] = afterCodebase ?? beforeCodebase ?? "";
          }
        }
        if (skipMetadata) {
          continue;
        }
      }
      changes.push({
        name: "edited",
        path: k,
        after: v,
        before: m2[k],
        codebase: codebaseChanges[k],
      });
    }
  }

  const remoteCodebase: Record<string, string> = {};
  for (const [k] of Object.entries(m2)) {
    if (m1[k] === undefined) {
      if (
        !ignoreMetadataDeletion ||
        (!k?.endsWith(".script.yaml") && !k?.endsWith(".script.json"))
      ) {
        changes.push({ name: "deleted", path: k });
      } else if (k?.endsWith(".script.yaml")) {
        const o = parseYaml(k, m2[k]);
        if (o.codebase != undefined) {
          remoteCodebase[k] = o.codebase;
        }
      }
    }
  }

  if (!ignoreCodebaseChanges) {
    for (const [k, v] of Object.entries(remoteCodebase)) {
      const tsFile = k.replace(".script.yaml", ".ts");
      if (
        changes.find(
          (c) =>
            c.path == tsFile && (c.name == "edited" || c.name == "deleted"),
        )
      ) {
        continue;
      }
      const c = findCodebase(tsFile, codebases);
      if ((await c?.getDigest()) != v) {
        changes.push({
          name: "edited",
          path: tsFile,
          codebase: v,
          before: m1[tsFile],
          after: m2[tsFile],
        });
      }
    }
  }

  if (!ignoreCodebaseChanges) {
    for (const change of changes) {
      const codebase = codebaseChanges[change.path];
      if (!codebase) continue;

      const tsFile = change.path.replace(".script.yaml", ".ts");
      if (change.name == "edited" && change.path == tsFile) {
        change.codebase = codebase;
      }
    }
  }

  changes.sort((a, b) => {
    const orderA = getOrderFromPath(a.path);
    const orderB = getOrderFromPath(b.path);
    if (orderA !== orderB) {
      return orderA - orderB;
    }
    // Within the same entity type, process deletes before adds/edits
    // to avoid conflicts (e.g. unique path constraints on triggers)
    const deletePriority = (name: string) => (name === "deleted" ? 0 : 1);
    const dp = deletePriority(a.name) - deletePriority(b.name);
    if (dp !== 0) {
      return dp;
    }
    return a.path.localeCompare(b.path);
  });

  // Expose the local-side map so callers (e.g. sync pull's auto-fill) can
  // reuse it instead of re-walking the filesystem. Which map is local depends
  // on which side `els1` was — pull passes remote as els1 (isEls1Remote=true),
  // push passes local as els1 (isEls1Remote=false).
  const localMap = isEls1Remote ? m2 : m1;
  return { changes, localMap };
}

function getOrderFromPath(p: string) {
  const typ = getTypeStrFromPath(p);
  // Order by dependencies: items that others depend on should be pushed first
  if (typ == "settings") {
    return 0;
  } else if (typ == "encryption_key") {
    return 1;
  } else if (typ == "user") {
    return 2;
  } else if (typ == "group") {
    return 3;
  } else if (typ == "folder") {
    return 4;
  } else if (typ == "resource-type") {
    return 5;
  } else if (typ == "variable") {
    return 6;
  } else if (typ == "resource") {
    return 7;
  } else if (typ == "workspace_dependencies") {
    return 8;
  } else if (typ == "script") {
    return 9;
  } else if (typ == "flow") {
    return 10;
  } else if (typ == "raw_app") {
    return 11;
  } else if (typ == "app") {
    return 12;
  } else if (typ == "schedule") {
    return 13;
  } else if (
    typ == "http_trigger" ||
    typ == "websocket_trigger" ||
    typ == "kafka_trigger" ||
    typ == "nats_trigger" ||
    typ == "postgres_trigger" ||
    typ == "mqtt_trigger" ||
    typ == "amqp_trigger" ||
    typ == "sqs_trigger" ||
    typ == "gcp_trigger" ||
    typ == "azure_trigger" ||
    typ == "email_trigger" ||
    typ == "native_trigger"
  ) {
    return 14;
  } else {
    return 15;
  }
}

const isNotWmillFile = (p: string, isDirectory: boolean) => {
  if (p.endsWith(SEP)) {
    return false;
  }
  // The `ui/` folder is workspace-shared frontend components for raw apps.
  // It's pushed/pulled separately from the diff machinery (see pushSharedUi/pullSharedUi).
  if (p.startsWith("ui" + SEP)) {
    return true;
  }
  if (isDirectory) {
    return (
      !p.startsWith("u" + SEP) &&
      !p.startsWith("f" + SEP) &&
      !p.startsWith("g" + SEP) &&
      !p.startsWith("users" + SEP) &&
      !p.startsWith("groups" + SEP) &&
      !p.startsWith("dependencies" + SEP) &&
      !p.startsWith("migrations" + SEP)
    );
  }

  // Files inside __mod/ folders are script module files — always valid wmill files
  if (isScriptModulePath(p)) {
    return false;
  }

  try {
    const typ = getTypeStrFromPath(p);
    // Datatable migrations live under migrations/datatable/<datatable>/, outside
    // the u/f/g namespaces, but are valid wmill files.
    if (typ == "datatable_migration") {
      return false;
    }
    if (
      typ == "resource-type" ||
      typ == "settings" ||
      typ == "encryption_key"
    ) {
      return p.includes(SEP);
    } else {
      return (
        !p.startsWith("u" + SEP) &&
        !p.startsWith("f" + SEP) &&
        !p.startsWith("g" + SEP) &&
        !p.startsWith("users" + SEP) &&
        !p.startsWith("groups" + SEP) &&
        !p.startsWith("dependencies" + SEP)
      );
    }
  } catch {
    return true;
  }
};

export const isWhitelisted = (p: string) => {
  return (
    p == "." + SEP ||
    p == "" ||
    p == "u" ||
    p == "f" ||
    p == "g" ||
    p == "ui" ||
    p == "users" ||
    p == "groups" ||
    p == "dependencies" ||
    p == "migrations"
  );
};

export async function ignoreF(wmillconf: {
  includes?: string[];
  excludes?: string[];
  extraIncludes?: string[];
  skipResourceTypes?: boolean;
  skipWorkspaceDependencies?: boolean;
  json?: boolean;
  includeUsers?: boolean;
  includeGroups?: boolean;
  includeSettings?: boolean;
  includeKey?: boolean;
}): Promise<(p: string, isDirectory: boolean) => boolean> {
  let whitelist: { approve(file: string): boolean } | undefined = undefined;

  if (
    (Array.isArray(wmillconf?.includes) && wmillconf?.includes?.length > 0) ||
    (Array.isArray(wmillconf?.excludes) && wmillconf?.excludes?.length > 0)
  ) {
    whitelist = {
      approve(file: string): boolean {
        return (
          (!wmillconf.includes ||
            wmillconf.includes?.some((i) => minimatch(file, i))) &&
          (!wmillconf?.excludes ||
            wmillconf.excludes!.every((i) => !minimatch(file, i))) &&
          (!wmillconf.extraIncludes ||
            wmillconf.extraIncludes.length === 0 ||
            wmillconf.extraIncludes.some((i) => minimatch(file, i)))
        );
      },
    };
  }

  try {
    await stat(".wmillignore");
    throw Error(".wmillignore is not supported anymore, switch to wmill.yaml");
  } catch {
    //expected
  }

  // new Gitignore.default({ initialRules: ignoreContent.split("\n")}).ignoreContent).compile();

  return (p: string, isDirectory: boolean) => {
    const ext = wmillconf.json ? ".json" : ".yaml";
    if (!isDirectory && p.endsWith(".resource-type" + ext)) {
      return wmillconf.skipResourceTypes ?? false;
    }

    // Special files should bypass path-based filtering when their include flags are set
    if (!isDirectory) {
      try {
        const fileType = getTypeStrFromPath(p);
        if (wmillconf.includeUsers && fileType === "user") {
          return false; // Don't ignore, always include
        }
        if (wmillconf.includeGroups && fileType === "group") {
          return false; // Don't ignore, always include
        }
        if (wmillconf.includeSettings && fileType === "settings") {
          return false; // Don't ignore, always include
        }
        if (wmillconf.includeKey && fileType === "encryption_key") {
          return false; // Don't ignore, always include
        }
        if (
          !wmillconf.skipWorkspaceDependencies &&
          fileType === "workspace_dependencies"
        ) {
          return false; // Don't ignore workspace dependencies (they are always included unless explicitly skipped)
        }
      } catch {
        // If getTypeStrFromPath can't determine the type, fall through to normal logic
      }
    }

    return (
      !isWhitelisted(p) &&
      (isNotWmillFile(p, isDirectory) ||
        (!isDirectory && whitelist != undefined && !whitelist.approve(p)))
    );
  };
}

interface ChangeTracker {
  scripts: string[];
  flows: string[];
  apps: string[];
  rawApps: string[];
}

async function addToChangedIfNotExists(p: string, tracker: ChangeTracker) {
  // Datatable migration .sql files are not scripts; they're synced via the
  // dedicated datatable_migration handler in the push loop.
  if (isDatatableMigrationPath(p)) {
    return;
  }
  const isScript = exts.some((e) => p.endsWith(e)) && !isFileResource(p) && !isFilesetResource(p);
  if (isScript) {
    if (isFlowPath(p)) {
      const folder = extractFolderPath(p, "flow")!;
      if (!tracker.flows.includes(folder)) {
        tracker.flows.push(folder);
      }
    } else if (isAppPath(p)) {
      const folder = extractFolderPath(p, "app")!;
      if (!tracker.apps.includes(folder)) {
        tracker.apps.push(folder);
      }
    } else if (isRawAppPath(p)) {
      const folder = extractFolderPath(p, "raw_app")!;
      if (!tracker.rawApps.includes(folder)) {
        tracker.rawApps.push(folder);
      }
    } else if (isScriptModulePath(p)) {
      if (isModuleEntryPoint(p)) {
        // Entry point (e.g. __mod/script.ts) IS the parent script content file
        if (!tracker.scripts.includes(p)) {
          tracker.scripts.push(p);
        }
      } else {
        // Module file changed — find the parent script content file
        const moduleSuffix = getModuleFolderSuffix() + "/";
        const idx = p.indexOf(moduleSuffix);
        if (idx !== -1) {
          const scriptBasePath = p.substring(0, idx);
          // Try folder layout first: __mod/script.{ext}
          try {
            const contentPath = await findContentFile(scriptBasePath + getModuleFolderSuffix() + "/script.yaml");
            if (contentPath && !tracker.scripts.includes(contentPath)) {
              tracker.scripts.push(contentPath);
            }
          } catch {
            // Fall back to flat layout: scriptBasePath.script.yaml
            try {
              const contentPath = await findContentFile(scriptBasePath + ".script.yaml");
              if (contentPath && !tracker.scripts.includes(contentPath)) {
                tracker.scripts.push(contentPath);
              }
            } catch {
              // ignore — content file not found
            }
          }
        }
      }
    } else {
      if (!tracker.scripts.includes(p)) {
        tracker.scripts.push(p);
      }
    }
  } else if (p.endsWith(".script.yaml") || p.endsWith(".script.json")) {
    try {
      const contentPath = await findContentFile(p);
      if (!contentPath) return;
      if (tracker.scripts.includes(contentPath)) return;
      tracker.scripts.push(contentPath);
    } catch {
      // ignore
    }
  }
}

async function buildTracker(changes: Change[]) {
  const tracker: ChangeTracker = {
    scripts: [],
    flows: [],
    apps: [],
    rawApps: [],
  };
  for (const change of changes) {
    if (change.name == "added" || change.name == "edited") {
      await addToChangedIfNotExists(change.path, tracker);
    }
  }
  return tracker;
}

/**
 * When a module file changes, find and push the parent script.
 * The parent script's handleFile will read the __mod/ folder and include all modules.
 */
async function pushParentScriptForModule(
  modulePath: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  opts: (GlobalOptions & { defaultTs?: "bun" | "deno" } & Skips) | undefined,
  rawWorkspaceDependencies: Record<string, string>,
  codebases: SyncCodebase[],
): Promise<void> {
  const moduleSuffix = getModuleFolderSuffix() + "/";
  const idx = modulePath.indexOf(moduleSuffix);
  if (idx === -1) return;
  const scriptBasePath = modulePath.substring(0, idx);
  const moduleFolderPath = scriptBasePath + getModuleFolderSuffix();

  // Try folder layout first: look for script.{ext} inside __mod/
  try {
    const entryPoint = await findContentFile(moduleFolderPath + "/script.yaml");
    if (entryPoint) {
      await handleFile(
        entryPoint,
        workspace,
        alreadySynced,
        message,
        opts,
        rawWorkspaceDependencies,
        codebases,
      );
      return;
    }
  } catch {}

  // Fall back to flat layout: look for content file alongside __mod/
  try {
    const contentPath = await findContentFile(scriptBasePath + ".script.yaml");
    if (contentPath) {
      await handleFile(
        contentPath,
        workspace,
        alreadySynced,
        message,
        opts,
        rawWorkspaceDependencies,
        codebases,
      );
    }
  } catch {
    log.debug(`Could not find parent script for module: ${modulePath}`);
  }
}

export async function pull(
  opts: GlobalOptions &
    SyncOptions & {
      repository?: string;
      promotion?: string;
      branch?: string;
      useIndividualBranch?: boolean;
      groupByFolder?: boolean;
      gitDeployItems?: string;
      onlyCreateBranch?: boolean;
      parentWorkspaceId?: string;
      devWorkspaceLabel?: string;
      parentDevWorkspaceLabel?: string;
      gitCommitterEmail?: string;
      gitCommitterName?: string;
    },
) {
  if ((opts as any).jsonOutput) log.setSilent(true);
  const originalCliOpts = { ...opts };
  opts = await mergeConfigWithConfigFile(opts);

  // --include-secrets overrides skipSecrets from wmill.yaml
  if ((originalCliOpts as any).includeSecrets) {
    opts.skipSecrets = false;
  }

  // Resolve workspace name for config lookups.
  // --branch resolves git branch → workspace name (deprecated but still supported).
  // --workspace selects a workspace config entry by name when it matches one,
  // regardless of --base-url. If it doesn't match any entry it's treated as a
  // profile/credential selector only.
  const hasExplicitCredentials = !!opts.baseUrl;
  let wsNameForConfig: string | undefined;

  if (opts.branch && !hasExplicitCredentials && !branchDeprecationWarned) {
    log.warn("⚠️  --branch/--env is deprecated. Use --workspace instead.");
    branchDeprecationWarned = true;
  }

  wsNameForConfig = resolveWsNameForConfigFromFlags(opts);

  if (!opts.branch && opts.workspace && !hasExplicitCredentials) {
    // Warn if override doesn't match a config key, or mismatches the auto-detected branch
    warnWorkspaceOverride(opts, opts.workspace);
  }

  // Validate workspace configuration early. Skip when ANY explicit flag is set
  // (even a --workspace value that doesn't match a config key — the user opted
  // out of branch-based auto-detection).
  try {
    await validateBranchConfiguration(opts, wsNameForConfig ?? opts.workspace);
  } catch (error) {
    if (error instanceof Error && error.message.includes("overrides")) {
      log.error(error.message);
      process.exit(1);
    }
    throw error;
  }

  if (opts.stateful) {
    await mkdir(path.join(process.cwd(), ".wmill"), { recursive: true });
  }

  const workspace = await resolveWorkspace(opts, wsNameForConfig);
  await requireLogin(opts);

  // Git-sync deployment-callback mode: when invoked from the git-sync hub
  // script with --git-deploy-items, the CWD is an existing clone of the repo.
  // Switch to the dedicated wm_deploy/fork branch (when applicable) BEFORE any
  // files are written so the deploy lands on the right branch instead of the
  // protected base branch.
  if (opts.gitDeployItems !== undefined) {
    let deployItems: GitSyncDeployItem[];
    try {
      deployItems = JSON.parse(opts.gitDeployItems);
    } catch (e) {
      log.error(`Invalid --git-deploy-items JSON: ${e}`);
      process.exit(1);
    }
    const clonedBranchName = getCurrentGitBranch() ?? "main";

    // Throwaway forks force-disable use_individual_branch / group_by_folder
    // (1:1 with the hub script's inner()). A dev workspace is the exception: it
    // honors promotion mode and gets per-item wm_deploy/** branches. Dev
    // workspaces have a prefix-less id, so detect them via the environment label
    // the backend passes with the deploy.
    const targetIsFork = isForkWorkspace(
      workspace.workspaceId,
      opts.parentWorkspaceId,
    );
    const forceOffPromotion = targetIsFork && !opts.devWorkspaceLabel;
    const useIndividualBranch = forceOffPromotion
      ? false
      : !!opts.useIndividualBranch;
    const groupByFolder = forceOffPromotion ? false : !!opts.groupByFolder;

    // Fork-of-a-fork: when the parent workspace is itself a fork, root the new
    // branch on the parent's fork branch (the content this fork diverged from).
    // A dev-workspace parent has a prefix-less id the prefix check can't see, so
    // the backend passes its environment label; its branch is the label verbatim.
    const parentBranch = opts.parentDevWorkspaceLabel
      ? opts.parentDevWorkspaceLabel
      : opts.parentWorkspaceId && isForkWorkspace(opts.parentWorkspaceId)
        ? computeGitSyncDeployBranch({
            workspaceId: opts.parentWorkspaceId,
            items: deployItems,
            useIndividualBranch,
            groupByFolder,
            clonedBranchName,
          })
        : null;
    if (parentBranch && parentBranch !== clonedBranchName) {
      checkoutGitSyncDeployBranch(parentBranch);
    }

    const deployBranch = computeGitSyncDeployBranch({
      workspaceId: workspace.workspaceId,
      parentWorkspaceId: opts.parentWorkspaceId,
      devWorkspaceLabel: opts.devWorkspaceLabel,
      items: deployItems,
      useIndividualBranch,
      groupByFolder,
      clonedBranchName,
    });
    // A dev workspace whose environment-label branch equals the repository's
    // tracked branch would silently commit the fork's content straight to the
    // tracked branch. Refuse instead of deploying in place.
    if (targetIsFork && deployBranch && deployBranch === clonedBranchName) {
      log.error(
        `Fork branch '${deployBranch}' equals the checked-out branch '${clonedBranchName}'; refusing to deploy a fork directly to the tracked branch. Use a different dev workspace label or tracked branch.`,
      );
      process.exit(1);
    }
    if (deployBranch && deployBranch !== clonedBranchName) {
      checkoutGitSyncDeployBranch(deployBranch);
    }

    if (opts.onlyCreateBranch) {
      // Branch-only publish: there is no commit here, so the GPG-cache-warmth
      // invariant that motivated moving commit+push to the hub script (WIN-1974,
      // #9284) does not apply — a bare `git push` of the (empty) branch ref needs
      // no signing. The hub script only runs its in-process commit+push for the
      // non-onlyCreateBranch path (`if (!only_create_branch) git_push(...)`), so
      // the CLI MUST publish the fork branch here or it is never pushed at all.
      gitSyncDeployPush({
        items: deployItems,
        authorName: process.env["WM_USERNAME"] || "windmill",
        authorEmail: process.env["WM_EMAIL"] || "windmill@windmill.dev",
        committerName: opts.gitCommitterName,
        committerEmail: opts.gitCommitterEmail,
        onlyCreateBranch: true,
      });
      return;
    }
  }

  // If wsNameForConfig wasn't set from flags, infer from the resolved profile
  if (!wsNameForConfig) {
    wsNameForConfig = inferWsNameFromProfile(opts, workspace);
  }

  // Resolve effective sync options with workspace awareness
  const effectiveOpts = await resolveEffectiveSyncOptions(
    workspace,
    opts,
    opts.promotion,
    wsNameForConfig,
  );

  // Extract specific items configuration
  let specificItems = getSpecificItemsForCurrentBranch(opts, wsNameForConfig);

  // Compute the workspace name for file naming (default to workspaceId)
  let wsNameForFiles = wsNameForConfig ? resolveWsNameForFiles(opts, wsNameForConfig) : workspace.workspaceId;

  // Augment specificItems with server-side ws_specific entries
  const localSpecificItems = specificItems;
  const wsSpecificMerge = await mergeWsSpecificFromServer(workspace.workspaceId, specificItems);
  specificItems = wsSpecificMerge.merged;

  // Merge CLI flags with resolved settings (CLI flags take precedence only for explicit overrides)
  opts = mergeCliWithEffectiveOptions(originalCliOpts, effectiveOpts);

  const codebases = await listSyncCodebases(opts);

  log.info(
    colors.gray(
      "Computing the files to update locally to match remote (taking wmill.yaml into account)",
    ),
  );

  let resourceTypeToFormatExtension: Record<string, string> = {};
  let resourceTypeToIsFileset: Record<string, boolean> = {};
  try {
    const raw = (await wmill.fileResourceTypeToFileExtMap({
      workspace: workspace.workspaceId,
    })) as Record<string, string | FileResourceTypeInfo>;
    const parsed = parseFileResourceTypeMap(raw);
    resourceTypeToFormatExtension = parsed.formatExtMap;
    resourceTypeToIsFileset = parsed.filesetMap;
  } catch {
    // ignore
  }

  const zipFile = await downloadZip(
    workspace,
    opts.plainSecrets,
    opts.skipVariables,
    opts.skipResources,
    opts.skipResourceTypes,
    opts.skipSecrets,
    opts.includeSchedules,
    opts.includeTriggers,
    opts.includeUsers,
    opts.includeGroups,
    opts.includeSettings,
    opts.includeKey,
    opts.skipWorkspaceDependencies,
    opts.defaultTs,
  );

  const remote = ZipFSElement(
    zipFile!,
    !opts.json,
    opts.defaultTs ?? "bun",
    resourceTypeToFormatExtension,
    resourceTypeToIsFileset,
    true,
    parseSyncBehavior(opts.syncBehavior) >= 1,
  );

  const local = !opts.stateful
    ? await FSFSElement(process.cwd(), codebases, true)
    : await FSFSElement(path.join(process.cwd(), ".wmill"), [], true);

  const { changes, localMap } = await compareDynFSElement(
    remote,
    local,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    false,
    codebases,
    true,
    specificItems,
    wsNameForFiles,
    true, // els1 (remote) is the remote source
    await isCaseInsensitiveFilesystem(process.cwd()),
  );

  log.info(
    `remote (${workspace.name}) -> local: ${changes.length} changes to apply`,
  );

  // Warn about items the server marks ws_specific that aren't covered by
  // wmill.yaml's specificItems patterns — but only for items affected by
  // this pull (changes list), so unrelated server-flagged items don't
  // spam the log. The merge above already ensures correctness for the
  // pull itself; this is a heads-up that the user's config drifts from
  // the remote and a future push from another machine without that config
  // would push the item as non-ws_specific.
  if (wsSpecificMerge.serverItems && wsSpecificMerge.serverItems.length > 0) {
    const changedPaths = new Set(changes.map((c) => c.path));
    for (const item of wsSpecificMerge.serverItems) {
      const filePath = `${item.path}.${item.item_kind}.yaml`;
      if (!changedPaths.has(filePath)) continue;
      if (!isSpecificItem(filePath, localSpecificItems)) {
        log.warn(
          `${item.item_kind} ${item.path} is workspace-specific on the remote ` +
            `but not flagged in wmill.yaml's specificItems — consider adding it.`,
        );
      }
    }
  }

  // Handle JSON output for dry-run
  if (opts.dryRun && opts.jsonOutput) {
    const result = {
      success: true,
      changes: changes.map((change) => ({
        type: change.name,
        path: change.path,
        ...(change.name === "edited" && change.codebase
          ? { codebase_changed: true }
          : {}),
        ...(specificItems && isSpecificItem(change.path, specificItems)
          ? {
              workspace_specific: true,
              workspace_specific_path: getWorkspaceSpecificPath(
                change.path,
                specificItems,
                wsNameForFiles,
              ),
            }
          : {}),
      })),
      total: changes.length,
    };
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  if (changes.length > 0) {
    if (!opts.jsonOutput) {
      prettyChanges(changes, specificItems, wsNameForFiles);
    }
    if (opts.dryRun) {
      log.info(colors.gray(`Dry run complete.`));
      return;
    }
    if (
      !opts.yes &&
      !(await Confirm.prompt({
        message: `Do you want to apply these ${changes.length} changes to your local files?`,
        default: true,
      }))
    ) {
      return;
    }

    const conflicts = [];

    log.info(colors.gray(`Applying changes to files ...`));
    for await (const change of changes) {
      // Determine if this file should be written to a workspace-specific path
      let targetPath = change.path;
      if (specificItems && isSpecificItem(change.path, specificItems)) {
        const workspaceSpecificPath = getWorkspaceSpecificPath(
          change.path,
          specificItems,
          wsNameForFiles,
        );
        if (workspaceSpecificPath) {
          targetPath = workspaceSpecificPath;
        }
      }

      const target = path.join(process.cwd(), targetPath);
      const stateTarget = path.join(process.cwd(), ".wmill", targetPath);
      if (change.name === "edited") {
        if (opts.stateful) {
          try {
            const currentLocal = await readTextFile(target);
            if (
              currentLocal !== change.before &&
              currentLocal !== change.after
            ) {
              log.info(
                colors.red(
                  `Conflict detected on ${change.path}\nBoth local and remote have been modified.`,
                ),
              );
              if (opts.failConflicts) {
                conflicts.push({
                  local: currentLocal,
                  change,
                  path: change.path,
                });
                continue;
              } else if (opts.yes) {
                log.info(
                  colors.red(
                    `Override local version with remote since --yes was passed and no --fail-conflicts.`,
                  ),
                );
              } else {
                showConflict(change.path, currentLocal, change.after);
                if (
                  await Confirm.prompt(
                    "Preserve local (push to change remote and avoid seeing this again)?",
                  )
                ) {
                  continue;
                }
              }
            }
          } catch {
            // ignore
          }
        }
        if (exts.some((e) => change.path.endsWith(e))) {
          log.info(
            `Editing script content of ${targetPath}${
              targetPath !== change.path
                ? colors.gray(` (workspace-specific override for ${change.path})`)
                : ""
            }`,
          );
        } else if (
          change.path.endsWith(".yaml") ||
          change.path.endsWith(".json")
        ) {
          log.info(
            `Editing ${changeTypeLabel(change.path)}${targetPath}${
              targetPath !== change.path
                ? colors.gray(` (workspace-specific override for ${change.path})`)
                : ""
            }`,
          );
        }
        await writeFile(target, change.after, "utf-8");

        if (opts.stateful) {
          await mkdir(path.dirname(stateTarget), { recursive: true });
          await copyFile(target, stateTarget);
        }
      } else if (change.name === "added") {
        await mkdir(path.dirname(target), { recursive: true });
        if (opts.stateful) {
          await mkdir(path.dirname(stateTarget), { recursive: true });
          log.info(
            `Adding ${changeTypeLabel(change.path)}${targetPath}${
              targetPath !== change.path
                ? colors.gray(` (workspace-specific override for ${change.path})`)
                : ""
            }`,
          );
        }
        await writeFile(target, change.content, "utf-8");
        log.info(
          `Writing ${changeTypeLabel(change.path)}${targetPath}${
            targetPath !== change.path
              ? colors.gray(` (workspace-specific override for ${change.path})`)
              : ""
          }`,
        );
        if (opts.stateful) {
          await copyFile(target, stateTarget);
        }
      } else if (change.name === "deleted") {
        try {
          log.info(
            `Deleting ${changeTypeLabel(change.path)}${change.path}`,
          );
          await rm(target);
          if (opts.stateful) {
            await rm(stateTarget);
          }
        } catch {
          if (opts.stateful) {
            await rm(stateTarget);
          }
        }
      }
    }
    if (opts.failConflicts) {
      if (conflicts.length > 0) {
        console.error(colors.red(`Conflicts were found`));
        log.info("Conflicts:");
        for (const conflict of conflicts) {
          showConflict(conflict.path, conflict.local, conflict.change.after);
        }
        log.info(
          colors.red(`Please resolve these conflicts manually by either:
  - reverting the content back to its remote (\`wmill pull\` and refuse to preserve local when prompted)
  - pushing the changes with \`wmill push --skip-pull\` to override wmill with all your local changes
`),
        );
        process.exit(1);
      }
    }
    log.info("All local changes pulled, now updating wmill-lock.yaml");
    await readLockfile(); // ensure wmill-lock.yaml exists

    const tracker: ChangeTracker = await buildTracker(changes);
    const rawWorkspaceDependencies: Record<string, string> =
      await getRawWorkspaceDependencies(true);

    for (const change of tracker.scripts) {
      await generateScriptMetadataInternal(
        change,
        workspace,
        opts,
        false,
        true,
        rawWorkspaceDependencies,
        codebases,
        true,
      );
    }
    for (const change of tracker.flows) {
      log.info(`Updating lock for flow ${change}`);
      await generateFlowLockInternal(
        change,
        false,
        workspace,
        opts,
        true,
        false,
      );
    }

    if (tracker.apps.length > 0) {
      log.info(
        colors.gray(
          `Apps ${tracker.apps.join(
            ", ",
          )} inline scripts were changed but ignoring metadata regeneration for now`,
        ),
      );
    }
    for (const change of tracker.rawApps) {
      log.info(`Updating lock metadata for raw app ${change}`);
      await generateAppLocksInternal(
        change,
        true,
        false,
        workspace,
        opts,
        true,
        true,
      );
    }
    for (const change of tracker.apps) {
      log.info(`Updating lock metadata for app ${change}`);
      await generateAppLocksInternal(
        change,
        false,
        false,
        workspace,
        opts,
        true,
        true,
      );
    }

    if (opts.jsonOutput) {
      const result = {
        success: true,
        message: `All ${changes.length} changes applied locally and wmill-lock.yaml updated`,
        changes: changes.map((change) => ({
          type: change.name,
          path: change.path,
          ...(change.name === "edited" && change.codebase
            ? { codebase_changed: true }
            : {}),
          ...(specificItems && isSpecificItem(change.path, specificItems)
            ? {
                workspace_specific: true,
                workspace_specific_path: getWorkspaceSpecificPath(
                  change.path,
                  specificItems,
                  wsNameForFiles,
                ),
              }
            : {}),
        })),
        total: changes.length,
      };
      console.log(JSON.stringify(result, null, 2));
    } else {
      log.info(
        colors.bold.green.underline(
          `\nDone! All ${changes.length} changes applied locally and wmill-lock.yaml updated.`,
        ),
      );
    }
  } else if (opts.jsonOutput) {
    console.log(
      JSON.stringify(
        { success: true, message: "No changes to apply", total: 0 },
        null,
        2,
      ),
    );
  }

  // Auto-fill missing lockfile entries for items that exist on disk but have
  // no entry yet (e.g. flows/apps that predate lockfile maintenance). Runs
  // regardless of whether the pull had changes from the backend so a no-op
  // pull still bootstraps the lockfile. Silent on a complete lockfile (just
  // dict lookups, no hashing). Skipped under --dry-run since auto-fill writes
  // to wmill-lock.yaml.
  if (!opts.jsonOutput && !opts.dryRun) {
    try {
      // Dynamic import to avoid a circular dep between sync.ts and
      // generate-metadata.ts. Don't "clean up" to a static import.
      const { rehashOnly } = await import("../generate-metadata/generate-metadata.ts");
      // Reuse the local-side file list from the change-tracker so we don't
      // re-walk the filesystem. Apply the just-applied changes to derive the
      // post-pull state: localMap is pre-pull, but auto-fill needs to see
      // additions and skip deletions.
      const postPullPaths = new Set<string>(Object.keys(localMap));
      for (const change of changes) {
        if (change.name === "added") postPullPaths.add(change.path);
        else if (change.name === "deleted") postPullPaths.delete(change.path);
      }
      const filled = await rehashOnly(opts as any, undefined, {
        missingOnly: true,
        localFiles: postPullPaths,
      });
      const total = filled.scripts + filled.flows + filled.apps;
      if (total > 0) {
        log.info(
          colors.gray(
            `Auto-filled ${total} missing lockfile entr${total === 1 ? "y" : "ies"} ` +
            `(${filled.scripts} script, ${filled.flows} flow, ${filled.apps} app) from disk.`,
          ),
        );
      }
    } catch (e) {
      // Re-throw fail-fast lockfile errors (unknown version, malformed yaml)
      // so the user sees them; only swallow soft failures from the auto-fill
      // walk itself.
      if (
        e instanceof UnknownLockVersionError ||
        e instanceof MalformedLockfileError
      ) {
        throw e;
      }
      log.warn(
        colors.yellow(
          `Could not auto-fill missing lockfile entries: ${e instanceof Error ? e.message : e}`,
        ),
      );
    }
  }

  try {
    await pullSharedUi(workspace.workspaceId);
  } catch (e) {
    log.warn(`Failed to pull shared UI folder: ${e}`);
  }

  // Datatable migrations are part of the workspace export now, so they flow
  // through the normal diff/apply above as `datatable_migration` items.

  // Git-sync deployment-callback mode stops here: branch checkout + pull have
  // happened, but commit + push are the caller's job. The hub script does
  // them in-process with `set_gpg_signing_secret` so the agent's pre-warmed
  // passphrase cache is still warm at sign time (WIN-1974). `gitSyncDeployPush`
  // stays exported for callers that want the same commit/push behavior.
}

// Internal git-sync deployment-callback entrypoint. Invoked only by the
// git-sync hub script (not user-facing — see the hidden `git-deploy`
// subcommand). Runs inside an existing clone of the repo: switches to the
// wm_deploy/fork branch when applicable, pulls the workspace content, then
// commits and pushes. Delegates to `pull` with deploy options set;
// non-interactive and branch-validation-free since there is no TTY.
export async function gitDeploy(
  opts: GlobalOptions &
    SyncOptions & {
      repository?: string;
      gitDeployItems?: string;
      useIndividualBranch?: boolean;
      groupByFolder?: boolean;
      onlyCreateBranch?: boolean;
      parentWorkspaceId?: string;
      devWorkspaceLabel?: string;
      parentDevWorkspaceLabel?: string;
      skipSecrets?: boolean;
      gitCommitterEmail?: string;
      gitCommitterName?: string;
    },
) {
  let items: GitSyncDeployItem[] = [];
  if (opts.gitDeployItems !== undefined) {
    try {
      items = JSON.parse(opts.gitDeployItems);
    } catch (e) {
      log.error(`Invalid --git-deploy-items JSON: ${e}`);
      process.exit(1);
    }
  }

  // Throwaway forks force-disable use_individual_branch / group_by_folder (1:1
  // with the hub script's inner()): they always sync to their own
  // wm-fork/<branch>/<id> branch, and — critically — that disabling also flips
  // the include/promotion derivation below. A dev workspace is the exception: it
  // honors promotion mode, detected via the environment label the backend passes.
  const isFork = isForkWorkspace(opts.workspace ?? "", opts.parentWorkspaceId);
  const useIndividualBranch =
    isFork && !opts.devWorkspaceLabel ? false : !!opts.useIndividualBranch;

  // Derive the include filters from the deployed items (replaces the hub
  // script's regexFromPath + per-kind --include-* construction).
  const includes = deriveGitSyncDeployIncludes(items, useIndividualBranch);

  // Promotion: in individual-branch mode, apply promotionOverrides from the
  // base branch the repo was cloned on (read now, before `pull` checks out
  // the wm_deploy branch). Mirrors the hub script's `--promotion <branch>`.
  const promotion =
    useIndividualBranch && !opts.promotion
      ? getCurrentGitBranch() ?? undefined
      : opts.promotion;

  await pull({
    ...opts,
    yes: true,
    skipBranchValidation: true,
    extraIncludes: [
      ...(opts.extraIncludes ?? []),
      ...includes.extraIncludes,
    ],
    // Workspace-wide mode force-includes the deployed default-excluded kinds
    // (full mirror). Individual-branch/promotion mode forces nothing — these
    // keys stay ABSENT so pull resolves them from the promotion target's
    // effective wmill.yaml filters. Spreading (not setting `false`) is what
    // makes the deferral work: an explicit `false` would clobber the effective
    // config in pull's Object.assign-based option merge.
    ...includes.forcedIncludes,
    promotion,
  } as any);
}

// Display label for a change's type, with a trailing space. Datatable migrations
// are self-describing via their `migrations/datatable/...` path, so they get no
// label prefix.
function changeTypeLabel(p: string): string {
  const t = getTypeStrFromPath(p);
  return t === "datatable_migration" ? "" : `${t} `;
}

function prettyChanges(
  changes: Change[],
  specificItems?: SpecificItemsConfig,
  branchOverride?: string,
  folderDefaultAnnotations?: Map<string, string>,
) {
  for (const change of changes) {
    let displayPath = change.path;
    let wsNote = "";

    // Check if this will be written as a workspace-specific file
    if (specificItems && isSpecificItem(change.path, specificItems)) {
      const workspaceSpecificPath = getWorkspaceSpecificPath(
        change.path,
        specificItems,
        branchOverride,
      );
      if (workspaceSpecificPath) {
        displayPath = workspaceSpecificPath;
        wsNote = " (workspace-specific)";
      }
    }

    const folderNote = folderDefaultAnnotations?.get(change.path);
    const extraNote = folderNote
      ? colors.cyan(` (will be permissioned as ${folderNote} via folder default)`)
      : "";

    if (change.name === "added") {
      log.info(
        colors.green(
          `+ ${changeTypeLabel(change.path)}` +
            displayPath +
            colors.gray(wsNote),
        ) + extraNote,
      );
    } else if (change.name === "deleted") {
      log.info(
        colors.red(
          `- ${changeTypeLabel(change.path)}` +
            displayPath +
            colors.gray(wsNote),
        ),
      );
    } else if (change.name === "edited") {
      const changeType = getTypeStrFromPath(change.path);
      log.info(
        colors.yellow(
          `~ ${changeTypeLabel(change.path)}` +
            displayPath +
            colors.gray(wsNote) +
            (change.codebase ? ` (codebase changed)` : ""),
        ),
      );
      if (change.before != change.after) {
        if (changeType === "encryption_key") {
          showDiff(
            redactEncryptionKey(change.before),
            redactEncryptionKey(change.after),
          );
        } else if (change.path.endsWith(".yaml")) {
          try {
            showDiff(
              yamlStringify(
                yamlParseContent(change.path, change.before),
                yamlOptions,
              ),
              yamlStringify(
                yamlParseContent(change.path, change.after),
                yamlOptions,
              ),
            );
          } catch {
            showDiff(change.before, change.after);
          }
        } else {
          showDiff(change.before, change.after);
        }
      }
    } else if (change.name === "ws_specific_flag") {
      log.info(
        colors.cyan(
          `~ ${change.kind} ${displayPath} ` +
            (change.wsSpecific
              ? "(mark as workspace-specific)"
              : "(unmark as workspace-specific)"),
        ),
      );
    }
  }
}

// function prettyDiff(diffs: Difference[]) {
//   for (const diff of diffs) {
//     let pathString = "";
//     for (const pathSegment of diff.path) {
//       if (typeof pathSegment === "string") {
//         pathString += ".";
//         pathString += pathSegment;
//       } else {
//         pathString += "[";
//         pathString += pathSegment;
//         pathString += "]";
//       }
//     }
//     if (diff.type === "REMOVE" || diff.type === "CHANGE") {
//      log.info(colors.red("- " + pathString + " = " + diff.oldValue));
//     }
//     if (diff.type === "CREATE" || diff.type === "CHANGE") {
//      log.info(colors.green("+ " + pathString + " = " + diff.value));
//     }
//   }
// }

function removeSuffix(str: string, suffix: string) {
  return str.slice(0, str.length - suffix.length);
}

// Shown after a `wmill sync push --dry-run` preview that has changes. `sync push`
// deploys to the remote workspace and is destructive (it overwrites and prunes
// remote items that differ from or are absent locally), so the preview reminds
// the caller — especially an AI agent that ran the dry-run to inspect changes —
// to get explicit user confirmation before applying it for real.
const SYNC_PUSH_DESTRUCTIVE_WARNING =
  "`wmill sync push` is destructive: applying it deploys these changes to the remote workspace and overwrites or deletes remote items that differ from or are absent locally — this is not automatically reversible. If you are an AI agent, do NOT run `wmill sync push` (without --dry-run) until the user has explicitly confirmed this deploy, unless your custom instructions explicitly allow bypassing that confirmation.";

// A script pushed without a local lock queues a server-side dependency job; if
// that job fails the script deploys broken (no lock/assets) with no CLI signal.
// One-shot (no polling): report this push's already-failed + still-running
// dependency jobs.
async function checkServerLockJobs(
  workspaceId: string,
  sinceIso: string,
  changedPaths: string[],
): Promise<{ pending: number; failed: { path: string; error?: string }[] }> {
  // A dependency job's script_path has no file extension; changed paths do.
  const belongsToPush = (scriptPath?: string) =>
    !!scriptPath &&
    changedPaths.some(
      (p) => p === scriptPath || p.startsWith(scriptPath + "."),
    );
  // Raw fetch: the checked-in generated client predates the `created_after` /
  // `success` filters on the job list routes (see the `apiGet` note in
  // pipeline.ts).
  const listJobs = async (path: string): Promise<unknown[]> => {
    const { OpenAPI } = await import("../../../gen/index.ts");
    const resp = await fetch(`${OpenAPI.BASE}${path}`, {
      headers: { ...getHeaders(), Authorization: `Bearer ${OpenAPI.TOKEN}` },
    });
    if (!resp.ok) throw new Error(`GET ${path} -> ${resp.status}`);
    return (await resp.json()) as unknown[];
  };
  try {
    const since = encodeURIComponent(sinceIso);
    const [queued, completed] = await Promise.all([
      listJobs(
        `/w/${workspaceId}/jobs/queue/list?job_kinds=dependencies&created_after=${since}`,
      ),
      listJobs(
        `/w/${workspaceId}/jobs/completed/list?job_kinds=dependencies&created_after=${since}&success=false`,
      ),
    ]);
    const pending = (queued as { script_path?: string }[]).filter((j) =>
      belongsToPush(j.script_path),
    ).length;
    const failed = (
      completed as { script_path?: string; result?: unknown }[]
    )
      .filter((j) => belongsToPush(j.script_path))
      .map((j) => ({
        path: j.script_path!,
        error: (j.result as { error?: { message?: string } } | undefined)
          ?.error?.message,
      }));
    return { pending, failed };
  } catch {
    // Advisory only: a failed check must not fail an otherwise-complete push.
    return { pending: 0, failed: [] };
  }
}

export async function push(
  opts: GlobalOptions & SyncOptions & { repository?: string; branch?: string; acceptOverridingPermissionedAsWithSelf?: boolean },
) {
  if ((opts as any).jsonOutput) log.setSilent(true);
  // Save original CLI options before merging with config file
  const originalCliOpts = { ...opts };

  // Load configuration from wmill.yaml and merge with CLI options
  opts = await mergeConfigWithConfigFile(opts);

  // --include-secrets overrides skipSecrets from wmill.yaml
  if ((originalCliOpts as any).includeSecrets) {
    opts.skipSecrets = false;
  }

  // Resolve workspace name for config lookups (same logic as pull)
  const hasExplicitCredentials = !!opts.baseUrl;
  let wsNameForConfig: string | undefined;

  if (opts.branch && !hasExplicitCredentials && !branchDeprecationWarned) {
    log.warn("⚠️  --branch/--env is deprecated. Use --workspace instead.");
    branchDeprecationWarned = true;
  }

  wsNameForConfig = resolveWsNameForConfigFromFlags(opts);

  if (!opts.branch && opts.workspace && !hasExplicitCredentials) {
    // Warn if override doesn't match a config key, or mismatches the auto-detected branch
    warnWorkspaceOverride(opts, opts.workspace);
  }

  // Validate workspace configuration early. Skip when ANY explicit flag is set
  // (even a --workspace value that doesn't match a config key — the user opted
  // out of branch-based auto-detection).
  try {
    await validateBranchConfiguration(opts, wsNameForConfig ?? opts.workspace);
  } catch (error) {
    if (error instanceof Error && error.message.includes("overrides")) {
      log.error(error.message);
      process.exit(1);
    }
    throw error;
  }

  const workspace = await resolveWorkspace(opts, wsNameForConfig);
  await requireLogin(opts);

  // If wsNameForConfig wasn't set from flags, infer from the resolved profile
  if (!wsNameForConfig) {
    wsNameForConfig = inferWsNameFromProfile(opts, workspace);
  }

  // Resolve effective sync options with workspace awareness
  const effectiveOpts = await resolveEffectiveSyncOptions(
    workspace,
    opts,
    opts.promotion,
    wsNameForConfig,
  );

  // Extract specific items configuration
  let specificItems = getSpecificItemsForCurrentBranch(opts, wsNameForConfig);

  // Compute the workspace name for file naming (default to workspaceId)
  let wsNameForFiles = wsNameForConfig ? resolveWsNameForFiles(opts, wsNameForConfig) : workspace.workspaceId;

  // Keep the pre-merge specificItems so we can detect entries that are
  // flagged locally but not yet ws_specific on the server (post-merge would
  // include both, making the comparison impossible).
  const localSpecificItems = specificItems;

  // Augment specificItems with server-side ws_specific entries
  const wsSpecificMerge = await mergeWsSpecificFromServer(workspace.workspaceId, specificItems);
  specificItems = wsSpecificMerge.merged;
  const serverWsSpecificItems = wsSpecificMerge.serverItems;

  // Merge CLI flags with resolved settings (CLI flags take precedence only for explicit overrides)
  opts = mergeCliWithEffectiveOptions(originalCliOpts, effectiveOpts);

  if (opts.lint) {
    log.info("Running lint validation before push...");
    const lintReport = await runLint(opts);
    printReport(lintReport, !!opts.jsonOutput);
    if (!lintReport.success) {
      log.error(colors.red("Push aborted due to lint failures."));
      process.exit(1);
    }
  }

  if (opts.locksRequired) {
    log.info("Checking for missing locks...");
    const lockIssues = await checkMissingLocks(opts);
    if (lockIssues.length > 0) {
      for (const issue of lockIssues) {
        for (const error of issue.errors) {
          log.error(colors.red(`  ${issue.path}: ${error}`));
        }
      }
      log.error(
        colors.red(
          `\nPush aborted: ${lockIssues.length} script(s) missing locks.`,
        ),
      );
      process.exit(1);
    }
    log.info(colors.green("All scripts have valid locks."));
  }

  const codebases = await listSyncCodebases(opts);
  if (opts.raw) {
    log.info("--raw is now the default, you can remove it as a flag");
  }
  if (opts.stateful) {
    if (!opts.skipPull) {
      log.info(
        colors.gray("You need to be up-to-date before pushing, pulling first."),
      );
      await pull(opts);
      log.info(colors.green("Pull done, now pushing."));
      log.info("\n");
    }
  }

  log.info(
    colors.gray(
      "Computing the files to update on the remote to match local (taking wmill.yaml includes/excludes into account)",
    ),
  );
  let resourceTypeToFormatExtension: Record<string, string> = {};
  let resourceTypeToIsFileset: Record<string, boolean> = {};
  try {
    const raw = (await wmill.fileResourceTypeToFileExtMap({
      workspace: workspace.workspaceId,
    })) as Record<string, string | FileResourceTypeInfo>;
    const parsed = parseFileResourceTypeMap(raw);
    resourceTypeToFormatExtension = parsed.formatExtMap;
    resourceTypeToIsFileset = parsed.filesetMap;
  } catch {
    // ignore
  }

  const remote = ZipFSElement(
    (await downloadZip(
      workspace,
      opts.plainSecrets,
      opts.skipVariables,
      opts.skipResources,
      opts.skipResourceTypes,
      opts.skipSecrets,
      opts.includeSchedules,
      opts.includeTriggers,
      opts.includeUsers,
      opts.includeGroups,
      opts.includeSettings,
      opts.includeKey,
      opts.skipWorkspaceDependencies,
      opts.defaultTs,
    ))!,
    !opts.json,
    opts.defaultTs ?? "bun",
    resourceTypeToFormatExtension,
    resourceTypeToIsFileset,
    false,
    parseSyncBehavior(opts.syncBehavior) >= 1,
  );

  const local = await FSFSElement(path.join(process.cwd(), ""), codebases, false);
  const { changes, localMap } = await compareDynFSElement(
    local,
    remote,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    true,
    codebases,
    false,
    specificItems,
    wsNameForFiles,
    false, // els1 (local) is not the remote source
    await isCaseInsensitiveFilesystem(process.cwd()),
  );

  // Detect resources/variables that the local config flags as ws_specific
  // but that aren't ws_specific on the server. The file-content diff misses
  // these because ws_specific isn't part of the YAML body — emit a dedicated
  // "ws_specific_flag" change so the apply loop can call updateResource /
  // updateVariable with just the metadata flag (no content payload). When the
  // file *also* has a content change, the existing "edited" change handles
  // both (push{Resource,Variable} forwards the wsSpecific arg), so we skip
  // injection in that case to avoid pushing twice.
  const wsSpecificFlagOnly = computeWsSpecificFlagOnlyPushes(
    localMap,
    localSpecificItems,
    serverWsSpecificItems,
  );
  for (const item of wsSpecificFlagOnly) {
    if (changes.some((c) => c.path === item.filePath)) continue;
    changes.push({
      name: "ws_specific_flag",
      path: item.filePath,
      kind: item.kind,
      wsSpecific: true,
    });
  }

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies(true);

  const tracker: ChangeTracker = await buildTracker(changes);

  const autoRegenerate = !!(opts as any).autoMetadata;
  const staleScripts: string[] = [];
  const staleFlows: string[] = [];
  const staleApps: string[] = [];

  // Auto-regenerate uses a DoubleLinkedDependencyTree so the dep job can
  // resolve cross-folder relative imports against not-yet-deployed scripts via
  // raw_script_temp + temp_script_refs. Without this the importer's lockgen
  // 404s on its sibling/parent imports because nothing has been pushed yet.
  const tree = autoRegenerate ? new DoubleLinkedDependencyTree() : undefined;
  if (tree) tree.setWorkspaceDeps(rawWorkspaceDependencies);

  // Pass 1: populate the tree (autoRegenerate) or run the legacy stale-check
  // (no autoRegenerate, just collect warnings).
  for (const change of tracker.scripts) {
    const stale = await generateScriptMetadataInternal(
      change,
      workspace,
      opts,
      true, // dryRun: pass 1 only populates the tree / detects staleness
      true,
      rawWorkspaceDependencies,
      codebases,
      false,
      tree,
    );
    if (!autoRegenerate && stale) {
      staleScripts.push(stale);
    }
  }
  for (const change of tracker.flows) {
    const stale = await generateFlowLockInternal(
      change,
      true,
      workspace,
      opts,
      false,
      true,
      tree,
    );
    if (!autoRegenerate && stale) {
      staleFlows.push(stale as string);
    }
  }
  for (const change of tracker.apps) {
    const stale = await generateAppLocksInternal(
      change,
      false,
      true,
      workspace,
      opts,
      true,
      true,
      tree,
    );
    if (!autoRegenerate && stale) {
      staleApps.push(stale as string);
    }
  }
  for (const change of tracker.rawApps) {
    const stale = await generateAppLocksInternal(
      change,
      true,
      true,
      workspace,
      opts,
      true,
      true,
      tree,
    );
    if (!autoRegenerate && stale) {
      staleApps.push(stale as string);
    }
  }

  if (autoRegenerate && tree) {
    // Propagate staleness through imports + upload script content to
    // raw_script_temp so the dep job can resolve cross-folder relative imports
    // via temp_script_refs (instead of hitting 404s for not-yet-deployed
    // scripts and recording lock_error_logs).
    tree.propagateStaleness();
    try {
      await uploadScripts(tree, workspace);
    } catch (e) {
      log.warn(
        colors.yellow(
          `Failed to upload scripts to temp storage (backend may be too old): ${e}. ` +
            `Locks will be generated using deployed script versions only — locally modified ` +
            `relative imports may not be reflected.`,
        ),
      );
    }

    // Pass 2: actually generate metadata/locks. Threading `tree` makes
    // generateScriptMetadataInternal include temp_script_refs in the
    // dependencies_async request so the dep job resolves relative imports
    // against raw_script_temp.
    for (const change of tracker.scripts) {
      const generated = await generateScriptMetadataInternal(
        change,
        workspace,
        opts,
        false,
        true,
        rawWorkspaceDependencies,
        codebases,
        false,
        tree,
      );
      if (generated) {
        staleScripts.push(generated);
      }
    }
    for (const change of tracker.flows) {
      const generated = await generateFlowLockInternal(
        change,
        false,
        workspace,
        opts,
        false,
        true,
        tree,
      );
      if (generated) {
        staleFlows.push(generated as string);
      }
    }
    for (const change of tracker.apps) {
      const generated = await generateAppLocksInternal(
        change,
        false,
        false,
        workspace,
        opts,
        true,
        true,
        tree,
      );
      if (generated) {
        staleApps.push(generated as string);
      }
    }
    for (const change of tracker.rawApps) {
      const generated = await generateAppLocksInternal(
        change,
        true,
        false,
        workspace,
        opts,
        true,
        true,
        tree,
      );
      if (generated) {
        staleApps.push(generated as string);
      }
    }
  }

  if (staleScripts.length > 0) {
    log.info("");
    if (autoRegenerate) {
      log.info("Auto-regenerated metadata for stale scripts:");
    } else {
      log.warn(
        "Stale scripts metadata found, you may want to update them using 'wmill generate-metadata' before pushing:",
      );
    }
    for (const stale of staleScripts) {
      if (autoRegenerate) {
        log.info(`  ${stale}`);
      } else {
        log.warn(stale);
      }
    }

    log.info("");
  }

  if (staleFlows.length > 0) {
    if (autoRegenerate) {
      log.info("Auto-regenerated locks for stale flows:");
    } else {
      log.warn(
        "Stale flows locks found, you may want to update them using 'wmill generate-metadata' before pushing:",
      );
    }
    for (const stale of staleFlows) {
      if (autoRegenerate) {
        log.info(`  ${stale}`);
      } else {
        log.warn(stale);
      }
    }
    log.info("");
  }

  if (staleApps.length > 0) {
    if (autoRegenerate) {
      log.info("Auto-regenerated locks for stale apps:");
    } else {
      log.warn(
        "Stale apps locks found, you may want to update them using 'wmill generate-metadata' before pushing:",
      );
    }
    for (const stale of staleApps) {
      if (autoRegenerate) {
        log.info(`  ${stale}`);
      } else {
        log.warn(stale);
      }
    }
    log.info("");
  }

  // Warn about local files for skipped types. Walks the in-memory DynFSElement tree
  // (not a fresh disk scan), but does re-traverse it. Acceptable cost for a one-time check.
  {
    const skippedWarnings: string[] = [];
    let scheduleCount = 0;
    let triggerCount = 0;
    for await (const entry of readDirRecursiveWithIgnore(() => false, local)) {
      if (entry.isDirectory) continue;
      if (!opts.includeSchedules && entry.path.endsWith(".schedule.yaml")) scheduleCount++;
      if (!opts.includeTriggers && entry.path.endsWith("_trigger.yaml")) triggerCount++;
    }
    if (scheduleCount > 0) {
      skippedWarnings.push(`Skipping ${scheduleCount} schedule file(s). Use --include-schedules or set includeSchedules: true in wmill.yaml`);
    }
    if (triggerCount > 0) {
      skippedWarnings.push(`Skipping ${triggerCount} trigger file(s). Use --include-triggers or set includeTriggers: true in wmill.yaml`);
    }
    for (const warning of skippedWarnings) {
      log.warn(warning);
    }
    if (skippedWarnings.length > 0) log.info("");
  }

  await fetchRemoteVersion(workspace);

  log.info(
    `remote (${workspace.name}) <- local: ${changes.length} changes to apply`,
  );
  // Check that every folder referenced in the changeset has a local folder.meta.yaml
  const missingFolders: string[] = [];
  if (changes.length > 0) {
    const folderNames = new Set<string>();
    for (const change of changes) {
      const parts = change.path.split(SEP);
      if (parts.length >= 3 && parts[0] === "f" && change.name !== "deleted") {
        folderNames.add(parts[1]);
      }
    }
    for (const folderName of folderNames) {
      const basePath = path.join("f", folderName, "folder.meta.yaml");
      const branchPath = getWorkspaceSpecificPath(
        `f/${folderName}/folder.meta.yaml`,
        specificItems,
        wsNameForFiles,
      );
      let found = false;
      // Check branch-specific variant first (e.g. folder.dev.meta.yaml)
      if (branchPath) {
        try {
          await stat(branchPath);
          found = true;
        } catch {
          // fall through to base path check
        }
      }
      // Then check base path
      if (!found) {
        try {
          await stat(basePath);
          found = true;
        } catch {
          // not found
        }
      }
      if (!found) {
        missingFolders.push(folderName);
      }
    }
  }

  if (missingFolders.length > 0) {
    const folderList = missingFolders.map((f) => `  - ${f}`).join("\n");
    const user = await wmill.whoami({ workspace: workspace.workspaceId });
    const userIsAdmin = user.is_admin;
    const msg =
      `${userIsAdmin ? "Warning: " : ""}Missing folder.meta.yaml for:\n${folderList}\n` +
      `Run 'wmill folder add-missing' to create them locally, then push again.`;
    if (!userIsAdmin) {
      if (opts.jsonOutput) {
        console.log(JSON.stringify({ success: false, error: "missing_folders", missing_folders: missingFolders, message: msg }, null, 2));
      } else {
        log.error(msg);
      }
      process.exit(1);
    }
    if (!opts.jsonOutput) {
      log.warn(msg);
    }
  }

  // Handle JSON output for dry-run
  if (opts.dryRun && opts.jsonOutput) {
    const result = {
      success: true,
      changes: changes.map((change) => ({
        type: change.name,
        path: change.path,
        ...(change.name === "edited" && change.codebase
          ? { codebase_changed: true }
          : {}),
        ...(specificItems && isSpecificItem(change.path, specificItems)
          ? {
              workspace_specific: true,
              workspace_specific_path: getWorkspaceSpecificPath(
                change.path,
                specificItems,
                wsNameForFiles,
              ),
            }
          : {}),
      })),
      total: changes.length,
      ...(changes.length > 0
        ? { warning: SYNC_PUSH_DESTRUCTIVE_WARNING }
        : {}),
    };
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  if (changes.length > 0) {
    // Compute folder-default annotations for added items (shown in prettyChanges + dry-run)
    let folderDefaultAnnotations: Map<string, string> | undefined;
    if (parseSyncBehavior(opts.syncBehavior) >= 1) {
      folderDefaultAnnotations = new Map();
      const folderRulesCache = new Map<string, Array<{ path_glob: string; permissioned_as: string }>>();
      for (const change of changes) {
        if (change.name !== "added") continue;
        const match = change.path.match(/^f\/([^/]+)\//);
        if (!match) continue;
        const folderName = match[1];
        if (!folderRulesCache.has(folderName)) {
          try {
            const folder = await wmill.getFolder({ workspace: workspace.workspaceId, name: folderName });
            folderRulesCache.set(folderName, (folder as any).default_permissioned_as ?? []);
          } catch {
            folderRulesCache.set(folderName, []);
          }
        }
        const rules = folderRulesCache.get(folderName)!;
        const remotePath = change.path.replace(/\.(script|schedule|http_trigger|websocket_trigger|kafka_trigger|nats_trigger|postgres_trigger|mqtt_trigger|amqp_trigger|sqs_trigger|gcp_trigger|azure_trigger|email_trigger)\.(yaml|json)$/, "").replace(/(\.flow|__flow)\/flow\.(yaml|json)$/, "").replace(/\.(app|raw_app)(\/app\.(yaml|json))?$/, "");
        const relative = remotePath.slice(`f/${folderName}/`.length);
        if (!relative) continue;
        for (const rule of rules) {
          if (minimatch(relative, rule.path_glob)) {
            folderDefaultAnnotations.set(change.path, rule.permissioned_as);
            break;
          }
        }
      }
    }

    if (!opts.jsonOutput) {
      prettyChanges(changes, specificItems, wsNameForFiles, folderDefaultAnnotations);
    }

    if (opts.dryRun) {
      log.info(colors.gray(`Dry run complete.`));
      log.warn(colors.yellow(`\n⚠ ${SYNC_PUSH_DESTRUCTIVE_WARNING}`));
      return;
    }

    let permissionedAsContext: PermissionedAsContext | undefined = undefined;
    if (parseSyncBehavior(opts.syncBehavior) >= 1) {
      const user = await wmill.whoami({ workspace: workspace.workspaceId });
      const userIsAdminOrDeployer =
        user.is_admin || (user.groups ?? []).includes("wm_deployers");
      log.debug(`permissioned_as: user=${user.email}, is_admin=${user.is_admin}, groups=${JSON.stringify(user.groups)}, isAdminOrDeployer=${userIsAdminOrDeployer}`);
      permissionedAsContext = {
        userCache: new Map(),
        userIsAdminOrDeployer,
        userEmail: user.email,
      };

      // ws_specific_flag changes have no content payload, so they don't
      // affect permissioned_as resolution — filter them out before the
      // pre-check (which expects only added/edited/deleted).
      await preCheckPermissionedAs(
        changes.filter((c) => c.name !== "ws_specific_flag"),
        user.email,
        userIsAdminOrDeployer,
        opts.acceptOverridingPermissionedAsWithSelf ?? false,
        !!process.stdin.isTTY,
      );
    } else if (folderDefaultAnnotations && folderDefaultAnnotations.size > 0) {
      log.warn(colors.yellow(
        `This workspace has folder default_permissioned_as rules that affect ${folderDefaultAnnotations.size} item(s) being pushed, ` +
        `but syncBehavior is not set in wmill.yaml. Add 'syncBehavior: v1' to enable ownership preservation on update and on_behalf_of stripping on pull.`
      ));
    }

    // Reject malformed datatable migrations (duplicate timestamps, orphan downs)
    // before touching the remote, scanning only the data tables in this push.
    const migrationDatatables = new Set(
      changes
        .map((c) => parseDatatableMigrationPath(c.path)?.datatable)
        .filter((d): d is string => !!d),
    );
    if (migrationDatatables.size > 0) {
      const migrationErrors = validateLocalMigrations(migrationDatatables);
      if (migrationErrors.length > 0) {
        log.error(
          "Invalid datatable migrations, aborting push:\n" +
            migrationErrors.map((e) => `  - ${e}`).join("\n"),
        );
        process.exit(1);
      }
    }

    if (
      !opts.yes &&
      !(await Confirm.prompt({
        message: `Do you want to apply these ${changes.length} changes to the remote?`,
        default: true,
      }))
    ) {
      return;
    }

    const start = performance.now();
    const pushStartedAt = new Date().toISOString();
    log.info(colors.gray(`Applying changes to files ...`));

    let stateful = opts.stateful;
    if (stateful) {
      try {
        await stat(path.join(process.cwd(), ".wmill"));
      } catch {
        stateful = false;
      }
    }

    // Group changes by base path (before first dot)
    const groupedChanges = new Map<string, typeof changes>();
    for (const change of changes) {
      const basePath = change.path.split(".")[0];
      if (!groupedChanges.has(basePath)) {
        groupedChanges.set(basePath, []);
      }
      groupedChanges.get(basePath)!.push(change);
    }

    let parallelizationFactor = opts.parallel ?? 1;
    if (parallelizationFactor <= 0) {
      parallelizationFactor = 1;
    }
    // Partition changes: folder.meta.yaml changes must be applied BEFORE any
    // item changes under those folders, so a push that updates a folder's
    // default_permissioned_as rules AND creates new items under it in the same
    // changeset has the rules in place when the backend resolves defaults for
    // the new items. Folder changes run sequentially first; everything else
    // runs through the parallel pool afterwards.
    const allGrouped = Array.from(groupedChanges.entries());
    const isFolderMetaGroup = ([basePath]: [string, typeof changes]) =>
      basePath.endsWith(`${SEP}folder`) || basePath === "folder";
    const folderMetaGroups = allGrouped.filter(isFolderMetaGroup);
    const groupedChangesArray = allGrouped.filter((g) => !isFolderMetaGroup(g));
    log.info(
      `found changes for ${
        allGrouped.length
      } items with a total of ${allGrouped.reduce(
        (acc, [_, changes]) => acc + changes.length,
        0,
      )} files to process`,
    );
    if (parallelizationFactor > 1) {
      log.info(`Parallelizing ${parallelizationFactor} changes at a time`);
    }

    // Create a pool of workers that processes items as they become available
    const pool = new Set();
    // Process folder.meta groups first (sequentially), then items in parallel.
    // This ensures a newly-added default_permissioned_as rule is applied before
    // any item created under that folder in the same push.
    const queue = [...folderMetaGroups, ...groupedChangesArray];
    let folderPhaseRemaining = folderMetaGroups.length;
    const effectiveParallelism = () =>
      folderPhaseRemaining > 0 ? 1 : parallelizationFactor;
    // Cache git branch at the start to avoid repeated execSync calls per change
    const cachedWsNameForPush = wsNameForFiles || (isGitRepository() ? getCurrentGitBranch() : null);

    // Datatable migrations are two files (.up.sql/.down.sql) for one record, so
    // dedupe upsert/delete by (datatable, version) across the whole push.
    const pushedMigrationKeys = new Set<string>();
    // Migrations newly added by this push (an added .up.sql) — offered to run once
    // the push has completed.
    const newDatatableMigrations = changes
      .filter((c) => c.name === "added")
      .map((c) => parseDatatableMigrationPath(c.path))
      .filter((p) => !!p && p.kind === "up")
      .map((p) => ({
        datatable: p!.datatable,
        timestamp: p!.timestamp,
        name: p!.name,
      }));

    while (queue.length > 0 || pool.size > 0) {
      // Fill the pool until we reach the effective parallelism limit.
      // During the folder-meta phase this is 1 (sequential) so no item change
      // starts before all folder.meta updates have been applied to the backend.
      while (pool.size < effectiveParallelism() && queue.length > 0) {
        const [groupBasePath, initialChanges] = queue.shift()!;
        let changes = initialChanges;
        const isFolderGroup =
          groupBasePath.endsWith(`${SEP}folder`) || groupBasePath === "folder";
        const promise = (async () => {
          const alreadySynced: string[] = [];
          const deletedVarsResPaths: string[] = [];
          const isRawApp = isRawAppFile(changes[0].path);
          if (isRawApp) {
            const deleteRawApp = changes.find(
              (change) =>
                change.name === "deleted" &&
                isRawAppFolderMetadataFile(change.path),
            );
            if (deleteRawApp) {
              changes = [deleteRawApp];
            } else {
              changes.splice(1, changes.length - 1);
            }
          }

          for await (const change of changes) {
            // A datatable migration is one record across two files; upsert/delete
            // it from disk once (deduped), regardless of which file changed.
            if (isDatatableMigrationPath(change.path)) {
              const parsed = parseDatatableMigrationPath(change.path);
              if (parsed) {
                const key = `${parsed.datatable}\0${parsed.timestamp}`;
                if (!pushedMigrationKeys.has(key)) {
                  pushedMigrationKeys.add(key);
                  await pushMigrationFromDisk(workspace.workspaceId, parsed);
                }
              }
              continue;
            }

            let stateTarget = undefined;
            if (stateful) {
              try {
                stateTarget = path.join(process.cwd(), ".wmill", change.path);
                await stat(stateTarget);
              } catch {
                stateTarget = undefined;
              }
            }

            if (change.name === "edited") {
              if (
                await handleScriptMetadata(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  rawWorkspaceDependencies,
                  codebases,
                  opts,
                  permissionedAsContext,
                )
              ) {
                if (stateTarget) {
                  await writeFile(stateTarget, change.after, "utf-8");
                }
                continue;
              } else if (
                await handleFile(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  rawWorkspaceDependencies,
                  codebases,
                  permissionedAsContext,
                )
              ) {
                if (stateTarget) {
                  await writeFile(stateTarget, change.after, "utf-8");
                }
                continue;
              } else if (isScriptModulePath(change.path)) {
                // Module file changed — push the parent script
                await pushParentScriptForModule(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  rawWorkspaceDependencies,
                  codebases,
                );
                if (stateTarget) {
                  await writeFile(stateTarget, change.after, "utf-8");
                }
                continue;
              }
              if (stateTarget) {
                await mkdir(path.dirname(stateTarget), { recursive: true });
                log.info(
                  `Editing ${getTypeStrFromPath(change.path)} ${change.path}`,
                );
              }

              if (isFileResource(change.path)) {
                const resourceFilePath = await findResourceFile(change.path);
                if (!alreadySynced.includes(resourceFilePath)) {
                  alreadySynced.push(resourceFilePath);

                  const newObj = parseFromPath(
                    resourceFilePath,
                    await readTextFile(resourceFilePath),
                  );

                  // For branch-specific resources, push to the base path on the workspace server
                  // This ensures workspace-specific files are stored with their base names in the workspace
                  let serverPath = resourceFilePath;
                  const currentBranch = cachedWsNameForPush;
                  let isFileResWsSpecific = false;

                  if (currentBranch && isWorkspaceSpecificFile(resourceFilePath)) {
                    serverPath = fromWorkspaceSpecificPath(
                      resourceFilePath,
                      currentBranch,
                    );
                    isFileResWsSpecific = true;
                  } else if (specificItems && isSpecificItem(change.path, specificItems)) {
                    isFileResWsSpecific = true;
                  }

                  await pushResource(
                    workspace.workspaceId,
                    serverPath,
                    undefined,
                    newObj,
                    resourceFilePath,
                    isFileResWsSpecific ? true : undefined,
                  );
                  if (stateTarget) {
                    await writeFile(stateTarget, change.after, "utf-8");
                  }
                  continue;
                }
              }
              if (isFilesetResource(change.path)) {
                const result = await pushFilesetParentResource(
                  change.path,
                  workspace.workspaceId,
                  alreadySynced,
                  cachedWsNameForPush,
                  specificItems,
                );
                if (result.status === "parent-missing") {
                  throw new Error(
                    `No resource metadata file found for fileset resource: ${change.path}`,
                  );
                }
                if (result.status === "pushed") {
                  if (stateTarget) {
                    await writeFile(stateTarget, change.after, "utf-8");
                  }
                  continue;
                }
                // "already-synced": fall through (pre-existing behavior).
              }
              const oldObj = parseFromPath(change.path, change.before);
              const newObj = parseFromPath(change.path, change.after);

              // Check if this is a branch-specific item and get the original workspace-specific path
              let originalWorkspaceSpecificPath: string | undefined;
              const isWsSpecific = specificItems && isSpecificItem(change.path, specificItems);
              if (isWsSpecific) {
                originalWorkspaceSpecificPath = getWorkspaceSpecificPath(
                  change.path,
                  specificItems,
                  wsNameForFiles,
                );
              }

              await pushObj(
                workspace.workspaceId,
                change.path,
                oldObj,
                newObj,
                opts.plainSecrets ?? false,
                alreadySynced,
                opts.message,
                originalWorkspaceSpecificPath,
                permissionedAsContext,
                isWsSpecific ? true : undefined,
                {
                  noninteractive: (opts.yes ?? false) || !process.stdin.isTTY,
                  skipReencrypt: opts.skipReencryptOnKeyChange,
                },
              );

              if (stateTarget) {
                await writeFile(stateTarget, change.after, "utf-8");
              }
            } else if (change.name === "added") {
              if (isFilesetResource(change.path)) {
                // Re-push the parent resource (guarded by alreadySynced).
                // Parent-missing means the parent itself is also being added and
                // its own change will push the full fileset — safe to skip.
                await pushFilesetParentResource(
                  change.path,
                  workspace.workspaceId,
                  alreadySynced,
                  cachedWsNameForPush,
                  specificItems,
                );
                continue;
              }
              if (
                !isRawAppFile(change.path) &&
                (change.path.endsWith(".script.json") ||
                  change.path.endsWith(".script.yaml") ||
                  change.path.endsWith(".lock") ||
                  isFileResource(change.path))
              ) {
                continue;
              } else if (
                await handleFile(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  rawWorkspaceDependencies,
                  codebases,
                  permissionedAsContext,
                )
              ) {
                continue;
              } else if (isScriptModulePath(change.path)) {
                await pushParentScriptForModule(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  rawWorkspaceDependencies,
                  codebases,
                );
                continue;
              }
              if (stateTarget) {
                await mkdir(path.dirname(stateTarget), { recursive: true });
                log.info(
                  `Adding ${getTypeStrFromPath(change.path)} ${change.path}`,
                );
              }
              const obj = parseFromPath(change.path, change.content);

              // Determine the actual local file path for this change
              // For branch-specific items, we read from workspace-specific files but push to base server paths
              let localFilePath = change.path;
              const isAddedWsSpecific = specificItems && isSpecificItem(change.path, specificItems);
              if (isAddedWsSpecific) {
                const workspaceSpecificPath = getWorkspaceSpecificPath(
                  change.path,
                  specificItems,
                  wsNameForFiles,
                );
                if (workspaceSpecificPath) {
                  localFilePath = workspaceSpecificPath;
                }
              }

              await pushObj(
                workspace.workspaceId,
                change.path,
                undefined,
                obj,
                opts.plainSecrets ?? false,
                [],
                opts.message,
                localFilePath, // Pass the actual local file path
                permissionedAsContext,
                isAddedWsSpecific ? true : undefined,
                {
                  noninteractive: (opts.yes ?? false) || !process.stdin.isTTY,
                  skipReencrypt: opts.skipReencryptOnKeyChange,
                },
              );

              if (stateTarget) {
                await writeFile(stateTarget, change.content, "utf-8");
              }
            } else if (change.name === "deleted") {
              if (change.path.endsWith(".lock")) {
                continue;
              }
              if (isScriptModulePath(change.path)) {
                // Module file deleted — push the parent script (which will now have fewer modules)
                await pushParentScriptForModule(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  rawWorkspaceDependencies,
                  codebases,
                );
                continue;
              }
              if (isFilesetResource(change.path)) {
                // Re-push the parent resource (guarded by alreadySynced).
                // Parent-missing means the parent itself is also being deleted
                // and its own "deleted" change removes the whole resource.
                await pushFilesetParentResource(
                  change.path,
                  workspace.workspaceId,
                  alreadySynced,
                  cachedWsNameForPush,
                  specificItems,
                );
                continue;
              }
              const typ = getTypeStrFromPath(change.path);

              if (typ == "script") {
                log.info(`Archiving ${typ} ${change.path}`);
              } else {
                log.info(`Deleting ${typ} ${change.path}`);
              }
              const workspaceId = workspace.workspaceId;
              const target = change.path.replaceAll(SEP, "/");
              switch (typ) {
                case "script": {
                  await wmill.archiveScriptByPath({
                    workspace: workspaceId,
                    path: removeExtensionToPath(target),
                  });
                  break;
                }
                case "folder":
                  await wmill.deleteFolder({
                    workspace: workspaceId,
                    name: change.path.split(SEP)[1],
                  });
                  break;
                case "resource": {
                  const resourcePath = removeSuffix(target, ".resource.json");
                  try {
                    await wmill.deleteResource({
                      workspace: workspaceId,
                      path: resourcePath,
                    });
                  } catch (e: any) {
                    if (e?.status === 404 && deletedVarsResPaths.includes(resourcePath)) {
                      log.debug(`Resource ${resourcePath} already deleted by linked variable`);
                    } else {
                      throw e;
                    }
                  }
                  deletedVarsResPaths.push(resourcePath);
                  break;
                }
                case "resource-type":
                  await wmill.deleteResourceType({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".resource-type.json"),
                  });
                  break;
                case "flow":
                  if (isFlowFolderMetadataFile(target)) {
                    // Metadata file deleted — delete the entire flow
                    await wmill.deleteFlowByPath({
                      workspace: workspaceId,
                      path: removeSuffix(target, getDeleteSuffix("flow", "json")),
                    });
                  } else {
                    // Inline script file deleted within flow folder
                    const flowFolder = extractFolderPath(target, "flow");
                    let flowFolderExists = false;
                    if (flowFolder) {
                      try {
                        await stat(flowFolder);
                        flowFolderExists = true;
                      } catch {
                        // folder doesn't exist
                      }
                    }
                    if (flowFolderExists) {
                      // Re-push the entire flow so the backend gets the updated definition
                      await pushObj(
                        workspaceId,
                        target,
                        undefined,
                        undefined,
                        opts.plainSecrets ?? false,
                        alreadySynced,
                        opts.message,
                      );
                    } else {
                      // Flow folder doesn't exist locally — delete on server
                      const remotePath = extractResourceName(target, "flow");
                      if (remotePath) {
                        await wmill.deleteFlowByPath({
                          workspace: workspaceId,
                          path: remotePath,
                        });
                      }
                    }
                  }
                  break;
                case "app":
                  if (isAppFolderMetadataFile(target)) {
                    // Metadata file deleted — delete the entire app
                    await wmill.deleteApp({
                      workspace: workspaceId,
                      path: removeSuffix(target, getDeleteSuffix("app", "json")),
                    });
                  } else {
                    // Inline script file deleted within app folder
                    const appFolder = extractFolderPath(target, "app");
                    let appFolderExists = false;
                    if (appFolder) {
                      try {
                        await stat(appFolder);
                        appFolderExists = true;
                      } catch {
                        // folder doesn't exist
                      }
                    }
                    if (appFolderExists) {
                      // Re-push the entire app so the backend gets the updated definition
                      await pushObj(
                        workspaceId,
                        target,
                        undefined,
                        undefined,
                        opts.plainSecrets ?? false,
                        alreadySynced,
                        opts.message,
                      );
                    } else {
                      // App folder doesn't exist locally — delete on server
                      const remotePath = extractResourceName(target, "app");
                      if (remotePath) {
                        await wmill.deleteApp({
                          workspace: workspaceId,
                          path: remotePath,
                        });
                      }
                    }
                  }
                  break;
                case "raw_app":
                  if (isRawAppFolderMetadataFile(target)) {
                    // Delete the entire raw app
                    await wmill.deleteApp({
                      workspace: workspaceId,
                      path: removeSuffix(target, getDeleteSuffix("raw_app", "json")),
                    });
                  } else {
                    const rawAppFolder = extractFolderPath(target, "raw_app");
                    let folderExists = false;
                    if (rawAppFolder) {
                      try {
                        await stat(rawAppFolder);
                        folderExists = true;
                      } catch {
                        // folder doesn't exist
                      }
                    }
                    if (folderExists) {
                      // For individual file deletions within a raw app,
                      // re-push the entire raw app so the backend gets the updated file list
                      // (the deleted file won't be included in the push)
                      await pushObj(
                        workspaceId,
                        target,
                        undefined,
                        undefined,
                        opts.plainSecrets ?? false,
                        alreadySynced,
                        opts.message,
                      );
                    } else {
                      // The entire raw app folder was deleted locally,
                      // delete the raw app on the server
                      const remotePath = extractResourceName(target, "raw_app");
                      if (remotePath) {
                        await wmill.deleteApp({
                          workspace: workspaceId,
                          path: remotePath,
                        });
                      }
                    }
                  }
                  break;
                case "schedule":
                  await wmill.deleteSchedule({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".schedule.json"),
                  });
                  break;
                case "http_trigger":
                  await wmill.deleteHttpTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".http_trigger.json"),
                  });
                  break;
                case "websocket_trigger":
                  await wmill.deleteWebsocketTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".websocket_trigger.json"),
                  });
                  break;
                case "kafka_trigger":
                  await wmill.deleteKafkaTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".kafka_trigger.json"),
                  });
                  break;
                case "nats_trigger":
                  await wmill.deleteNatsTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".nats_trigger.json"),
                  });
                  break;
                case "postgres_trigger":
                  await wmill.deletePostgresTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".postgres_trigger.json"),
                  });
                  break;
                case "mqtt_trigger":
                  await wmill.deleteMqttTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".mqtt_trigger.json"),
                  });
                  break;
                case "amqp_trigger":
                  await wmill.deleteAmqpTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".amqp_trigger.json"),
                  });
                  break;
                case "sqs_trigger":
                  await wmill.deleteSqsTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".sqs_trigger.json"),
                  });
                  break;
                case "gcp_trigger":
                  await wmill.deleteGcpTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".gcp_trigger.json"),
                  });
                  break;
                case "azure_trigger":
                  await wmill.deleteAzureTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".azure_trigger.json"),
                  });
                  break;
                case "email_trigger":
                  await wmill.deleteEmailTrigger({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".email_trigger.json"),
                  });
                  break;
                case "native_trigger": {
                  const triggerInfo = extractNativeTriggerInfo(change.path);
                  if (!triggerInfo) {
                    throw new Error(
                      `Invalid native trigger path: ${change.path}`
                    );
                  }
                  await wmill.deleteNativeTrigger({
                    workspace: workspaceId,
                    serviceName: triggerInfo.serviceName as NativeServiceName,
                    externalId: triggerInfo.externalId,
                  });
                  break;
                }
                case "variable": {
                  const variablePath = removeSuffix(target, ".variable.json");
                  try {
                    await wmill.deleteVariable({
                      workspace: workspaceId,
                      path: variablePath,
                    });
                  } catch (e: any) {
                    if (e?.status === 404 && deletedVarsResPaths.includes(variablePath)) {
                      log.debug(`Variable ${variablePath} already deleted by linked resource`);
                    } else {
                      throw e;
                    }
                  }
                  deletedVarsResPaths.push(variablePath);
                  break;
                }
                case "user": {
                  const users = await wmill.listUsers({
                    workspace: workspaceId,
                  });

                  const email = removeSuffix(
                    removePathPrefix(change.path, "users"),
                    ".user.json",
                  );
                  const user = users.find((u) => u.email === email);
                  if (!user) {
                    throw new Error(`User ${email} not found`);
                  }
                  await wmill.deleteUser({
                    workspace: workspaceId,
                    username: user.username,
                  });
                  break;
                }
                case "group":
                  await wmill.deleteGroup({
                    workspace: workspaceId,
                    name: removeSuffix(
                      removePathPrefix(change.path, "groups"),
                      ".group.json",
                    ),
                  });
                  break;
                case "workspace_dependencies":
                  const relativePath = removePathPrefix(
                    change.path,
                    "dependencies",
                  );

                  const res = workspaceDependenciesPathToLanguageAndFilename(
                    change.path,
                  );
                  if (!res) {
                    throw new Error(
                      `Unknown workspace dependencies file format: ${change.path}`,
                    );
                  }
                  const { name, language } = res;

                  await wmill.deleteWorkspaceDependencies({
                    workspace: workspaceId,
                    language,
                    name,
                  });

                  break;
                default:
                  break;
              }
              if (stateTarget) {
                try {
                  await rm(stateTarget);
                } catch {
                  // state target may not exist already
                }
              }
            } else if (change.name === "ws_specific_flag") {
              const target = change.path.replaceAll(SEP, "/");
              if (change.kind === "resource") {
                await wmill.updateResource({
                  workspace: workspace.workspaceId,
                  path: removeType(target, "resource"),
                  requestBody: { ws_specific: change.wsSpecific },
                });
              } else if (change.kind === "variable") {
                await wmill.updateVariable({
                  workspace: workspace.workspaceId,
                  path: removeType(target, "variable"),
                  requestBody: { ws_specific: change.wsSpecific },
                });
              } else {
                log.warn(
                  `ws_specific_flag change for unsupported kind '${change.kind}' at ${change.path} — skipping`,
                );
              }
            }
          }
        })();

        pool.add(promise);
        // Remove from pool when complete
        promise.then(() => {
          pool.delete(promise);
          if (isFolderGroup) folderPhaseRemaining--;
        });
      }

      // Wait for at least one task to complete before continuing
      if (pool.size > 0) {
        await Promise.race(pool);
      }
    }
    try {
      await pushSharedUi(workspace.workspaceId);
    } catch (e) {
      log.warn(`Failed to push shared UI folder: ${e}`);
    }
    try {
      await offerToRunNewMigrations(workspace.workspaceId, newDatatableMigrations, {
        yes: opts.yes,
        jsonOutput: opts.jsonOutput,
      });
    } catch (e: any) {
      log.warn(
        `Failed to run new datatable migrations: ${e?.body ?? e?.message ?? e}`,
      );
    }
    const lockJobs = await checkServerLockJobs(
      workspace.workspaceId,
      pushStartedAt,
      changes.map((c) => c.path.replaceAll(SEP, "/")),
    );
    if (!opts.jsonOutput) {
      for (const f of lockJobs.failed) {
        log.warn(
          `⚠ server-side lock generation FAILED for ${f.path} — the deployed script is broken until it locks.` +
            (f.error ? `\n  ${f.error.split("\n")[0]}` : ""),
        );
      }
      if (lockJobs.pending > 0) {
        log.info(
          colors.gray(
            `${lockJobs.pending} server-side lock job(s) still running — locks (and inferred assets) land when they finish; check the Runs page if a script stays broken.`,
          ),
        );
      }
    }
    if (opts.jsonOutput) {
      const result = {
        success: true,
        lock_jobs: lockJobs,
        message: `All ${changes.length} changes pushed to the remote workspace ${workspace.workspaceId} named ${workspace.name}`,
        changes: changes.map((change) => ({
          type: change.name,
          path: change.path,
          ...(change.name === "edited" && change.codebase
            ? { codebase_changed: true }
            : {}),
          ...(specificItems && isSpecificItem(change.path, specificItems)
            ? {
                workspace_specific: true,
                workspace_specific_path: getWorkspaceSpecificPath(
                  change.path,
                  specificItems,
                  wsNameForFiles,
                ),
              }
            : {}),
        })),
        total: changes.length,
        duration_ms: Math.round(performance.now() - start),
      };
      console.log(JSON.stringify(result, null, 2));
    } else {
      log.info(
        colors.bold.green.underline(
          `\nDone! All ${
            changes.length
          } changes pushed to the remote workspace ${
            workspace.workspaceId
          } named ${workspace.name} (${(performance.now() - start).toFixed(
            0,
          )}ms)`,
        ),
      );
    }
  } else {
    try {
      await pushSharedUi(workspace.workspaceId);
    } catch (e) {
      log.warn(`Failed to push shared UI folder: ${e}`);
    }
    // No changes pushed, so no new datatable migrations to run.
    if (opts.jsonOutput) {
      console.log(
        JSON.stringify(
          { success: true, message: "No changes to push", total: 0 },
          null,
          2,
        ),
      );
    }
  }
}

const command = new Command()
  .description(
    "sync local with a remote workspaces or the opposite (push or pull)",
  )
  .action(() =>
    log.info("2 actions available, pull and push. Use -h to display help."),
  )
  .command("pull")
  .description("Pull any remote changes and apply them locally.")
  .option("--yes", "Pull without needing confirmation")
  .option(
    "--dry-run",
    "Show changes that would be pulled without actually pushing",
  )
  .option("--plain-secrets", "Pull secrets as plain text")
  .option("--json", "Use JSON instead of YAML")
  .option("--skip-variables", "Skip syncing variables (including secrets)")
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option("--include-secrets", "Include secrets in sync (overrides skipSecrets in wmill.yaml)")
  .option("--skip-resources", "Skip syncing  resources")
  .option("--skip-resource-types", "Skip syncing  resource types")
  .option("--skip-scripts", "Skip syncing scripts")
  .option("--skip-flows", "Skip syncing flows")
  .option("--skip-apps", "Skip syncing apps")
  .option("--skip-folders", "Skip syncing folders")
  .option(
    "--skip-workspace-dependencies",
    "Skip syncing workspace dependencies",
  )
  // .option("--skip-scripts-metadata", "Skip syncing scripts metadata, focus solely on logic")
  .option("--include-schedules", "Include syncing  schedules")
  .option("--include-triggers", "Include syncing triggers")
  .option("--include-users", "Include syncing users")
  .option("--include-groups", "Include syncing groups")
  .option("--include-settings", "Include syncing workspace settings")
  .option("--include-key", "Include workspace encryption key")
  .option("--skip-branch-validation", "Skip git branch validation and prompts")
  .option("--json-output", "Output results in JSON format")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Overrides wmill.yaml includes",
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account. Overrides wmill.yaml excludes",
  )
  .option(
    "--extra-includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Useful to still take wmill.yaml into account and act as a second pattern to satisfy",
  )
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo) when multiple repositories exist",
  )
  .option(
    "--promotion <branch:string>",
    "Use promotionOverrides from the specified branch instead of regular overrides",
  )
  .option(
    "--branch, --env <branch:string>",
    "[Deprecated: use --workspace] Override the current git branch/environment",
  )
  .action(pull as any)
  .command("push")
  .description("Push any local changes and apply them remotely.")
  .option("--yes", "Push without needing confirmation")
  .option(
    "--dry-run",
    "Show changes that would be pushed without actually pushing",
  )
  .option("--plain-secrets", "Push secrets as plain text")
  .option("--json", "Use JSON instead of YAML")
  .option("--skip-variables", "Skip syncing variables (including secrets)")
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option("--include-secrets", "Include secrets in sync (overrides skipSecrets in wmill.yaml)")
  .option("--skip-resources", "Skip syncing  resources")
  .option("--skip-resource-types", "Skip syncing  resource types")
  .option("--skip-scripts", "Skip syncing scripts")
  .option("--skip-flows", "Skip syncing flows")
  .option("--skip-apps", "Skip syncing apps")
  .option("--skip-folders", "Skip syncing folders")
  .option(
    "--skip-workspace-dependencies",
    "Skip syncing workspace dependencies",
  )
  // .option("--skip-scripts-metadata", "Skip syncing scripts metadata, focus solely on logic")
  .option("--include-schedules", "Include syncing schedules")
  .option("--include-triggers", "Include syncing triggers")
  .option("--include-users", "Include syncing users")
  .option("--include-groups", "Include syncing groups")
  .option("--include-settings", "Include syncing workspace settings")
  .option("--include-key", "Include workspace encryption key")
  .option(
    "--skip-reencrypt-on-key-change",
    "When the pushed encryption key differs from the remote, do NOT re-encrypt existing remote secrets. Only safe if they are already encrypted with the new key (e.g. workspace/instance migration). Default is to re-encrypt.",
  )
  .option("--skip-branch-validation", "Skip git branch validation and prompts")
  .option("--json-output", "Output results in JSON format")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)",
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account.",
  )
  .option(
    "--extra-includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Useful to still take wmill.yaml into account and act as a second pattern to satisfy",
  )
  .option(
    "--message <message:string>",
    "Include a message that will be added to all scripts/flows/apps updated during this push",
  )
  .option("--parallel <number>", "Number of changes to process in parallel")
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo) when multiple repositories exist",
  )
  .option(
    "--branch, --env <branch:string>",
    "[Deprecated: use --workspace] Override the current git branch/environment",
  )
  .option("--lint", "Run lint validation before pushing")
  .option(
    "--locks-required",
    "Fail if scripts or flow inline scripts that need locks have no locks",
  )
  .option("--auto-metadata", "Automatically regenerate stale metadata (locks and schemas) before pushing")
  .option(
    "--accept-overriding-permissioned-as-with-self",
    "Accept that items with a different permissioned_as will be updated with your own user",
  )
  .action(push as any)
  // Internal: invoked only by the git-sync hub script. Hidden from help and
  // the generated agent system prompts (see system_prompts/generate.py).
  .command("git-deploy")
  .hidden()
  .description(
    "Internal git-sync deployment-callback step (used by the git-sync hub script). Runs inside an existing clone: switches to the wm_deploy/fork branch when applicable, pulls workspace content, then commits and pushes.",
  )
  .option(
    "--repository <repo:string>",
    "Repository resource path (e.g. u/user/repo)",
  )
  .option(
    "--git-deploy-items <json:string>",
    "JSON array of {path_type,path,parent_path,commit_msg} being deployed",
  )
  .option(
    "--use-individual-branch",
    "Push each deployed object to its own wm_deploy/<workspace>/<...> branch",
  )
  .option(
    "--group-by-folder",
    "With --use-individual-branch, group deployed objects per folder branch",
  )
  .option(
    "--only-create-branch",
    "Only create/push the deploy branch, skip pulling and committing files",
  )
  .option(
    "--parent-workspace-id <id:string>",
    "Parent workspace id, used to root a fork-of-a-fork branch",
  )
  .option(
    "--dev-workspace-label <label:string>",
    "Environment label of a dev workspace (dev/staging); its deploys go to that branch",
  )
  .option(
    "--parent-dev-workspace-label <label:string>",
    "Environment label of the parent dev workspace; roots a fork-of-dev branch on it",
  )
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option(
    "--git-committer-email <email:string>",
    "Committer email for the deploy commit (GPG-signed repos pass the GPG key email; defaults to WM_EMAIL)",
  )
  .option(
    "--git-committer-name <name:string>",
    "Committer name for the deploy commit (defaults to WM_USERNAME)",
  )
  .action(gitDeploy as any);

export default command;
