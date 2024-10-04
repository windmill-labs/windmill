import { fetchVersion, requireLogin, resolveWorkspace } from "./context.ts";
import {
  colors,
  Command,
  Confirm,
  ensureDir,
  minimatch,
  JSZip,
  path,
  log,
  yamlStringify,
  yamlParse,
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
import { SyncOptions, mergeConfigWithConfigFile } from "./conf.ts";
import { removePathPrefix } from "./types.ts";
import { SyncCodebase, listSyncCodebases } from "./codebase.ts";
import {
  generateFlowLockInternal,
  generateScriptMetadataInternal,
} from "./metadata.ts";
import { FlowModule, OpenFlow, RawScript } from "./gen/types.gen.ts";
import { pushResource } from "./resource.ts";

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
  codebases: SyncCodebase[]
): Promise<string> {
  const isScript = path.endsWith(".script.yaml");
  if (!isScript) {
    return content;
  }
  let isTs = true;
  try {
    await Deno.stat(path.replace(".script.yaml", ".ts"));
  } catch {
    isTs = false;
  }
  if (!isTs) {
    return content;
  }
  if (isTs) {
    const c = findCodebase(path, codebases);
    if (c) {
      const parsed: any = yamlParse(content);
      if (parsed && typeof parsed == "object") {
        parsed["codebase"] = c.digest;
        parsed["lock"] = undefined;
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
  codebases: SyncCodebase[]
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

        const r = await addCodebaseDigestIfRelevant(localP, content, codebases);
        // console.log(r);
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
    else if (language == "snowflake") ext = "sf.sql";
    else if (language == "mssql") ext = "ms.sql";
    else if (language == "graphql") ext = "gql";
    else if (language == "nativets") ext = "native.ts";
    else if (language == "frontend") ext = "frontend.js";
    else if (language == "php") ext = "php";
    else if (language == "rust") ext = "rs";
    else if (language == "ansible") ext = "playbook.yml";
    else ext = "no_ext";

    return [`${name}.inline_script.`, ext];
  }

  return { assignPath };
}
function ZipFSElement(
  zip: JSZip,
  useYaml: boolean,
  defaultTs: "bun" | "deno",
  resourceTypeToFormatExtension: Record<string, string>
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
    yield e;
    for await (const e2 of e.c()) {
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
type Edit = { name: "edited"; path: string; before: string; after: string };

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
    if (json && path.endsWith(".yaml")) continue;
    if (!json && path.endsWith(".json")) continue;
    const ext = json ? ".json" : ".yaml";
    if (!skips.includeSchedules && path.endsWith(".schedule" + ext)) continue;
    if (!skips.includeUsers && path.endsWith(".user" + ext)) continue;
    if (!skips.includeGroups && path.endsWith(".group" + ext)) continue;
    if (!skips.includeSettings && path === "settings" + ext) continue;
    if (!skips.includeKey && path === "encryption_key") continue;
    if (skips.skipResources && path.endsWith(".resource" + ext)) continue;
    if (skips.skipVariables && path.endsWith(".variable" + ext)) continue;

    if (skips.skipResources && path.endsWith(".resource" + ext)) continue;
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
        "yml",
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
          o = yamlParse(content);
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

interface Skips {
  skipVariables?: boolean | undefined;
  skipResources?: boolean | undefined;
  skipSecrets?: boolean | undefined;
  includeSchedules?: boolean | undefined;
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
  ignoreMetadataDeletion: boolean
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
      const o: any = yamlParse(v);
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
      const o: any = yamlParse(v);
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
      return yamlParse(v);
    }
  }
  for (const [k, v] of Object.entries(m1)) {
    if (m2[k] === undefined) {
      changes.push({ name: "added", path: k, content: v });
    } else if (
      m2[k] != v &&
      (!k.endsWith(".json") || !deepEqual(JSON.parse(v), JSON.parse(m2[k]))) &&
      (!k.endsWith(".yaml") || !deepEqual(parseYaml(k, v), parseYaml(k, m2[k])))
    ) {
      changes.push({ name: "edited", path: k, after: v, before: m2[k] });
    }
  }

  for (const [k] of Object.entries(m2)) {
    if (
      m1[k] === undefined &&
      (!ignoreMetadataDeletion ||
        (!k?.endsWith(".script.yaml") && !k?.endsWith(".script.json")))
    ) {
      changes.push({ name: "deleted", path: k });
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
  } else if (typ == "variable") {
    return 8;
  } else if (typ == "user") {
    return 9;
  } else if (typ == "group") {
    return 10;
  } else if (typ == "encryption_key") {
    return 11;
  } else {
    return 12;
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
    return (
      !isWhitelisted(p) &&
      (isNotWmillFile(p, isDirectory) ||
        (!isDirectory && whitelist != undefined && !whitelist.approve(p)))
    );
  };
}

export async function pull(opts: GlobalOptions & SyncOptions) {
  opts = await mergeConfigWithConfigFile(opts);

  if (opts.stateful) {
    await ensureDir(path.join(Deno.cwd(), ".wmill"));
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

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
      opts.skipSecrets,
      opts.includeSchedules,
      opts.includeUsers,
      opts.includeGroups,
      opts.includeSettings,
      opts.includeKey,
      opts.defaultTs
    ))!,
    !opts.json,
    opts.defaultTs ?? "bun",
    resourceTypeToFormatExtension
  );
  const local = !opts.stateful
    ? await FSFSElement(Deno.cwd(), codebases)
    : await FSFSElement(path.join(Deno.cwd(), ".wmill"), []);
  const changes = await compareDynFSElement(
    remote,
    local,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    false
  );

  log.info(
    `remote (${workspace.name}) -> local: ${changes.length} changes to apply`
  );
  if (changes.length > 0) {
    prettyChanges(changes);
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
    const changedScripts: string[] = [];
    const changedFlows: string[] = [];
    const changedApps: string[] = [];

    // deno-lint-ignore no-inner-declarations
    async function addToChangedIfNotExists(p: string) {
      const isScript = exts.some((e) => p.endsWith(e));
      if (isScript) {
        if (p.includes(".flow" + SEP)) {
          const folder =
            p.substring(0, p.indexOf(".flow" + SEP)) + ".flow" + SEP;
          if (!changedFlows.includes(folder)) {
            changedFlows.push(folder);
          }
        } else if (p.includes(".app" + SEP)) {
          const folder = p.substring(0, p.indexOf(".app" + SEP)) + ".app" + SEP;
          if (!changedApps.includes(folder)) {
            changedApps.push(folder);
          }
        } else {
          if (!changedScripts.includes(p)) {
            changedScripts.push(p);
          }
        }
      } else if (p.endsWith(".script.yaml") || p.endsWith(".script.json")) {
        try {
          const contentPath = await findContentFile(p);
          if (!contentPath) return;
          if (changedScripts.includes(contentPath)) return;
          changedScripts.push(contentPath);
        } catch {
          // ignore
        }
      }
    }

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
        await addToChangedIfNotExists(change.path);
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
        await addToChangedIfNotExists(change.path);
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

    const globalDeps = await findGlobalDeps();

    for (const change of changedScripts) {
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
    for (const change of changedFlows) {
      log.info(`Updating lock for flow ${change}`);
      await generateFlowLockInternal(change, false, workspace, true);
    }
    if (changedApps.length > 0) {
      log.info(
        `Apps ${changedApps.join(
          ", "
        )} scripts were changed but ignoring for now`
      );
    }
    log.info(
      colors.bold.green.underline(
        `\nDone! All ${changes.length} changes applied locally and wmill-lock.yaml updated.`
      )
    );
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
        colors.yellow(`~ ${getTypeStrFromPath(change.path)} ` + change.path)
      );
      showDiff(change.before, change.after);
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

export async function push(opts: GlobalOptions & SyncOptions) {
  opts = await mergeConfigWithConfigFile(opts);
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

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

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
      opts.skipSecrets,
      opts.includeSchedules,
      opts.includeUsers,
      opts.includeGroups,
      opts.includeSettings,
      opts.includeKey,
      opts.defaultTs
    ))!,
    !opts.json,
    opts.defaultTs ?? "bun",
    resourceTypeToFormatExtension
  );

  const local = await FSFSElement(path.join(Deno.cwd(), ""), codebases);
  const changes = await compareDynFSElement(
    local,
    remote,
    await ignoreF(opts),
    opts.json ?? false,
    opts,
    true
  );

  const version = await fetchVersion(workspace.remote);

  log.info(colors.gray("Remote version: " + version));

  log.info(
    `remote (${workspace.name}) <- local: ${changes.length} changes to apply`
  );

  if (changes.length > 0) {
    prettyChanges(changes);
    if (
      !opts.yes &&
      !(await Confirm.prompt({
        message: `Do you want to apply these ${changes.length} changes to the remote?`,
        default: true,
      }))
    ) {
      return;
    }

    log.info(colors.gray(`Applying changes to files ...`));

    const alreadySynced: string[] = [];
    const globalDeps = await findGlobalDeps();

    for await (const change of changes) {
      const stateTarget = path.join(Deno.cwd(), ".wmill", change.path);
      let stateExists = true;
      try {
        await Deno.stat(stateTarget);
      } catch {
        stateExists = false;
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
          if (opts.stateful && stateExists) {
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
          if (opts.stateful && stateExists) {
            await Deno.writeTextFile(stateTarget, change.after);
          }
          continue;
        }
        if (opts.stateful) {
          await ensureDir(path.dirname(stateTarget));
          log.info(`Editing ${getTypeStrFromPath(change.path)} ${change.path}`);
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
            if (opts.stateful && stateExists) {
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

        if (opts.stateful && stateExists) {
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
        if (opts.stateful && stateExists) {
          await ensureDir(path.dirname(stateTarget));
          log.info(`Adding ${getTypeStrFromPath(change.path)} ${change.path}`);
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

        if (opts.stateful && stateExists) {
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
        switch (typ) {
          case "script": {
            const script = await wmill.getScriptByPath({
              workspace: workspaceId,
              path: removeExtensionToPath(change.path),
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
              path: removeSuffix(change.path, ".resource.json"),
            });
            break;
          case "resource-type":
            await wmill.deleteResourceType({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".resource-type.json"),
            });
            break;
          case "flow":
            await wmill.deleteFlowByPath({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".flow/flow.json"),
            });
            break;
          case "app":
            await wmill.deleteApp({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".app/app.json"),
            });
            break;
          case "schedule":
            await wmill.deleteSchedule({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".schedule.json"),
            });
            break;
          case "variable":
            await wmill.deleteVariable({
              workspace: workspaceId,
              path: removeSuffix(change.path, ".variable.json"),
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
        try {
          await Deno.remove(stateTarget);
        } catch {
          // state target may not exist already
        }
      }
    }
    log.info(
      colors.bold.green.underline(
        `\nDone! All ${changes.length} changes pushed to the remote workspace ${workspace.workspaceId} named ${workspace.name}.`
      )
    );
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
  .option(
    "--fail-conflicts",
    "Error on conflicts (both remote and local have changes on the same item)"
  )
  .option(
    "--raw",
    "Push without using state, just overwrite. (Default, has no effect)"
  )
  .option("--yes", "Pull without needing confirmation")
  .option(
    "--stateful",
    "Pull using state tracking (create .wmill folder and needed for --fail-conflicts)"
  )
  .option("--plain-secrets", "Pull secrets as plain text")
  .option("--json", "Use JSON instead of YAML")
  .option("--skip-variables", "Skip syncing variables (including secrets)")
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option("--skip-resources", "Skip syncing  resources")
  .option("--include-schedules", "Include syncing  schedules")
  .option("--include-users", "Include syncing users")
  .option("--include-groups", "Include syncing groups")
  .option("--include-settings", "Include syncing workspace settings")
  .option("--include-key", "Include workspace encryption key")
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
  // deno-lint-ignore no-explicit-any
  .action(pull as any)
  .command("push")
  .description("Push any local changes and apply them remotely.")
  .option(
    "--fail-conflicts",
    "Error on conflicts (both remote and local have changes on the same item)"
  )
  .option(
    "--raw",
    "Push without using state, just overwrite. (Default, has no effect)"
  )
  .option(
    "--stateful",
    "Pull using state tracking (use .wmill folder and needed for --fail-conflicts)w"
  )
  .option("--skip-pull", "(stateful only) Push without pulling first")
  .option("--yes", "Push without needing confirmation")
  .option("--plain-secrets", "Push secrets as plain text")
  .option("--json", "Use JSON instead of YAML")
  .option("--skip-variables", "Skip syncing variables (including secrets)")
  .option("--skip-secrets", "Skip syncing only secrets variables")
  .option("--skip-resources", "Skip syncing  resources")
  .option("--include-schedules", "Include syncing schedules")
  .option("--include-users", "Include syncing users")
  .option("--include-groups", "Include syncing groups")
  .option("--include-settings", "Include syncing workspace settings")
  .option("--include-key", "Include workspace encryption key")
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
  // deno-lint-ignore no-explicit-any
  .action(push as any);

export default command;
