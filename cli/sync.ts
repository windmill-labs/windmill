import { fetchVersion, requireLogin, resolveWorkspace } from "./context.ts";
import {
  colors,
  Command,
  Confirm,
  Input,
  ensureDir,
  minimatch,
  JSZip,
  path,
  log,
  yamlStringify,
  yamlParseContent,
  SEP,
} from "./deps.ts";
import * as wmill from "./gen/services.gen.ts";

import {
  getTypeStrFromPath,
  GlobalOptions,
  parseFromPath,
  pushObj,
  showConflict,
  showDiff,
} from "./types.ts";
import { downloadZip } from "./pull.ts";

import {
  exts,
  findContentFile,
  findGlobalDeps,
  findResourceFile,
  handleScriptMetadata,
  removeExtensionToPath,
} from "./script.ts";

import { handleFile } from "./script.ts";
import { deepEqual, isFileResource } from "./utils.ts";
import { SyncOptions, mergeConfigWithConfigFile, readConfigFile, getEffectiveSettings } from "./conf.ts";
import { removePathPrefix } from "./types.ts";
import { SyncCodebase, listSyncCodebases } from "./codebase.ts";
import {
  generateFlowLockInternal,
  generateScriptMetadataInternal,
  readLockfile,
} from "./metadata.ts";
import { FlowModule, OpenFlow, RawScript } from "./gen/types.gen.ts";
import { pushResource } from "./resource.ts";

// Repository selection helper (similar to gitsync-settings)
async function selectRepositoryInteractively(
  repositories: Array<{ git_repo_resource_path: string }>,
  operation: string
): Promise<string> {
  if (repositories.length === 0) {
    throw new Error("No git-sync repositories configured in workspace");
  }

  if (repositories.length === 1) {
    const repoPath = repositories[0].git_repo_resource_path.replace(/^\$res:/, "");
    log.info(colors.gray(`Using repository: ${repoPath}`));
    return repoPath;
  }

  // Check if we're in a non-interactive environment
  const isInteractive = Deno.stdin.isTerminal() && Deno.stdout.isTerminal();
  
  if (!isInteractive) {
    const repoPaths = repositories.map(r => r.git_repo_resource_path.replace(/^\$res:/, ""));
    throw new Error(`Multiple repositories found: ${repoPaths.join(', ')}. Use --repository to specify which one to ${operation}.`);
  }

  // Import Select dynamically to avoid dependency issues
  const { Select } = await import("./deps.ts");

  console.log(`\nMultiple repositories found. Please select which repository to ${operation}:\n`);

  const selectedRepo = await Select.prompt({
    message: `Select repository for ${operation}:`,
    options: repositories.map((repo, index) => {
      const displayPath = repo.git_repo_resource_path.replace(/^\$res:/, "");
      return {
        name: `${index + 1}. ${displayPath}`,
        value: displayPath
      };
    })
  });

  return selectedRepo;
}

// Resolve effective sync options with repository awareness
async function resolveEffectiveSyncOptions(
  workspace: { workspaceId: string; name?: string; remote?: string },
  repositoryPath?: string
): Promise<SyncOptions> {
  const localConfig = await readConfigFile();

  // Check if config has overrides that apply to the current workspace
  const applicableOverrides = findApplicableOverrides(localConfig, workspace);

  if (applicableOverrides.length === 0) {
    // No applicable overrides - use legacy behavior for backward compatibility
    return await mergeConfigWithConfigFile({});
  }

  // Determine repository from overrides or explicit parameter
  let selectedRepository = repositoryPath;

  if (!selectedRepository) {
    if (applicableOverrides.length === 1) {
      // Single applicable override - use its repository
      selectedRepository = applicableOverrides[0].repository;
    } else {
      // Multiple applicable overrides - require explicit selection
      const repositoryChoices = applicableOverrides.map((override, index) => ({
        name: `${index + 1}. ${override.repository}`,
        value: override.repository
      }));

      const isInteractive = Deno.stdin.isTerminal() && Deno.stdout.isTerminal();
      if (!isInteractive) {
        const repoList = applicableOverrides.map(o => o.repository).join(', ');
        throw new Error(`Multiple repository overrides found for workspace "${workspace.name || workspace.workspaceId}": ${repoList}. Use --repository to specify which one to sync.`);
      }

      selectedRepository = await Input.prompt({
        message: "Select repository override to use for sync:",
        type: Input.type.select,
        options: repositoryChoices
      });
    }
  }

  // Use hierarchical resolution with workspace + repository
  if (!selectedRepository) {
    throw new Error("Repository could not be determined. This should not happen.");
  }
  return getEffectiveSettings(localConfig, workspace.workspaceId, selectedRepository, workspace);
}

// Helper function to find overrides that apply to the current workspace
function findApplicableOverrides(
  config: SyncOptions,
  workspace: { workspaceId: string; name?: string; remote?: string }
): Array<{ key: string; repository: string }> {
  if (!config.overrides) {
    return [];
  }

  const applicableOverrides: Array<{ key: string; repository: string }> = [];
  
  // Generate possible workspace prefixes in order of preference
  const possiblePrefixes = workspace.name ? [
    `${workspace.name}:`,                          // Primary: "localhost_test:"
    `${workspace.remote}:${workspace.workspaceId}:`, // Disambiguated: "http://localhost:8000/:test:"
    `${workspace.workspaceId}:`,                   // Legacy: "test:"
  ] : [
    `${workspace.workspaceId}:`,                   // Legacy only
  ];

  // Find all override keys that match current workspace
  for (const [key, value] of Object.entries(config.overrides)) {
    // Skip workspace-level overrides (ending with :*)
    if (key.endsWith(':*')) {
      continue;
    }

    // Check if this override key applies to the current workspace
    for (const prefix of possiblePrefixes) {
      if (key.startsWith(prefix)) {
        const repository = key.substring(prefix.length);
        if (repository && repository.length > 0) {
          applicableOverrides.push({ key, repository });
          break; // Found match, no need to check other prefixes
        }
      }
    }
  }

  // Remove duplicates based on repository (prefer earlier prefixes)
  const uniqueOverrides = [];
  const seenRepositories = new Set();
  for (const override of applicableOverrides) {
    if (!seenRepositories.has(override.repository)) {
      seenRepositories.add(override.repository);
      uniqueOverrides.push(override);
    }
  }

  return uniqueOverrides;
}

type DynFSElement = {
  isDirectory: boolean;
  path: string;
  // getContentBytes(): Promise<Uint8Array>;
  getContentText(): Promise<string>;
  getChildren(): AsyncIterable<DynFSElement>;
};

export function findCodebase(
  path: string,
  codebases: SyncCodebase[]
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
  ignoreCodebaseChanges: boolean
): Promise<string> {
  const isScript = path.endsWith(".script.yaml");
  if (!isScript) {
    return content;
  }
  let isTs = true;
  const replacedPath = path.replace(".script.yaml", ".ts");
  try {
    await Deno.stat(replacedPath);
  } catch {
    isTs = false;
  }
  if (!isTs) {
    return content;
  }
  if (isTs) {
    const c = findCodebase(replacedPath, codebases);
    if (c) {
      const parsed: any = yamlParseContent(path, content);
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
          `Expected local yaml ${path} to be an object, found: ${content} instead`
        );
      }
    }
  }
  return content;
}

export async function FSFSElement(
  p: string,
  codebases: SyncCodebase[],
  ignoreCodebaseChanges: boolean
): Promise<DynFSElement> {
  function _internal_element(
    localP: string,
    isDir: boolean,
    codebases: SyncCodebase[]
  ): DynFSElement {
    return {
      isDirectory: isDir,
      path: localP.substring(p.length + 1),
      async *getChildren(): AsyncIterable<DynFSElement> {
        if (!isDir) return [];
        try {
          for await (const e of Deno.readDir(localP)) {
            yield _internal_element(
              path.join(localP, e.name),
              e.isDirectory,
              codebases
            );
          }
        } catch (e) {
          log.warn(`Error reading dir: ${localP}, ${e}`);
        }
      },
      // async getContentBytes(): Promise<Uint8Array> {
      //   return await Deno.readFile(localP);
      // },
      async getContentText(): Promise<string> {
        const content = await Deno.readTextFile(localP);
        const itemPath = localP.substring(p.length + 1);
        const r = await addCodebaseDigestIfRelevant(
          itemPath,
          content,
          codebases,
          ignoreCodebaseChanges
        );
        return r;
      },
    };
  }
  return _internal_element(p, (await Deno.stat(p)).isDirectory, codebases);
}

function prioritizeName(name: string): string {
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

  return name;
}

export const yamlOptions = {
  sortKeys: (a: any, b: any) => {
    return prioritizeName(a).localeCompare(prioritizeName(b));
  },
  noCompatMode: true,
  noRefs: true,
  skipInvalid: true,
};

export interface InlineScript {
  path: string;
  content: string;
}

export function extractInlineScriptsForFlows(
  modules: FlowModule[],
  pathAssigner: PathAssigner
): InlineScript[] {
  return modules.flatMap((m) => {
    if (m.value.type == "rawscript") {
      const [basePath, ext] = pathAssigner.assignPath(
        m.summary,
        m.value.language
      );
      const path = basePath + ext;
      const content = m.value.content;
      const r = [{ path: path, content: content }];
      m.value.content = "!inline " + path.replaceAll(SEP, "/");
      const lock = m.value.lock;
      if (lock && lock != "") {
        const lockPath = basePath + "lock";
        m.value.lock = "!inline " + lockPath.replaceAll(SEP, "/");
        r.push({ path: lockPath, content: lock });
      }
      return r;
    } else if (m.value.type == "forloopflow") {
      return extractInlineScriptsForFlows(m.value.modules, pathAssigner);
    } else if (m.value.type == "branchall") {
      return m.value.branches.flatMap((b) =>
        extractInlineScriptsForFlows(b.modules, pathAssigner)
      );
    } else if (m.value.type == "whileloopflow") {
      return extractInlineScriptsForFlows(m.value.modules, pathAssigner);
    } else if (m.value.type == "branchone") {
      return [
        ...m.value.branches.flatMap((b) =>
          extractInlineScriptsForFlows(b.modules, pathAssigner)
        ),
        ...extractInlineScriptsForFlows(m.value.default, pathAssigner),
      ];
    } else {
      return [];
    }
  });
}

interface PathAssigner {
  assignPath(summary: string | undefined, language: string): [string, string];
}
const INLINE_SCRIPT = "inline_script";

export function extractInlineScriptsForApps(
  rec: any,
  pathAssigner: PathAssigner
): InlineScript[] {
  if (!rec) {
    return [];
  }
  if (typeof rec == "object") {
    return Object.entries(rec).flatMap(([k, v]) => {
      if (k == "inlineScript" && typeof v == "object") {
        const o: Record<string, any> = v as any;
        const name = rec["name"];
        const [basePath, ext] = pathAssigner.assignPath(name, o["language"]);
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
        return r;
      } else {
        return extractInlineScriptsForApps(v, pathAssigner);
      }
    });
  }
  return [];
}

export function newPathAssigner(defaultTs: "bun" | "deno"): PathAssigner {
  let counter = 0;
  const seen_names = new Set<string>();
  function assignPath(
    summary: string | undefined,
    language: RawScript["language"] | "frontend" | "bunnative"
  ): [string, string] {
    let name;

    name = summary?.toLowerCase()?.replaceAll(" ", "_") ?? "";

    let original_name = name;

    if (name == "") {
      original_name = INLINE_SCRIPT;
      name = `${INLINE_SCRIPT}_0`;
    }

    while (seen_names.has(name)) {
      counter++;
      name = `${original_name}_${counter}`;
    }
    seen_names.add(name);

    let ext;
    if (language == "python3") ext = "py";
    else if (language == defaultTs || language == "bunnative") ext = "ts";
    else if (language == "bun") ext = "bun.ts";
    else if (language == "deno") ext = "deno.ts";
    else if (language == "go") ext = "go";
    else if (language == "bash") ext = "sh";
    else if (language == "powershell") ext = "ps1";
    else if (language == "postgresql") ext = "pg.sql";
    else if (language == "mysql") ext = "my.sql";
    else if (language == "bigquery") ext = "bq.sql";
    else if (language == "oracledb") ext = "odb.sql";
    else if (language == "snowflake") ext = "sf.sql";
    else if (language == "mssql") ext = "ms.sql";
    else if (language == "graphql") ext = "gql";
    else if (language == "nativets") ext = "native.ts";
    else if (language == "frontend") ext = "frontend.js";
    else if (language == "php") ext = "php";
    else if (language == "rust") ext = "rs";
    else if (language == "csharp") ext = "cs";
    else if (language == "nu") ext = "nu";
    else if (language == "ansible") ext = "playbook.yml";
    else if (language == "java") ext = "java";
    else if (language == "duckdb") ext = "duckdb.sql";
    // for related places search: ADD_NEW_LANG
    else ext = "no_ext";

    return [`${name}.inline_script.`, ext];
  }

  return { assignPath };
}
function ZipFSElement(
  zip: JSZip,
  useYaml: boolean,
  defaultTs: "bun" | "deno",
  resourceTypeToFormatExtension: Record<string, string>,
  ignoreCodebaseChanges: boolean
): DynFSElement {
  async function _internal_file(
    p: string,
    f: JSZip.JSZipObject
  ): Promise<DynFSElement[]> {
    const kind: "flow" | "app" | "script" | "resource" | "other" = p.endsWith(
      "flow.json"
    )
      ? "flow"
      : p.endsWith("app.json")
      ? "app"
      : p.endsWith("script.json")
      ? "script"
      : p.endsWith("resource.json")
      ? "resource"
      : "other";

    const isJson = p.endsWith(".json");

    function transformPath() {
      if (kind == "flow") {
        return p.replace("flow.json", "flow");
      } else if (kind == "app") {
        return p.replace("app.json", "app");
      } else {
        return useYaml && isJson ? p.replaceAll(".json", ".yaml") : p;
      }
    }

    const finalPath = transformPath();
    const r = [
      {
        isDirectory: kind == "flow" || kind == "app",
        path: finalPath,
        async *getChildren(): AsyncIterable<DynFSElement> {
          if (kind == "flow") {
            const flow: OpenFlow = JSON.parse(await f.async("text"));
            const inlineScripts = extractInlineScriptsForFlows(
              flow.value.modules,
              newPathAssigner(defaultTs)
            );
            for (const s of inlineScripts) {
              yield {
                isDirectory: false,
                path: path.join(finalPath, s.path),
                async *getChildren() {},
                // deno-lint-ignore require-await
                async getContentText() {
                  return s.content;
                },
              };
            }

            yield {
              isDirectory: false,
              path: path.join(finalPath, "flow.yaml"),
              async *getChildren() {},
              // deno-lint-ignore require-await
              async getContentText() {
                return yamlStringify(flow, yamlOptions);
              },
            };
          } else if (kind == "app") {
            const app = JSON.parse(await f.async("text"));
            const inlineScripts = extractInlineScriptsForApps(
              app?.["value"],
              newPathAssigner(defaultTs)
            );
            for (const s of inlineScripts) {
              yield {
                isDirectory: false,
                path: path.join(finalPath, s.path),
                async *getChildren() {},
                // deno-lint-ignore require-await
                async getContentText() {
                  return s.content;
                },
              };
            }

            yield {
              isDirectory: false,
              path: path.join(finalPath, "app.yaml"),
              async *getChildren() {},
              // deno-lint-ignore require-await
              async getContentText() {
                return yamlStringify(app, yamlOptions);
              },
            };
          }
        },

        async getContentText(): Promise<string> {
          const content = await f.async("text");

          if (kind == "script") {
            const parsed = JSON.parse(content);
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
            const parsed = JSON.parse(content);
            const formatExtension =
              resourceTypeToFormatExtension[parsed["resource_type"]];

            if (formatExtension) {
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

          return useYaml && isJson
            ? yamlStringify(JSON.parse(content), yamlOptions)
            : content;
        },
      },
    ];
    if (kind == "script") {
      const content = await f.async("text");
      const parsed = JSON.parse(content);
      const lock = parsed["lock"];
      if (lock && lock != "") {
        r.push({
          isDirectory: false,
          path: removeSuffix(finalPath, ".json") + ".lock",
          async *getChildren() {},
          // deno-lint-ignore require-await
          async getContentText() {
            return lock;
          },
        });
      }
    }
    if (kind == "resource") {
      const content = await f.async("text");
      const parsed = JSON.parse(content);
      const formatExtension =
        resourceTypeToFormatExtension[parsed["resource_type"]];

      if (formatExtension) {
        const fileContent: string = parsed["value"]["content"];
        if (typeof fileContent === "string") {
          r.push({
            isDirectory: false,
            path:
              removeSuffix(finalPath, ".resource.json") +
              ".resource.file." +
              formatExtension,
            async *getChildren() {},
            // deno-lint-ignore require-await
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
      // // deno-lint-ignore require-await
      // async getContentBytes(): Promise<Uint8Array> {
      //   throw new Error("Cannot get content of folder");
      // },
      // deno-lint-ignore require-await
      async getContentText(): Promise<string> {
        throw new Error("Cannot get content of folder");
      },
    };
  }
  return _internal_folder("." + SEP, zip);
}

export async function* readDirRecursiveWithIgnore(
  ignore: (path: string, isDirectory: boolean) => boolean,
  root: DynFSElement
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
    // console.log(e.path);
    yield e;
    for await (const e2 of e.c()) {
      if (e2.isDirectory) {
        const dirName = e2.path.split(SEP).pop();
        if (dirName == "node_modules" || dirName?.startsWith(".")) {
          continue;
        }
      }
      // console.log(e2.path);
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
  skips: Skips
): Promise<{ [key: string]: string }> {
  const map: { [key: string]: string } = {};
  for await (const entry of readDirRecursiveWithIgnore(ignore, els)) {
    if (entry.isDirectory || entry.ignored) continue;
    const path = entry.path;
    if (json && path.endsWith(".yaml") && !isFileResource(path)) continue;
    if (!json && path.endsWith(".json") && !isFileResource(path)) continue;
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
        path.endsWith(".gcp_trigger" + ext))
    )
      continue;
    if (!skips.includeUsers && path.endsWith(".user" + ext)) continue;
    if (!skips.includeGroups && path.endsWith(".group" + ext)) continue;
    if (!skips.includeSettings && path === "settings" + ext) continue;
    if (!skips.includeKey && path === "encryption_key") continue;
    if (skips.skipResources && path.endsWith(".resource" + ext)) continue;
    if (skips.skipResourceTypes && path.endsWith(".resource-type" + ext))
      continue;

    if (skips.skipVariables && path.endsWith(".variable" + ext)) continue;

    if (skips.skipResources && isFileResource(path)) continue;

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
        // for related places search: ADD_NEW_LANG
      ].includes(path.split(".").pop() ?? "") &&
      !isFileResource(path)
    )
      continue;
    const content = await entry.getContentText();

    if (skips.skipSecrets && path.endsWith(".variable" + ext)) {
      try {
        let o;
        if (json) {
          o = JSON.parse(content);
        } else {
          o = yamlParseContent(path, content);
        }
        if (o["is_secret"]) {
          continue;
        }
      } catch (e) {
        log.warn(`Error reading variable ${path} to check for secrets`);
      }
    }
    map[entry.path] = content;
  }
  return map;
}

export interface Skips {
  skipVariables?: boolean | undefined;
  skipResources?: boolean | undefined;
  skipResourceTypes?: boolean | undefined;
  skipSecrets?: boolean | undefined;
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
  ignoreCodebaseChanges: boolean
): Promise<Change[]> {
  const [m1, m2] = els2
    ? await Promise.all([
        elementsToMap(els1, ignore, json, skips),
        elementsToMap(els2, ignore, json, skips),
      ])
    : [await elementsToMap(els1, ignore, json, skips), {}];

  const changes: Change[] = [];

  function parseYaml(k: string, v: string) {
    if (k.endsWith(".script.yaml")) {
      const o: any = yamlParseContent(k, v);
      if (typeof o == "object") {
        if (Array.isArray(o?.["lock"])) {
          o["lock"] = o["lock"].join("\n");
        }
        if (o["is_template"] != undefined) {
          delete o["is_template"];
        }
      }
      return o;
    } else if (k.endsWith(".app.yaml")) {
      const o: any = yamlParseContent(k, v);
      const o2 = o["policy"];

      if (typeof o2 == "object") {
        if (o2["on_behalf_of"] != undefined) {
          delete o2["on_behalf_of"];
        }
        if (o2["on_behalf_of_email"] != undefined) {
          delete o2["on_behalf_of_email"];
        }
      }
      return o;
    } else {
      return yamlParseContent(k, v);
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
      changes.push({ name: "added", path: k, content: v });
    } else {
      if (m2[k] == v) {
        continue;
      } else if (k.endsWith(".json")) {
        if (deepEqual(JSON.parse(v), JSON.parse(m2[k]))) {
          continue;
        }
      } else if (k.endsWith(".yaml")) {
        const before = parseYaml(k, m2[k]);
        const after = parseYaml(k, v);
        if (deepEqual(before, after)) {
          continue;
        }
        if (!ignoreCodebaseChanges) {
          if (before.codebase != undefined) {
            delete before.codebase;
            m2[k] = yamlStringify(before, yamlOptions);
          }
          if (after.codebase != undefined) {
            if (before.codebase != after.codebase) {
              codebaseChanges[k] = after.codebase;
            }
            delete after.codebase;
            v = yamlStringify(after, yamlOptions);
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
        let o = parseYaml(k, m2[k]);
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
          (c) => c.path == tsFile && (c.name == "edited" || c.name == "deleted")
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
      : getOrderFromPath(a.path) - getOrderFromPath(b.path)
  );

  return changes;
}

function getOrderFromPath(p: string) {
  const typ = getTypeStrFromPath(p);
  if (typ == "settings") {
    return 0;
  } else if (typ == "folder") {
    return 1;
  } else if (typ == "resource-type") {
    return 2;
  } else if (typ == "resource") {
    return 3;
  } else if (typ == "script") {
    return 4;
  } else if (typ == "flow") {
    return 5;
  } else if (typ == "app") {
    return 6;
  } else if (typ == "schedule") {
    return 7;
  } else if (
    typ == "http_trigger" ||
    typ == "websocket_trigger" ||
    typ == "kafka_trigger" ||
    typ == "nats_trigger" ||
    typ == "postgres_trigger" ||
    typ == "mqtt_trigger" ||
    typ == "sqs_trigger" ||
    typ == "gcp_trigger"
  ) {
    return 8;
  } else if (typ == "variable") {
    return 9;
  } else if (typ == "user") {
    return 10;
  } else if (typ == "group") {
    return 11;
  } else if (typ == "encryption_key") {
    return 12;
  } else {
    return 13;
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
      !p.startsWith("groups" + SEP)
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
        !p.startsWith("groups" + SEP)
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
    p == "groups"
  );
};

export async function ignoreF(wmillconf: {
  includes?: string[];
  excludes?: string[];
  extraIncludes?: string[];
  skipResourceTypes?: boolean;
  json?: boolean;
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
            wmillconf.extraIncludes.some((i) => minimatch(file, i)))
        );
      },
    };
  }

  try {
    await Deno.stat(".wmillignore");
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
}

// deno-lint-ignore no-inner-declarations
async function addToChangedIfNotExists(p: string, tracker: ChangeTracker) {
  const isScript = exts.some((e) => p.endsWith(e));
  if (isScript) {
    if (p.includes(".flow" + SEP)) {
      const folder = p.substring(0, p.indexOf(".flow" + SEP)) + ".flow" + SEP;
      if (!tracker.flows.includes(folder)) {
        tracker.flows.push(folder);
      }
    } else if (p.includes(".app" + SEP)) {
      const folder = p.substring(0, p.indexOf(".app" + SEP)) + ".app" + SEP;
      if (!tracker.apps.includes(folder)) {
        tracker.apps.push(folder);
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
  };
  for (const change of changes) {
    if (change.name == "added" || change.name == "edited") {
      await addToChangedIfNotExists(change.path, tracker);
    }
  }
  return tracker;
}

export async function pull(opts: GlobalOptions & SyncOptions & { repository?: string }) {
  if (opts.stateful) {
    await ensureDir(path.join(Deno.cwd(), ".wmill"));
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Resolve effective sync options with repository awareness
  const effectiveOpts = await resolveEffectiveSyncOptions(workspace, opts.repository);
  
  // Merge CLI flags with resolved settings (CLI flags take precedence only for explicit overrides)
  // Start with effective options from config, then overlay only explicitly provided CLI flags
  const mergedOpts = Object.assign({}, effectiveOpts);
  
  // Always preserve these operational CLI flags
  if (opts.dryRun !== undefined) mergedOpts.dryRun = opts.dryRun;
  if (opts.yes !== undefined) mergedOpts.yes = opts.yes;
  if (opts.stateful !== undefined) mergedOpts.stateful = opts.stateful;
  if (opts.skipPull !== undefined) mergedOpts.skipPull = opts.skipPull;
  if (opts.failConflicts !== undefined) mergedOpts.failConflicts = opts.failConflicts;
  if (opts.plainSecrets !== undefined) mergedOpts.plainSecrets = opts.plainSecrets;
  if (opts.json !== undefined) mergedOpts.json = opts.json;
  if (opts.message !== undefined) mergedOpts.message = opts.message;
  if (opts.parallel !== undefined) mergedOpts.parallel = opts.parallel;
  if (opts.jsonOutput !== undefined) mergedOpts.jsonOutput = opts.jsonOutput;
  if (opts.repository !== undefined) mergedOpts.repository = opts.repository;
  
  opts = mergedOpts;

  const codebases = await listSyncCodebases(opts);

  log.info(
    colors.gray(
      "Computing the files to update locally to match remote (taking wmill.yaml into account)"
    )
  );

  let resourceTypeToFormatExtension: Record<string, string> = {};
  try {
    resourceTypeToFormatExtension = (await wmill.fileResourceTypeToFileExtMap({
      workspace: workspace.workspaceId,
    })) as Record<string, string>;
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
      opts.defaultTs
    ))!,
    !opts.json,
    opts.defaultTs ?? "bun",
    resourceTypeToFormatExtension,
    true
  );
  const local = !opts.stateful
    ? await FSFSElement(Deno.cwd(), codebases, true)
    : await FSFSElement(path.join(Deno.cwd(), ".wmill"), [], true);
  const changes = await compareDynFSElement(
    remote,
    local,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    false,
    codebases,
    true
  );

  log.info(
    `remote (${workspace.name}) -> local: ${changes.length} changes to apply`
  );
  
  // Handle JSON output for dry-run
  if (opts.dryRun && opts.jsonOutput) {
    const result = {
      success: true,
      changes: changes.map(change => ({
        type: change.name,
        path: change.path,
        ...(change.name === "edited" && change.codebase ? { codebase_changed: true } : {})
      })),
      total: changes.length
    };
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  
  if (changes.length > 0) {
    if (!opts.jsonOutput) {
      prettyChanges(changes);
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
      const target = path.join(Deno.cwd(), change.path);
      const stateTarget = path.join(Deno.cwd(), ".wmill", change.path);
      if (change.name === "edited") {
        if (opts.stateful) {
          try {
            const currentLocal = await Deno.readTextFile(target);
            if (
              currentLocal !== change.before &&
              currentLocal !== change.after
            ) {
              log.info(
                colors.red(
                  `Conflict detected on ${change.path}\nBoth local and remote have been modified.`
                )
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
                    `Override local version with remote since --yes was passed and no --fail-conflicts.`
                  )
                );
              } else {
                showConflict(change.path, currentLocal, change.after);
                if (
                  await Confirm.prompt(
                    "Preserve local (push to change remote and avoid seeing this again)?"
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
          log.info(`Editing script content of ${change.path}`);
        } else if (
          change.path.endsWith(".yaml") ||
          change.path.endsWith(".json")
        ) {
          log.info(`Editing ${getTypeStrFromPath(change.path)} ${change.path}`);
        }
        await Deno.writeTextFile(target, change.after);

        if (opts.stateful) {
          await ensureDir(path.dirname(stateTarget));
          await Deno.copyFile(target, stateTarget);
        }
      } else if (change.name === "added") {
        await ensureDir(path.dirname(target));
        if (opts.stateful) {
          await ensureDir(path.dirname(stateTarget));
          log.info(`Adding ${getTypeStrFromPath(change.path)} ${change.path}`);
        }
        await Deno.writeTextFile(target, change.content);
        log.info(`Writing ${getTypeStrFromPath(change.path)} ${change.path}`);
        if (opts.stateful) {
          await Deno.copyFile(target, stateTarget);
        }
      } else if (change.name === "deleted") {
        try {
          log.info(
            `Deleting ${getTypeStrFromPath(change.path)} ${change.path}`
          );
          await Deno.remove(target);
          if (opts.stateful) {
            await Deno.remove(stateTarget);
          }
        } catch {
          if (opts.stateful) {
            await Deno.remove(stateTarget);
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
`)
        );
        Deno.exit(1);
      }
    }
    log.info("All local changes pulled, now updating wmill-lock.yaml");
    await readLockfile(); // ensure wmill-lock.yaml exists
    const globalDeps = await findGlobalDeps();

    const tracker: ChangeTracker = await buildTracker(changes);

    for (const change of tracker.scripts) {
      await generateScriptMetadataInternal(
        change,
        workspace,
        opts,
        false,
        true,
        globalDeps,
        codebases,
        true
      );
    }
    for (const change of tracker.flows) {
      log.info(`Updating lock for flow ${change}`);
      await generateFlowLockInternal(change, false, workspace, true);
    }
    if (tracker.apps.length > 0) {
      log.info(
        `Apps ${tracker.apps.join(
          ", "
        )} scripts were changed but ignoring for now`
      );
    }
    if (opts.jsonOutput) {
      const result = {
        success: true,
        message: `All ${changes.length} changes applied locally and wmill-lock.yaml updated`,
        changes: changes.map(change => ({
          type: change.name,
          path: change.path,
          ...(change.name === "edited" && change.codebase ? { codebase_changed: true } : {})
        })),
        total: changes.length
      };
      console.log(JSON.stringify(result, null, 2));
    } else {
      log.info(
        colors.bold.green.underline(
          `\nDone! All ${changes.length} changes applied locally and wmill-lock.yaml updated.`
        )
      );
    }
  } else if (opts.jsonOutput) {
    console.log(JSON.stringify({ success: true, message: "No changes to apply", total: 0 }, null, 2));
  }
}

function prettyChanges(changes: Change[]) {
  for (const change of changes) {
    if (change.name === "added") {
      log.info(
        colors.green(`+ ${getTypeStrFromPath(change.path)} ` + change.path)
      );
    } else if (change.name === "deleted") {
      log.info(
        colors.red(`- ${getTypeStrFromPath(change.path)} ` + change.path)
      );
    } else if (change.name === "edited") {
      log.info(
        colors.yellow(
          `~ ${getTypeStrFromPath(change.path)} ` +
            change.path +
            (change.codebase ? ` (codebase changed)` : "")
        )
      );
      if (change.before != change.after) {
        showDiff(change.before, change.after);
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

export async function push(opts: GlobalOptions & SyncOptions & { repository?: string }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Resolve effective sync options with repository awareness
  const effectiveOpts = await resolveEffectiveSyncOptions(workspace, opts.repository);
  
  // Merge CLI flags with resolved settings (CLI flags take precedence only for explicit overrides)
  // Start with effective options from config, then overlay only explicitly provided CLI flags
  const mergedOpts = Object.assign({}, effectiveOpts);
  
  // Always preserve these operational CLI flags
  if (opts.dryRun !== undefined) mergedOpts.dryRun = opts.dryRun;
  if (opts.yes !== undefined) mergedOpts.yes = opts.yes;
  if (opts.stateful !== undefined) mergedOpts.stateful = opts.stateful;
  if (opts.skipPull !== undefined) mergedOpts.skipPull = opts.skipPull;
  if (opts.failConflicts !== undefined) mergedOpts.failConflicts = opts.failConflicts;
  if (opts.plainSecrets !== undefined) mergedOpts.plainSecrets = opts.plainSecrets;
  if (opts.json !== undefined) mergedOpts.json = opts.json;
  if (opts.message !== undefined) mergedOpts.message = opts.message;
  if (opts.parallel !== undefined) mergedOpts.parallel = opts.parallel;
  if (opts.jsonOutput !== undefined) mergedOpts.jsonOutput = opts.jsonOutput;
  if (opts.repository !== undefined) mergedOpts.repository = opts.repository;
  
  opts = mergedOpts;
  
  const codebases = await listSyncCodebases(opts);
  if (opts.raw) {
    log.info("--raw is now the default, you can remove it as a flag");
  }
  if (opts.stateful) {
    if (!opts.skipPull) {
      log.info(
        colors.gray("You need to be up-to-date before pushing, pulling first.")
      );
      await pull(opts);
      log.info(colors.green("Pull done, now pushing."));
      log.info("\n");
    }
  }

  log.info(
    colors.gray(
      "Computing the files to update on the remote to match local (taking wmill.yaml includes/excludes into account)"
    )
  );
  let resourceTypeToFormatExtension: Record<string, string> = {};
  try {
    resourceTypeToFormatExtension = (await wmill.fileResourceTypeToFileExtMap({
      workspace: workspace.workspaceId,
    })) as Record<string, string>;
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
      opts.defaultTs
    ))!,
    !opts.json,
    opts.defaultTs ?? "bun",
    resourceTypeToFormatExtension,
    false
  );

  const local = await FSFSElement(path.join(Deno.cwd(), ""), codebases, false);
  const changes = await compareDynFSElement(
    local,
    remote,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    true,
    codebases,
    false
  );

  const globalDeps = await findGlobalDeps();

  const tracker: ChangeTracker = await buildTracker(changes);

  const staleScripts: string[] = [];
  const staleFlows: string[] = [];
  for (const change of tracker.scripts) {
    const stale = await generateScriptMetadataInternal(
      change,
      workspace,
      opts,
      true,
      true,
      globalDeps,
      codebases,
      false
    );
    if (stale) {
      staleScripts.push(stale);
    }
  }

  if (staleScripts.length > 0) {
    log.info("");
    log.warn(
      "Stale scripts metadata found, you may want to update them using 'wmill script generate-metadata' before pushing:"
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
      false,
      true
    );
    if (stale) {
      staleFlows.push(stale);
    }
  }

  if (staleFlows.length > 0) {
    log.warn(
      "Stale flows locks found, you may want to update them using 'wmill flow generate-locks' before pushing:"
    );
    for (const stale of staleFlows) {
      log.warn(stale);
    }
    log.info("");
  }

  const version = await fetchVersion(workspace.remote);

  log.info(colors.gray("Remote version: " + version));

  log.info(
    `remote (${workspace.name}) <- local: ${changes.length} changes to apply`
  );

  // Handle JSON output for dry-run
  if (opts.dryRun && opts.jsonOutput) {
    const result = {
      success: true,
      changes: changes.map(change => ({
        type: change.name,
        path: change.path,
        ...(change.name === "edited" && change.codebase ? { codebase_changed: true } : {})
      })),
      total: changes.length
    };
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  if (changes.length > 0) {
    if (!opts.jsonOutput) {
      prettyChanges(changes);
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
        await Deno.stat(path.join(Deno.cwd(), ".wmill"));
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
        0
      )} files to process`
    );
    if (parallelizationFactor > 1) {
      log.info(`Parallelizing ${parallelizationFactor} changes at a time`);
    }

    // Create a pool of workers that processes items as they become available
    const pool = new Set();
    const queue = [...groupedChangesArray];

    while (queue.length > 0 || pool.size > 0) {
      // Fill the pool until we reach parallelizationFactor
      while (pool.size < parallelizationFactor && queue.length > 0) {
        const [_basePath, changes] = queue.shift()!;
        const promise = (async () => {
          const alreadySynced: string[] = [];

          for await (const change of changes) {
            let stateTarget = undefined;
            if (stateful) {
              try {
                stateTarget = path.join(Deno.cwd(), ".wmill", change.path);
                await Deno.stat(stateTarget);
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
                  globalDeps,
                  codebases,
                  opts
                )
              ) {
                if (stateTarget) {
                  await Deno.writeTextFile(stateTarget, change.after);
                }
                continue;
              } else if (
                await handleFile(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  globalDeps,
                  codebases
                )
              ) {
                if (stateTarget) {
                  await Deno.writeTextFile(stateTarget, change.after);
                }
                continue;
              }
              if (stateTarget) {
                await ensureDir(path.dirname(stateTarget));
                log.info(
                  `Editing ${getTypeStrFromPath(change.path)} ${change.path}`
                );
              }

              if (isFileResource(change.path)) {
                const resourceFilePath = await findResourceFile(change.path);
                if (!alreadySynced.includes(resourceFilePath)) {
                  alreadySynced.push(resourceFilePath);

                  const newObj = parseFromPath(
                    resourceFilePath,
                    await Deno.readTextFile(resourceFilePath)
                  );

                  await pushResource(
                    workspace.workspaceId,
                    resourceFilePath,
                    undefined,
                    newObj
                  );
                  if (stateTarget) {
                    await Deno.writeTextFile(stateTarget, change.after);
                  }
                  continue;
                }
              }
              const oldObj = parseFromPath(change.path, change.before);
              const newObj = parseFromPath(change.path, change.after);

              await pushObj(
                workspace.workspaceId,
                change.path,
                oldObj,
                newObj,
                opts.plainSecrets ?? false,
                alreadySynced,
                opts.message
              );

              if (stateTarget) {
                await Deno.writeTextFile(stateTarget, change.after);
              }
            } else if (change.name === "added") {
              if (
                change.path.endsWith(".script.json") ||
                change.path.endsWith(".script.yaml") ||
                change.path.endsWith(".lock") ||
                isFileResource(change.path)
              ) {
                continue;
              } else if (
                await handleFile(
                  change.path,
                  workspace,
                  alreadySynced,
                  opts.message,
                  opts,
                  globalDeps,
                  codebases
                )
              ) {
                continue;
              }
              if (stateTarget) {
                await ensureDir(path.dirname(stateTarget));
                log.info(
                  `Adding ${getTypeStrFromPath(change.path)} ${change.path}`
                );
              }
              const obj = parseFromPath(change.path, change.content);
              await pushObj(
                workspace.workspaceId,
                change.path,
                undefined,
                obj,
                opts.plainSecrets ?? false,
                [],
                opts.message
              );

              if (stateTarget) {
                await Deno.writeTextFile(stateTarget, change.content);
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
                  const script = await wmill.getScriptByPath({
                    workspace: workspaceId,
                    path: removeExtensionToPath(target),
                  });
                  await wmill.archiveScriptByHash({
                    workspace: workspaceId,
                    hash: script.hash,
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
                    path: removeSuffix(target, ".flow/flow.json"),
                  });
                  break;
                case "app":
                  await wmill.deleteApp({
                    workspace: workspaceId,
                    path: removeSuffix(target, ".app/app.json"),
                  });
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
                    ".user.json"
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
                      ".group.json"
                    ),
                  });
                  break;
                default:
                  break;
              }
              if (stateTarget) {
                try {
                  await Deno.remove(stateTarget);
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
        changes: changes.map(change => ({
          type: change.name,
          path: change.path,
          ...(change.name === "edited" && change.codebase ? { codebase_changed: true } : {})
        })),
        total: changes.length,
        duration_ms: Math.round(performance.now() - start)
      };
      console.log(JSON.stringify(result, null, 2));
    } else {
      log.info(
        colors.bold.green.underline(
          `\nDone! All ${changes.length} changes pushed to the remote workspace ${
            workspace.workspaceId
          } named ${workspace.name} (${(performance.now() - start).toFixed(0)}ms)`
        )
      );
    }
  } else if (opts.jsonOutput) {
    console.log(JSON.stringify({ success: true, message: "No changes to push", total: 0 }, null, 2));
  }
}

const command = new Command()
  .description(
    "sync local with a remote workspaces or the opposite (push or pull)"
  )

  .action(() =>
    log.info("2 actions available, pull and push. Use -h to display help.")
  )
  .command("pull")
  .description("Pull any remote changes and apply them locally.")
  .option("--yes", "Pull without needing confirmation")
  .option(
    "--dry-run",
    "Show changes that would be pulled without actually pushing"
  )
  .option("--plain-secrets", "Pull secrets as plain text")
  .option("--json", "Use JSON instead of YAML")
  .option("--skip-variables", "Skip syncing variables (including secrets)")
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option("--skip-resources", "Skip syncing  resources")
  .option("--skip-resource-types", "Skip syncing  resource types")
  // .option("--skip-scripts-metadata", "Skip syncing scripts metadata, focus solely on logic")
  .option("--include-schedules", "Include syncing  schedules")
  .option("--include-triggers", "Include syncing triggers")
  .option("--include-users", "Include syncing users")
  .option("--include-groups", "Include syncing groups")
  .option("--include-settings", "Include syncing workspace settings")
  .option("--include-key", "Include workspace encryption key")
  .option("--json-output", "Output results in JSON format")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Overrides wmill.yaml includes"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account. Overrides wmill.yaml excludes"
  )
  .option(
    "--extra-includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Useful to still take wmill.yaml into account and act as a second pattern to satisfy"
  )
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo) when multiple repositories exist"
  )
  // deno-lint-ignore no-explicit-any
  .action(pull as any)
  .command("push")
  .description("Push any local changes and apply them remotely.")
  .option("--yes", "Push without needing confirmation")
  .option(
    "--dry-run",
    "Show changes that would be pushed without actually pushing"
  )
  .option("--plain-secrets", "Push secrets as plain text")
  .option("--json", "Use JSON instead of YAML")
  .option("--skip-variables", "Skip syncing variables (including secrets)")
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option("--skip-resources", "Skip syncing  resources")
  .option("--skip-resource-types", "Skip syncing  resource types")

  // .option("--skip-scripts-metadata", "Skip syncing scripts metadata, focus solely on logic")
  .option("--include-schedules", "Include syncing schedules")
  .option("--include-triggers", "Include syncing triggers")
  .option("--include-users", "Include syncing users")
  .option("--include-groups", "Include syncing groups")
  .option("--include-settings", "Include syncing workspace settings")
  .option("--include-key", "Include workspace encryption key")
  .option("--json-output", "Output results in JSON format")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account."
  )
  .option(
    "--extra-includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Useful to still take wmill.yaml into account and act as a second pattern to satisfy"
  )
  .option(
    "--message <message:string>",
    "Include a message that will be added to all scripts/flows/apps updated during this push"
  )
  .option("--parallel <number>", "Number of changes to process in parallel")
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo) when multiple repositories exist"
  )
  // deno-lint-ignore no-explicit-any
  .action(push as any);

export default command;
