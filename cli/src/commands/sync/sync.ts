import { requireLogin } from "../../core/auth.ts";
import { fetchVersion, resolveWorkspace } from "../../core/context.ts";
import { readFile, writeFile, readdir, stat, rm, copyFile, mkdir } from "node:fs/promises";
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
  showConflict,
  showDiff,
  extractNativeTriggerInfo,
} from "../../types.ts";
import { downloadZip } from "./pull.ts";
import { runLint, printReport, checkMissingLocks } from "../lint/lint.ts";

import {
  exts,
  findContentFile,
  findResourceFile,
  handleScriptMetadata,
  removeExtensionToPath,
} from "../script/script.ts";

import { handleFile } from "../script/script.ts";
import {
  deepEqual,
  fetchRemoteVersion,
  isFileResource,
  isFilesetResource,
  isRawAppFile,
  isWorkspaceDependencies,
} from "../../utils/utils.ts";
import {
  getEffectiveSettings,
  mergeConfigWithConfigFile,
  SyncOptions,
  validateBranchConfiguration,
} from "../../core/conf.ts";
import {
  fromBranchSpecificPath,
  getBranchSpecificPath,
  getSpecificItemsForCurrentBranch,
  isBranchSpecificFile,
  isCurrentBranchFile,
  isItemTypeConfigured,
  isSpecificItem,
  SpecificItemsConfig,
} from "../../core/specific_items.ts";
import { getCurrentGitBranch, isGitRepository } from "../../utils/git.ts";
import { Workspace } from "../workspace/workspace.ts";
import { removePathPrefix } from "../../types.ts";
import { listSyncCodebases, SyncCodebase } from "../../utils/codebase.ts";
import {
  generateScriptMetadataInternal,
  getRawWorkspaceDependencies,
  readLockfile,
  workspaceDependenciesPathToLanguageAndFilename,
} from "../../utils/metadata.ts";
import { OpenFlow, NativeServiceName } from "../../../gen/types.gen.ts";
import { pushResource } from "../resource/resource.ts";
import {
  newPathAssigner,
  newRawAppPathAssigner,
  PathAssigner,
} from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";
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
  getDeleteSuffix,
  transformJsonPathToDir,
  getFolderSuffix,
  getFolderSuffixWithSep,
  getNonDottedPaths,
} from "../../utils/resource_folders.ts";

// Merge CLI options with effective settings, preserving CLI flags as overrides
function mergeCliWithEffectiveOptions<
  T extends GlobalOptions & SyncOptions & { repository?: string },
>(cliOpts: T, effectiveOpts: SyncOptions): T {
  // overlay CLI options on top (undefined cliOpts won't override effectiveOpts)
  return Object.assign({}, effectiveOpts, cliOpts) as T;
}

// Resolve effective sync options using branch-based configuration
async function resolveEffectiveSyncOptions(
  workspace: Workspace,
  localConfig: SyncOptions,
  promotion?: string,
  branchOverride?: string,
): Promise<SyncOptions> {
  return await getEffectiveSettings(localConfig, promotion, false, false, branchOverride);
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
        const content = await readFile(localP, "utf-8");
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

**Backend runnable:** Add \`backend/<name>.ts\` (or .py, etc.), then run \`wmill app generate-locks\`

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
    return Object.entries(rec).flatMap(([k, v]) => {
      if (k == "inlineScript" && typeof v == "object") {
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

function ZipFSElement(
  zip: JSZip,
  useYaml: boolean,
  defaultTs: "bun" | "deno",
  resourceTypeToFormatExtension: Record<string, string>,
  resourceTypeToIsFileset: Record<string, boolean>,
  ignoreCodebaseChanges: boolean,
): DynFSElement {
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

    const finalPath = transformPath();

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
              inlineScripts = extractInlineScriptsForFlows(
                flow.value.modules as any,
                {},
                SEP,
                defaultTs,
                undefined, // pathAssigner - let it create one
                { skipInlineScriptSuffix: getNonDottedPaths() },
              );
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
            if (
              parsed["lock"] &&
              parsed["lock"] != "" &&
              parsed["codebase"] == undefined
            ) {
              parsed["lock"] =
                "!inline " +
                removeSuffix(p.replaceAll(SEP, "/"), ".json") +
                ".lock";
            } else if (parsed["lock"] == "") {
              parsed["lock"] = "";
            } else {
              parsed["lock"] = undefined;
            }
            if (ignoreCodebaseChanges && parsed["codebase"]) {
              parsed["codebase"] = undefined;
            }
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

          return useYaml && isJson && kind != "dependencies"
            ? (() => {
                try {
                  return yamlStringify(JSON.parse(content), yamlOptions);
                } catch (error) {
                  log.error(`Failed to parse JSON content at path: ${p}`);
                  throw error;
                }
              })()
            : content;
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
      if (lock && lock != "") {
        r.push({
          isDirectory: false,
          path: removeSuffix(finalPath, ".json") + ".lock",
          async *getChildren() {},
          async getContentText() {
            return lock;
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

type Change = Added | Deleted | Edit;

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
  // Cache git branch at the start to avoid repeated execSync calls per file
  const cachedBranch = branchOverride ?? getCurrentGitBranch() ?? undefined;
  for await (const entry of readDirRecursiveWithIgnore(ignore, els)) {
    // console.log("FOO", entry.path, entry.ignored, entry.isDirectory)
    if (entry.isDirectory || entry.ignored) {
      continue;
    }
    const path = entry.path;
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
        path.endsWith(".sqs_trigger" + ext) ||
        path.endsWith(".gcp_trigger" + ext) ||
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

    // Handle branch-specific files - skip files for other branches
    if (specificItems && isBranchSpecificFile(path)) {
      if (!isCurrentBranchFile(path, cachedBranch)) {
        // Skip branch-specific files for other branches
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

    // Handle branch-specific path mapping after all filtering
    if (cachedBranch && isCurrentBranchFile(path, cachedBranch)) {
      // This is a branch-specific file for current branch
      const currentBranch = cachedBranch;
      const basePath = fromBranchSpecificPath(path, currentBranch);

      // Only use branch-specific files if the item type IS configured as branch-specific
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
    } else if (!isBranchSpecificFile(path)) {
      // This is a regular base file
      if (processedBasePaths.has(path)) {
        // Skip base file, we already processed branch-specific version
        continue;
      }
      // Skip base file if it's configured as branch-specific (expect branch version)
      // Only for LOCAL files - remote workspace only has base paths
      if (!isRemote && isSpecificItem(path, specificItems)) {
        continue;
      }
      map[path] = content;
    }
    // Note: branch-specific files for other branches are already filtered out earlier
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
): Promise<Change[]> {
  const [m1, m2] = els2
    ? await Promise.all([
        elementsToMap(els1, ignore, json, skips, specificItems, branchOverride, isEls1Remote),
        elementsToMap(els2, ignore, json, skips, specificItems, branchOverride, !isEls1Remote),
      ])
    : [await elementsToMap(els1, ignore, json, skips, specificItems, branchOverride, isEls1Remote), {}];

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

  changes.sort((a, b) =>
    getOrderFromPath(a.path) == getOrderFromPath(b.path)
      ? a.path.localeCompare(b.path)
      : getOrderFromPath(a.path) - getOrderFromPath(b.path),
  );

  return changes;
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
    typ == "sqs_trigger" ||
    typ == "gcp_trigger" ||
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
  if (isDirectory) {
    return (
      !p.startsWith("u" + SEP) &&
      !p.startsWith("f" + SEP) &&
      !p.startsWith("g" + SEP) &&
      !p.startsWith("users" + SEP) &&
      !p.startsWith("groups" + SEP) &&
      !p.startsWith("dependencies" + SEP)
    );
  }

  try {
    const typ = getTypeStrFromPath(p);
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
    p == "users" ||
    p == "groups" ||
    p == "dependencies"
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
  const isScript = exts.some((e) => p.endsWith(e));
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

export async function pull(
  opts: GlobalOptions &
    SyncOptions & { repository?: string; promotion?: string; branch?: string },
) {
  const originalCliOpts = { ...opts };
  opts = await mergeConfigWithConfigFile(opts);

  // Validate branch configuration early (skipped when --branch is used)
  try {
    await validateBranchConfiguration(opts, opts.branch);
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

  const workspace = await resolveWorkspace(opts, opts.branch);
  await requireLogin(opts);

  // Resolve effective sync options with branch awareness
  const effectiveOpts = await resolveEffectiveSyncOptions(
    workspace,
    opts,
    opts.promotion,
    opts.branch,
  );

  // Extract specific items configuration before merging overwrites gitBranches
  const specificItems = getSpecificItemsForCurrentBranch(opts, opts.branch);

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
  );

  const local = !opts.stateful
    ? await FSFSElement(process.cwd(), codebases, true)
    : await FSFSElement(path.join(process.cwd(), ".wmill"), [], true);

  const changes = await compareDynFSElement(
    remote,
    local,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    false,
    codebases,
    true,
    specificItems,
    opts.branch,
    true, // els1 (remote) is the remote source
  );

  log.info(
    `remote (${workspace.name}) -> local: ${changes.length} changes to apply`,
  );

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
              branch_specific: true,
              branch_specific_path: getBranchSpecificPath(
                change.path,
                specificItems,
                opts.branch,
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
      prettyChanges(changes, specificItems, opts.branch);
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
      // Determine if this file should be written to a branch-specific path
      let targetPath = change.path;
      if (specificItems && isSpecificItem(change.path, specificItems)) {
        const branchSpecificPath = getBranchSpecificPath(
          change.path,
          specificItems,
          opts.branch,
        );
        if (branchSpecificPath) {
          targetPath = branchSpecificPath;
        }
      }

      const target = path.join(process.cwd(), targetPath);
      const stateTarget = path.join(process.cwd(), ".wmill", targetPath);
      if (change.name === "edited") {
        if (opts.stateful) {
          try {
            const currentLocal = await readFile(target, "utf-8");
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
                ? colors.gray(` (branch-specific override for ${change.path})`)
                : ""
            }`,
          );
        } else if (
          change.path.endsWith(".yaml") ||
          change.path.endsWith(".json")
        ) {
          log.info(
            `Editing ${getTypeStrFromPath(change.path)} ${targetPath}${
              targetPath !== change.path
                ? colors.gray(` (branch-specific override for ${change.path})`)
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
            `Adding ${getTypeStrFromPath(change.path)} ${targetPath}${
              targetPath !== change.path
                ? colors.gray(` (branch-specific override for ${change.path})`)
                : ""
            }`,
          );
        }
        await writeFile(target, change.content, "utf-8");
        log.info(
          `Writing ${getTypeStrFromPath(change.path)} ${targetPath}${
            targetPath !== change.path
              ? colors.gray(` (branch-specific override for ${change.path})`)
              : ""
          }`,
        );
        if (opts.stateful) {
          await copyFile(target, stateTarget);
        }
      } else if (change.name === "deleted") {
        try {
          log.info(
            `Deleting ${getTypeStrFromPath(change.path)} ${change.path}`,
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
      await getRawWorkspaceDependencies();

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
        true,
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
                branch_specific: true,
                branch_specific_path: getBranchSpecificPath(
                  change.path,
                  specificItems,
                  opts.branch,
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
}

function prettyChanges(changes: Change[], specificItems?: SpecificItemsConfig, branchOverride?: string) {
  for (const change of changes) {
    let displayPath = change.path;
    let branchNote = "";

    // Check if this will be written as a branch-specific file
    if (specificItems && isSpecificItem(change.path, specificItems)) {
      const branchSpecificPath = getBranchSpecificPath(
        change.path,
        specificItems,
        branchOverride,
      );
      if (branchSpecificPath) {
        displayPath = branchSpecificPath;
        branchNote = " (branch-specific)";
      }
    }

    if (change.name === "added") {
      log.info(
        colors.green(
          `+ ${getTypeStrFromPath(change.path)} ` +
            displayPath +
            colors.gray(branchNote),
        ),
      );
    } else if (change.name === "deleted") {
      log.info(
        colors.red(
          `- ${getTypeStrFromPath(change.path)} ` +
            displayPath +
            colors.gray(branchNote),
        ),
      );
    } else if (change.name === "edited") {
      log.info(
        colors.yellow(
          `~ ${getTypeStrFromPath(change.path)} ` +
            displayPath +
            colors.gray(branchNote) +
            (change.codebase ? ` (codebase changed)` : ""),
        ),
      );
      if (change.before != change.after) {
        if (change.path.endsWith(".yaml")) {
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

export async function push(
  opts: GlobalOptions & SyncOptions & { repository?: string; branch?: string },
) {
  // Save original CLI options before merging with config file
  const originalCliOpts = { ...opts };

  // Load configuration from wmill.yaml and merge with CLI options
  opts = await mergeConfigWithConfigFile(opts);

  // Validate branch configuration early (skipped when --branch is used)
  try {
    await validateBranchConfiguration(opts, opts.branch);
  } catch (error) {
    if (error instanceof Error && error.message.includes("overrides")) {
      log.error(error.message);
      process.exit(1);
    }
    throw error;
  }

  const workspace = await resolveWorkspace(opts, opts.branch);
  await requireLogin(opts);

  // Resolve effective sync options with branch awareness
  const effectiveOpts = await resolveEffectiveSyncOptions(
    workspace,
    opts,
    opts.promotion,
    opts.branch,
  );

  // Extract specific items configuration BEFORE merging overwrites gitBranches
  const specificItems = getSpecificItemsForCurrentBranch(opts, opts.branch);

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
  );

  const local = await FSFSElement(path.join(process.cwd(), ""), codebases, false);
  const changes = await compareDynFSElement(
    local,
    remote,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    true,
    codebases,
    false,
    specificItems,
    opts.branch,
    false, // els1 (local) is not the remote source
  );

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies();

  const tracker: ChangeTracker = await buildTracker(changes);

  const staleScripts: string[] = [];
  const staleFlows: string[] = [];
  const staleApps: string[] = [];

  for (const change of tracker.scripts) {
    const stale = await generateScriptMetadataInternal(
      change,
      workspace,
      opts,
      true,
      true,
      rawWorkspaceDependencies,
      codebases,
      false,
    );
    if (stale) {
      staleScripts.push(stale);
    }
  }

  if (staleScripts.length > 0) {
    log.info("");
    log.warn(
      "Stale scripts metadata found, you may want to update them using 'wmill script generate-metadata' before pushing:",
    );
    for (const stale of staleScripts) {
      log.warn(stale);
    }

    log.info("");
  }

  for (const change of tracker.flows) {
    const stale = await generateFlowLockInternal(
      change,
      true,
      workspace,
      opts,
      false,
      true,
    );
    if (stale) {
      staleFlows.push(stale);
    }
  }

  if (staleFlows.length > 0) {
    log.warn(
      "Stale flows locks found, you may want to update them using 'wmill flow generate-locks' before pushing:",
    );
    for (const stale of staleFlows) {
      log.warn(stale);
    }
    log.info("");
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
    );
    if (stale) {
      staleApps.push(stale);
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
    );
    if (stale) {
      staleApps.push(stale);
    }
  }

  if (staleApps.length > 0) {
    log.warn(
      "Stale apps locks found, you may want to update them using 'wmill app generate-locks' before pushing:",
    );
    for (const stale of staleApps) {
      log.warn(stale);
    }
    log.info("");
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
      try {
        await stat(path.join("f", folderName, "folder.meta.yaml"));
      } catch {
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
              branch_specific: true,
              branch_specific_path: getBranchSpecificPath(
                change.path,
                specificItems,
                opts.branch,
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
      prettyChanges(changes, specificItems, opts.branch);
    }

    if (opts.dryRun) {
      log.info(colors.gray(`Dry run complete.`));
      return;
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
    const groupedChangesArray = Array.from(groupedChanges.entries());
    log.info(
      `found changes for ${
        groupedChangesArray.length
      } items with a total of ${groupedChangesArray.reduce(
        (acc, [_, changes]) => acc + changes.length,
        0,
      )} files to process`,
    );
    if (parallelizationFactor > 1) {
      log.info(`Parallelizing ${parallelizationFactor} changes at a time`);
    }

    // Create a pool of workers that processes items as they become available
    const pool = new Set();
    const queue = [...groupedChangesArray];
    // Cache git branch at the start to avoid repeated execSync calls per change
    const cachedBranchForPush = opts.branch || (isGitRepository() ? getCurrentGitBranch() : null);

    while (queue.length > 0 || pool.size > 0) {
      // Fill the pool until we reach parallelizationFactor
      while (pool.size < parallelizationFactor && queue.length > 0) {
        let [_basePath, changes] = queue.shift()!;
        const promise = (async () => {
          const alreadySynced: string[] = [];
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
                )
              ) {
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
                    await readFile(resourceFilePath, "utf-8"),
                  );

                  // For branch-specific resources, push to the base path on the workspace server
                  // This ensures branch-specific files are stored with their base names in the workspace
                  let serverPath = resourceFilePath;
                  const currentBranch = cachedBranchForPush;

                  if (currentBranch && isBranchSpecificFile(resourceFilePath)) {
                    serverPath = fromBranchSpecificPath(
                      resourceFilePath,
                      currentBranch,
                    );
                  }

                  await pushResource(
                    workspace.workspaceId,
                    serverPath,
                    undefined,
                    newObj,
                    resourceFilePath,
                  );
                  if (stateTarget) {
                    await writeFile(stateTarget, change.after, "utf-8");
                  }
                  continue;
                }
              }
              if (isFilesetResource(change.path)) {
                const resourceFilePath = await findFilesetResourceFile(change.path);
                if (!alreadySynced.includes(resourceFilePath)) {
                  alreadySynced.push(resourceFilePath);

                  const newObj = parseFromPath(
                    resourceFilePath,
                    await readFile(resourceFilePath, "utf-8"),
                  );

                  let serverPath = resourceFilePath;
                  const currentBranch = cachedBranchForPush;

                  if (currentBranch && isBranchSpecificFile(resourceFilePath)) {
                    serverPath = fromBranchSpecificPath(
                      resourceFilePath,
                      currentBranch,
                    );
                  }

                  await pushResource(
                    workspace.workspaceId,
                    serverPath,
                    undefined,
                    newObj,
                    resourceFilePath,
                  );
                  if (stateTarget) {
                    await writeFile(stateTarget, change.after, "utf-8");
                  }
                  continue;
                }
              }
              const oldObj = parseFromPath(change.path, change.before);
              const newObj = parseFromPath(change.path, change.after);

              // Check if this is a branch-specific item and get the original branch-specific path
              let originalBranchSpecificPath: string | undefined;
              if (specificItems && isSpecificItem(change.path, specificItems)) {
                originalBranchSpecificPath = getBranchSpecificPath(
                  change.path,
                  specificItems,
                  opts.branch,
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
                originalBranchSpecificPath,
              );

              if (stateTarget) {
                await writeFile(stateTarget, change.after, "utf-8");
              }
            } else if (change.name === "added") {
              if (
                change.path.endsWith(".script.json") ||
                change.path.endsWith(".script.yaml") ||
                change.path.endsWith(".lock") ||
                isFileResource(change.path) ||
                isFilesetResource(change.path)
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
                )
              ) {
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
              // For branch-specific items, we read from branch-specific files but push to base server paths
              let localFilePath = change.path;
              if (specificItems && isSpecificItem(change.path, specificItems)) {
                const branchSpecificPath = getBranchSpecificPath(
                  change.path,
                  specificItems,
                  opts.branch,
                );
                if (branchSpecificPath) {
                  localFilePath = branchSpecificPath;
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
              );

              if (stateTarget) {
                await writeFile(stateTarget, change.content, "utf-8");
              }
            } else if (change.name === "deleted") {
              if (change.path.endsWith(".lock")) {
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
                case "resource":
                  await wmill.deleteResource({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".resource.json"),
                  });
                  break;
                case "resource-type":
                  await wmill.deleteResourceType({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".resource-type.json"),
                  });
                  break;
                case "flow":
                  await wmill.deleteFlowByPath({
                    workspace: workspaceId,
                    path: removeSuffix(target, getDeleteSuffix("flow", "json")),
                  });
                  break;
                case "app":
                  await wmill.deleteApp({
                    workspace: workspaceId,
                    path: removeSuffix(target, getDeleteSuffix("app", "json")),
                  });
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
                case "variable":
                  await wmill.deleteVariable({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".variable.json"),
                  });
                  break;
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
            }
          }
        })();

        pool.add(promise);
        // Remove from pool when complete
        promise.then(() => pool.delete(promise));
      }

      // Wait for at least one task to complete before continuing
      if (pool.size > 0) {
        await Promise.race(pool);
      }
    }
    if (opts.jsonOutput) {
      const result = {
        success: true,
        message: `All ${changes.length} changes pushed to the remote workspace ${workspace.workspaceId} named ${workspace.name}`,
        changes: changes.map((change) => ({
          type: change.name,
          path: change.path,
          ...(change.name === "edited" && change.codebase
            ? { codebase_changed: true }
            : {}),
          ...(specificItems && isSpecificItem(change.path, specificItems)
            ? {
                branch_specific: true,
                branch_specific_path: getBranchSpecificPath(
                  change.path,
                  specificItems,
                  opts.branch,
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
  } else if (opts.jsonOutput) {
    console.log(
      JSON.stringify(
        { success: true, message: "No changes to push", total: 0 },
        null,
        2,
      ),
    );
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
    "--branch <branch:string>",
    "Override the current git branch (works even outside a git repository)",
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
    "--branch <branch:string>",
    "Override the current git branch (works even outside a git repository)",
  )
  .option("--lint", "Run lint validation before pushing")
  .option(
    "--locks-required",
    "Fail if scripts or flow inline scripts that need locks have no locks",
  )
  .action(push as any);

export default command;
