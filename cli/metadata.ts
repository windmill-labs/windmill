// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import {
  SEP,
  colors,
  log,
  path,
  yamlParseFile,
  yamlStringify,
} from "./deps.ts";
import {
  ScriptMetadata,
  defaultScriptMetadata,
} from "./bootstrap/script_bootstrap.ts";
import { Workspace } from "./workspace.ts";
import { SchemaProperty } from "./bootstrap/common.ts";
import {
  languagesWithRawReqsSupport,
  LanguageWithRawReqsSupport,
  ScriptLanguage,
} from "./script_common.ts";
import { inferContentTypeFromFilePath } from "./script_common.ts";
import { GlobalDeps, exts, findGlobalDeps } from "./script.ts";
import {
  FSFSElement,
  findCodebase,
  newPathAssigner,
  yamlOptions,
} from "./sync.ts";
import { generateHash, readInlinePathSync } from "./utils.ts";
import { SyncCodebase } from "./codebase.ts";
import { FlowFile } from "./flow.ts";
import { replaceInlineScripts } from "../../windmill-utils/src/inline-scripts/replacer.ts";
import { getIsWin } from "./main.ts";
import { FlowValue } from "./gen/types.gen.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../windmill-utils/src/inline-scripts/extractor.ts";

export class LockfileGenerationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "LockfileGenerationError";
  }
}

export async function generateAllMetadata() {}

function findClosestRawReqs(
  lang: LanguageWithRawReqsSupport | undefined,
  remotePath: string,
  globalDeps: GlobalDeps
): string | undefined {
  let bestCandidate: { k: string; v: string } | undefined = undefined;
  if (lang) {
    Object.entries(globalDeps.get(lang) ?? {}).forEach(([k, v]) => {
      if (
        remotePath.startsWith(k) &&
        k.length >= (bestCandidate?.k ?? "").length
      ) {
        bestCandidate = { k, v };
      }
    });
  }
  // @ts-ignore
  return bestCandidate?.v;
}

const TOP_HASH = "__flow_hash";
async function generateFlowHash(
  rawReqs: Record<string, string> | undefined,
  folder: string,
  defaultTs: "bun" | "deno" | undefined
) {
  const elems = await FSFSElement(path.join(Deno.cwd(), folder), [], true);
  const hashes: Record<string, string> = {};
  for await (const f of elems.getChildren()) {
    if (exts.some((e) => f.path.endsWith(e))) {
      let reqs: string | undefined;
      if (rawReqs) {
        // Get language name from path
        const lang = inferContentTypeFromFilePath(f.path, defaultTs);
        // Get lock for that language
        [, reqs] =
          Object.entries(rawReqs).find(([lang2, _]) => lang == lang2) ?? [];
      }

      // Embed lock into hash
      hashes[f.path] = await generateHash(
        (await f.getContentText()) + (reqs ?? "")
      );
    }
  }
  return { ...hashes, [TOP_HASH]: await generateHash(JSON.stringify(hashes)) };
}
export async function generateFlowLockInternal(
  folder: string,
  dryRun: boolean,
  workspace: Workspace,
  opts: GlobalOptions & {
    defaultTs?: "bun" | "deno";
  },
  justUpdateMetadataLock?: boolean,
  noStaleMessage?: boolean,
  useRawReqs?: boolean
): Promise<string | void> {
  if (folder.endsWith(SEP)) {
    folder = folder.substring(0, folder.length - 1);
  }
  const remote_path = folder
    .replaceAll(SEP, "/")
    .substring(0, folder.length - ".flow".length);
  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating lock for flow ${folder} at ${remote_path}`);
  }

  let rawReqs: Record<string, string> | undefined = undefined;
  if (useRawReqs) {
    // Find all dependency files in the workspace
    const globalDeps = await findGlobalDeps();

    // Find closest dependency files for this flow
    rawReqs = {};

    // TODO: PERF: Only include raw reqs for the languages that are in the flow
    languagesWithRawReqsSupport.map((lang) => {
      const dep = findClosestRawReqs(lang, folder, globalDeps);
      if (dep) {
        // @ts-ignore
        rawReqs[lang.language] = dep;
      }
    });
  }
  let hashes = await generateFlowHash(rawReqs, folder, opts.defaultTs);

  const conf = await readLockfile();
  if (await checkifMetadataUptodate(folder, hashes[TOP_HASH], conf, TOP_HASH)) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`Flow ${remote_path} metadata is up-to-date, skipping`)
      );
    }
    return;
  } else if (dryRun) {
    return remote_path;
  }

  if (useRawReqs) {
    log.warn(
      "If using local lockfiles, following redeployments from Web App will inevitably override generated lockfiles by CLI. To maintain your script's lockfiles you will need to redeploy only from CLI. (Behavior is subject to change)"
    );
    log.info(
      (await blueColor())(
        `Found raw requirements (${languagesWithRawReqsSupport
          .map((l) => l.rrFilename)
          .join("/")}) for ${folder}, using it`
      )
    );
  }

  const flowValue = (await yamlParseFile(
    folder! + SEP + "flow.yaml"
  )) as FlowFile;

  if (!justUpdateMetadataLock) {
    const changedScripts = [];
    //find hashes that do not correspond to previous hashes
    for (const [path, hash] of Object.entries(hashes)) {
      if (path == TOP_HASH) {
        continue;
      }
      if (!(await checkifMetadataUptodate(folder, hash, conf, path))) {
        changedScripts.push(path);
      }
    }

    log.info(`Recomputing locks of ${changedScripts.join(", ")} in ${folder}`);
    replaceInlineScripts(
      flowValue.value.modules,
      Deno.readTextFileSync,
      folder + SEP!,
      { removeLocks: changedScripts }
    );

    //removeChangedLocks
    flowValue.value = await updateFlow(
      workspace,
      flowValue.value,
      remote_path,
      rawReqs
    );

    const inlineScripts = extractInlineScriptsForFlows(flowValue.value.modules);
    inlineScripts
      .filter((s) => s.path.endsWith(".lock"))
      .forEach((s) => {
        Deno.writeTextFileSync(
          Deno.cwd() + SEP + folder + SEP + s.path,
          s.content
        );
      });

    // Overwrite `flow.yaml` with the new lockfile references
    await Deno.writeTextFile(
      Deno.cwd() + SEP + folder + SEP + "flow.yaml",
      yamlStringify(flowValue as Record<string, any>)
    );
  }

  hashes = await generateFlowHash(rawReqs, folder, opts.defaultTs);

  for (const [path, hash] of Object.entries(hashes)) {
    await updateMetadataGlobalLock(folder, hash, path);
  }
  log.info(colors.green(`Flow ${remote_path} lockfiles updated`));
}

// on windows, when using powershell, blue is not readable
export async function blueColor(): Promise<(x: string) => void> {
  const isWin = await getIsWin();
  return isWin ? colors.black : colors.blue;
}

export async function generateScriptMetadataInternal(
  scriptPath: string,
  workspace: Workspace,
  opts: GlobalOptions & {
    lockOnly?: boolean | undefined;
    schemaOnly?: boolean | undefined;
    defaultTs?: "bun" | "deno";
  },
  dryRun: boolean,
  noStaleMessage: boolean,
  globalDeps: GlobalDeps,
  codebases: SyncCodebase[],
  justUpdateMetadataLock?: boolean
): Promise<string | undefined> {
  const remotePath = scriptPath
    .substring(0, scriptPath.indexOf("."))
    .replaceAll(SEP, "/");

  const language = inferContentTypeFromFilePath(scriptPath, opts.defaultTs);

  const rrLang = languagesWithRawReqsSupport.find(
    (l) => language == l.language
  );

  const rawReqs = findClosestRawReqs(rrLang, scriptPath, globalDeps);

  if (rawReqs && rrLang) {
    log.info(
      (await blueColor())(
        `Found raw requirements (${rrLang.rrFilename}) for ${scriptPath}, using it`
      )
    );
  }
  const metadataWithType = await parseMetadataFile(
    remotePath,
    undefined,
    globalDeps,
    codebases
  );

  // read script content
  const scriptContent = await Deno.readTextFile(scriptPath);
  const metadataContent = await Deno.readTextFile(metadataWithType.path);

  let hash = await generateScriptHash(rawReqs, scriptContent, metadataContent);

  if (await checkifMetadataUptodate(remotePath, hash, undefined)) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`Script ${remotePath} metadata is up-to-date, skipping`)
      );
    }
    return;
  } else if (dryRun) {
    return `${remotePath} (${language})`;
  }

  if (!justUpdateMetadataLock) {
    log.info(colors.gray(`Generating metadata for ${scriptPath}`));
  }

  const metadataParsedContent = metadataWithType?.payload as Record<
    string,
    any
  >;

  if (!opts.lockOnly && !justUpdateMetadataLock) {
    await updateScriptSchema(
      scriptContent,
      language,
      metadataParsedContent,
      scriptPath
    );
  }

  if (!opts.schemaOnly && !justUpdateMetadataLock) {
    const hasCodebase = findCodebase(scriptPath, codebases) != undefined;

    if (!hasCodebase) {
      await updateScriptLock(
        workspace,
        scriptContent,
        language,
        remotePath,
        metadataParsedContent,
        rawReqs
      );
    } else {
      metadataParsedContent.lock = "";
    }
  } else {
    metadataParsedContent.lock =
      "!inline " + remotePath.replaceAll(SEP, "/") + ".script.lock";
  }
  let metaPath = remotePath + ".script.yaml";
  let newMetadataContent = yamlStringify(metadataParsedContent, yamlOptions);
  if (metadataWithType.isJson) {
    metaPath = remotePath + ".script.json";
    newMetadataContent = JSON.stringify(metadataParsedContent);
  }

  const metadataContentUsedForHash = newMetadataContent;

  hash = await generateScriptHash(
    rawReqs,
    scriptContent,
    metadataContentUsedForHash
  );
  await updateMetadataGlobalLock(remotePath, hash);
  if (!justUpdateMetadataLock) {
    await Deno.writeTextFile(metaPath, newMetadataContent);
  }
  return `${remotePath} (${language})`;
}

export async function updateScriptSchema(
  scriptContent: string,
  language: ScriptLanguage,
  metadataContent: Record<string, any>,
  path: string
): Promise<void> {
  // infer schema from script content and update it inplace
  const result = await inferSchema(
    language,
    scriptContent,
    metadataContent.schema,
    path
  );
  metadataContent.schema = result.schema;
  if (result.has_preprocessor) {
    metadataContent.has_preprocessor = result.has_preprocessor;
  } else {
    delete metadataContent.has_preprocessor;
  }
  if (result.no_main_func) {
    metadataContent.no_main_func = result.no_main_func;
  } else {
    delete metadataContent.no_main_func;
  }
}

async function updateScriptLock(
  workspace: Workspace,
  scriptContent: string,
  language: ScriptLanguage,
  remotePath: string,
  metadataContent: Record<string, any>,
  rawDeps: string | undefined
): Promise<void> {
  if (
    !(
      languagesWithRawReqsSupport.some((l) => l.language == language) ||
      language == "deno" ||
      language == "rust" ||
      language == "ansible"
    )
  ) {
    return;
  }
  // generate the script lock running a dependency job in Windmill and update it inplace
  // TODO: update this once the client is released
  const rawResponse = await fetch(
    `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/dependencies`,
    {
      method: "POST",
      headers: {
        Cookie: `token=${workspace.token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        raw_scripts: [
          {
            raw_code: scriptContent,
            language: language,
            script_path: remotePath,
          },
        ],
        raw_deps: rawDeps,
        entrypoint: remotePath,
      }),
    }
  );

  let responseText = "reading response failed";
  try {
    responseText = await rawResponse.text();
    const response = JSON.parse(responseText);
    const lock = response.lock;
    if (lock === undefined) {
      if (response?.["error"]?.["message"]) {
        throw new LockfileGenerationError(
          `Failed to generate lockfile: ${response?.["error"]?.["message"]}`
        );
      }
      throw new LockfileGenerationError(
        `Failed to generate lockfile: ${JSON.stringify(response, null, 2)}`
      );
    }
    const lockPath = remotePath + ".script.lock";
    if (lock != "") {
      await Deno.writeTextFile(lockPath, lock);
      metadataContent.lock = "!inline " + lockPath.replaceAll(SEP, "/");
    } else {
      try {
        if (await Deno.stat(lockPath)) {
          await Deno.remove(lockPath);
        }
      } catch {}
      metadataContent.lock = "";
    }
  } catch (e) {
    if (e instanceof LockfileGenerationError) {
      throw e;
    }
    throw new LockfileGenerationError(
      `Failed to generate lockfile:${rawResponse.statusText}, ${responseText}, ${e}`
    );
  }
}

export async function updateFlow(
  workspace: Workspace,
  flow_value: FlowValue,
  remotePath: string,
  rawDeps?: Record<string, string>
): Promise<FlowValue | undefined> {
  let rawResponse;

  if (rawDeps != undefined) {
    log.info(colors.blue("Using raw requirements for flow dependencies"));

    // generate the script lock running a dependency job in Windmill and update it inplace
    rawResponse = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/flow_dependencies`,
      {
        method: "POST",
        headers: {
          Cookie: `token=${workspace.token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          flow_value,
          path: remotePath,
          use_local_lockfiles: true,
          raw_deps: rawDeps,
        }),
      }
    );
  } else {
    // Standard dependency resolution on the server
    rawResponse = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/flow_dependencies`,
      {
        method: "POST",
        headers: {
          Cookie: `token=${workspace.token}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          flow_value,
          path: remotePath,
        }),
      }
    );
  }

  let responseText = "reading response failed";
  try {
    const res = (await rawResponse.json()) as
      | { updated_flow_value: any }
      | { error: { message: string } }
      | undefined;
    if (rawResponse.status != 200) {
      const msg = (res as any)?.["error"]?.["message"];
      if (msg) {
        throw new LockfileGenerationError(
          `Failed to generate lockfile: ${msg}`
        );
      }
      throw new LockfileGenerationError(
        `Failed to generate lockfile: ${rawResponse.statusText}, ${responseText}`
      );
    }
    return (res as any).updated_flow_value;
  } catch (e) {
    try {
      responseText = await rawResponse.text();
    } catch {}
    throw new Error(
      `Failed to generate lockfile. Status was: ${rawResponse.statusText}, ${responseText}, ${e}`
    );
  }
}

////////////////////////////////////////////////////////////////////////////////////////////
// below functions copied from Windmill's FE inferArgs function. TODO: refactor           //
////////////////////////////////////////////////////////////////////////////////////////////
export async function inferSchema(
  language: ScriptLanguage,
  content: string,
  currentSchema: any,
  path: string
): Promise<{
  schema: any;
  has_preprocessor: boolean | undefined;
  no_main_func: boolean | undefined;
}> {
  let inferedSchema: any;
  if (language === "python3") {
    const { parse_python } = await import(
      "./wasm/py/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_python(content));
  } else if (language === "nativets") {
    const { parse_deno } = await import("./wasm/ts/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "bun") {
    const { parse_deno } = await import("./wasm/ts/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "deno") {
    const { parse_deno } = await import("./wasm/ts/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "go") {
    const { parse_go } = await import("./wasm/go/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_go(content));
  } else if (language === "mysql") {
    const { parse_mysql } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );

    inferedSchema = JSON.parse(parse_mysql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "mysql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "bigquery") {
    const { parse_bigquery } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_bigquery(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "bigquery" } },
      ...inferedSchema.args,
    ];
  } else if (language === "oracledb") {
    const { parse_oracledb } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_oracledb(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "oracledb" } },
      ...inferedSchema.args,
    ];
  } else if (language === "snowflake") {
    const { parse_snowflake } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_snowflake(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "snowflake" } },
      ...inferedSchema.args,
    ];
  } else if (language === "mssql") {
    const { parse_mssql } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_mssql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "ms_sql_server" } },
      ...inferedSchema.args,
    ];
  } else if (language === "postgresql") {
    const { parse_sql } = await import("./wasm/regex/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_sql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "postgresql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "duckdb") {
    const { parse_duckdb } = await import("./wasm/regex/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_duckdb(content));
  } else if (language === "graphql") {
    const { parse_graphql } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_graphql(content));
    inferedSchema.args = [
      { name: "api", typ: { resource: "graphql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "bash") {
    const { parse_bash } = await import("./wasm/regex/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_bash(content));
  } else if (language === "powershell") {
    const { parse_powershell } = await import(
      "./wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_powershell(content));
  } else if (language === "php") {
    const { parse_php } = await import("./wasm/php/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_php(content));
  } else if (language === "rust") {
    const { parse_rust } = await import("./wasm/rust/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_rust(content));
  } else if (language === "csharp") {
    const { parse_csharp } = await import(
      "./wasm/csharp/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_csharp(content));
  } else if (language === "nu") {
    const { parse_nu } = await import("./wasm/nu/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_nu(content));
  } else if (language === "ansible") {
    const { parse_ansible } = await import(
      "./wasm/yaml/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_ansible(content));
  } else if (language === "java") {
    const { parse_java } = await import("./wasm/java/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_java(content));
    // for related places search: ADD_NEW_LANG
  } else {
    throw new Error("Invalid language: " + language);
  }
  if (inferedSchema.type == "Invalid") {
    log.info(
      colors.yellow(
        `Script ${path} invalid, it cannot be parsed to infer schema.`
      )
    );
    return {
      schema: defaultScriptMetadata().schema,
      has_preprocessor: false,
      no_main_func: false,
    };
  }

  if (!currentSchema) {
    currentSchema = {};
  }
  currentSchema.required = [];
  const oldProperties = JSON.parse(
    JSON.stringify(currentSchema?.properties ?? {})
  );
  currentSchema.properties = {};

  for (const arg of inferedSchema.args) {
    if (!(arg.name in oldProperties)) {
      currentSchema.properties[arg.name] = { description: "", type: "" };
    } else {
      currentSchema.properties[arg.name] = oldProperties[arg.name];
    }
    currentSchema.properties[arg.name] = sortObject(
      currentSchema.properties[arg.name]
    );

    argSigToJsonSchemaType(arg.typ, currentSchema.properties[arg.name]);

    currentSchema.properties[arg.name].default = arg.default;

    if (!arg.has_default && !currentSchema.required.includes(arg.name)) {
      currentSchema.required.push(arg.name);
    }
  }

  return {
    schema: currentSchema,
    has_preprocessor: inferedSchema.has_preprocessor,
    no_main_func: inferedSchema.no_main_func,
  };
}

function sortObject(obj: any): any {
  return Object.keys(obj)
    .sort()
    .reduce(
      (acc, key) => ({
        ...acc,
        [key]: obj[key],
      }),
      {}
    );
}

//copied straight fron frontend /src/utils/inferArgs.ts
export function argSigToJsonSchemaType(
  t:
    | string
    | { resource: string | null }
    | {
        list:
          | (
              | string
              | {
                  object: {
                    name?: string;
                    props?: { key: string; typ: any }[];
                  };
                }
            )
          | { str: any }
          | { object: { name?: string; props?: { key: string; typ: any }[] } }
          | null;
      }
    | { dynselect: string }
    | { str: string[] | null }
    | { object: { name?: string; props?: { key: string; typ: any }[] } }
    | {
        oneof: {
          label: string;
          properties: { key: string; typ: any }[];
        }[];
      },
  oldS: SchemaProperty
): void {
  const newS: SchemaProperty = { type: "" };
  if (t === "int") {
    newS.type = "integer";
  } else if (t === "float") {
    newS.type = "number";
  } else if (t === "bool") {
    newS.type = "boolean";
  } else if (t === "email") {
    newS.type = "string";
    newS.format = "email";
  } else if (t === "sql") {
    newS.type = "string";
    newS.format = "sql";
  } else if (t === "yaml") {
    newS.type = "string";
    newS.format = "yaml";
  } else if (t === "bytes") {
    newS.type = "string";
    newS.contentEncoding = "base64";
    newS.originalType = "bytes";
  } else if (t === "datetime") {
    newS.type = "string";
    newS.format = "date-time";
  } else if (typeof t !== "string" && "oneof" in t) {
    newS.type = "object";
    if (t.oneof) {
      newS.oneOf = t.oneof.map((obj) => {
        const oldObjS =
          oldS.oneOf?.find((o) => o?.title === obj.label) ?? undefined;
        const properties: Record<string, any> = {};
        for (const prop of obj.properties) {
          if (oldObjS?.properties && prop.key in oldObjS?.properties) {
            properties[prop.key] = oldObjS?.properties[prop.key];
          } else {
            properties[prop.key] = { description: "", type: "" };
          }
          argSigToJsonSchemaType(prop.typ, properties[prop.key]);
        }
        return {
          type: "object",
          title: obj.label,
          properties,
          order: oldObjS?.order ?? undefined,
        };
      });
    }
  } else if (typeof t !== "string" && `object` in t) {
    newS.type = "object";
    if (t.object.name) {
      newS.format = `resource-${t.object.name}`;
    }
    if (t.object.props) {
      const properties: Record<string, any> = {};
      for (const prop of t.object.props) {
        if (oldS.properties && prop.key in oldS.properties) {
          properties[prop.key] = oldS.properties[prop.key];
        } else {
          properties[prop.key] = { description: "", type: "" };
        }
        argSigToJsonSchemaType(prop.typ, properties[prop.key]);
      }
      newS.properties = properties;
    }
  } else if (typeof t !== "string" && `str` in t) {
    newS.type = "string";
    if (t.str) {
      newS.originalType = "enum";
      newS.enum = t.str;
    } else if (oldS.originalType == "string" && oldS.enum) {
      newS.originalType = "string";
      newS.enum = oldS.enum;
    } else {
      newS.originalType = "string";
      newS.enum = undefined;
    }
  } else if (typeof t !== "string" && `resource` in t) {
    newS.type = "object";
    newS.format = `resource-${t.resource}`;
  } else if (typeof t !== "string" && `dynselect` in t) {
    newS.type = "object";
    newS.format = `dynselect-${t.dynselect}`;
  } else if (typeof t !== "string" && `list` in t) {
    newS.type = "array";
    if (t.list === "int" || t.list === "float") {
      newS.items = { type: "number" };
      newS.originalType = "number[]";
    } else if (t.list === "bytes") {
      newS.items = { type: "string", contentEncoding: "base64" };
      newS.originalType = "bytes[]";
    } else if (
      t.list &&
      typeof t.list == "object" &&
      "str" in t.list &&
      t.list.str
    ) {
      newS.items = { type: "string", enum: t.list.str };
      newS.originalType = "enum[]";
    } else if (
      t.list == "string" ||
      (t.list && typeof t.list == "object" && "str" in t.list)
    ) {
      newS.items = { type: "string", enum: oldS.items?.enum };
      newS.originalType = "string[]";
    } else if (
      t.list &&
      typeof t.list == "object" &&
      "resource" in t.list &&
      t.list.resource
    ) {
      newS.items = {
        type: "resource",
        resourceType: t.list.resource as string,
      };
      newS.originalType = "resource[]";
    } else if (
      t.list &&
      typeof t.list == "object" &&
      "object" in t.list &&
      t.list.object
    ) {
      if (t.list.object.name) {
        newS.format = `resource-${t.list.object.name}`;
      }
      if (t.list.object.props && t.list.object.props.length > 0) {
        const properties: Record<string, any> = {};
        for (const prop of t.list.object.props) {
          properties[prop.key] = { description: "", type: "" };
          argSigToJsonSchemaType(prop.typ, properties[prop.key]);
        }
        newS.items = { type: "object", properties: properties };
      } else {
        newS.items = { type: "object" };
      }
      newS.originalType = "record[]";
    } else {
      newS.items = { type: "object" };
      newS.originalType = "object[]";
    }
  } else {
    newS.type = "object";
  }

  const preservedFields = [
    "description",
    "pattern",
    "min",
    "max",
    "currency",
    "currencyLocale",
    "multiselect",
    "customErrorMessage",
    "required",
    "showExpr",
    "password",
    "order",
    "dateFormat",
    "title",
    "placeholder",
  ];

  preservedFields.forEach((field) => {
    // @ts-ignore
    if (oldS[field] !== undefined) {
      // @ts-ignore
      newS[field] = oldS[field];
    }
  });

  if (oldS.type != newS.type) {
    for (const prop of Object.getOwnPropertyNames(newS)) {
      if (prop != "description") {
        // @ts-ignore
        delete oldS[prop];
      }
    }
  } else if (
    (oldS.format == "date" || oldS.format === "date-time") &&
    newS.format == "string"
  ) {
    newS.format = oldS.format;
  } else if (newS.format == "date-time" && oldS.format == "date") {
    newS.format = "date";
  } else if (oldS.items?.type != newS.items?.type) {
    delete oldS.items;
  }

  if (oldS.format && !newS.format) {
    oldS.format = undefined
  }

  Object.assign(oldS, newS);
  // if (sameItems && savedItems != undefined && savedItems.enum != undefined) {
  // 	sendUserToast(JSON.stringify(savedItems))
  // 	oldS.items = savedItems
  // }
}

////////////////////////////////////////////////////////////////////////////////////////////
// end of refactoring TODO                                                                //
////////////////////////////////////////////////////////////////////////////////////////////

export function replaceLock(o?: { lock?: string | string[] }) {
  if (Array.isArray(o?.lock)) {
    o.lock = o.lock.join("\n");
  }
  if (o?.lock?.startsWith("!inline ")) {
    try {
      const lockPath = o?.lock?.split(" ")[1];
      o.lock = readInlinePathSync(lockPath);
    } catch (e) {
      log.info(
        colors.yellow(`Failed to read lockfile, doing as if it was empty: ${e}`)
      );
      o.lock = "";
    }
  }
}
export async function parseMetadataFile(
  scriptPath: string,
  generateMetadataIfMissing:
    | (GlobalOptions & {
        path: string;
        workspaceRemote: Workspace;
        schemaOnly?: boolean;
      })
    | undefined,
  globalDeps: GlobalDeps,
  codebases: SyncCodebase[]
): Promise<{ isJson: boolean; payload: any; path: string }> {
  let metadataFilePath = scriptPath + ".script.json";
  try {
    await Deno.stat(metadataFilePath);
    return {
      path: metadataFilePath,
      payload: JSON.parse(await Deno.readTextFile(metadataFilePath)),
      isJson: true,
    };
  } catch {
    try {
      metadataFilePath = scriptPath + ".script.yaml";
      await Deno.stat(metadataFilePath);
      const payload: any = await yamlParseFile(metadataFilePath);
      replaceLock(payload);

      return {
        path: metadataFilePath,
        payload,
        isJson: false,
      };
    } catch {
      // no metadata file at all. Create it
      log.info(
        (await blueColor())(
          `Creating script metadata file for ${metadataFilePath}`
        )
      );
      metadataFilePath = scriptPath + ".script.yaml";
      let scriptInitialMetadata = defaultScriptMetadata();
      const scriptInitialMetadataYaml = yamlStringify(
        scriptInitialMetadata as Record<string, any>,
        yamlOptions
      );
      await Deno.writeTextFile(metadataFilePath, scriptInitialMetadataYaml, {
        createNew: true,
      });

      if (generateMetadataIfMissing) {
        log.info(
          (await blueColor())(
            `Generating lockfile and schema for ${metadataFilePath}`
          )
        );
        try {
          await generateScriptMetadataInternal(
            generateMetadataIfMissing.path,
            generateMetadataIfMissing.workspaceRemote,
            generateMetadataIfMissing,
            false,
            false,
            globalDeps,
            codebases,
            false
          );
          scriptInitialMetadata = (await yamlParseFile(
            metadataFilePath
          )) as ScriptMetadata;
          if (!generateMetadataIfMissing.schemaOnly) {
            replaceLock(scriptInitialMetadata);
          }
        } catch (e) {
          log.info(
            colors.yellow(
              `Failed to generate lockfile and schema for ${metadataFilePath}: ${e}`
            )
          );
        }
      }
      return {
        path: metadataFilePath,
        payload: scriptInitialMetadata,
        isJson: false,
      };
    }
  }
}

interface Lock {
  locks?: { [path: string]: string | { [subpath: string]: string } };
}

const WMILL_LOCKFILE = "wmill-lock.yaml";
export async function readLockfile(): Promise<Lock> {
  try {
    const read = await yamlParseFile(WMILL_LOCKFILE);
    if (typeof read == "object" && read != null) {
      return read as Lock;
    } else {
      throw new Error("Invalid lockfile");
    }
  } catch {
    const lock = { locks: {} };
    await Deno.writeTextFile(WMILL_LOCKFILE, yamlStringify(lock, yamlOptions));
    log.info(colors.green("wmill-lock.yaml created"));

    return lock;
  }
}

export async function checkifMetadataUptodate(
  path: string,
  hash: string,
  conf: Lock | undefined,
  subpath?: string
) {
  if (!conf) {
    conf = await readLockfile();
  }
  if (!conf.locks) {
    return false;
  }
  const obj = conf.locks?.[path];
  const current = subpath && typeof obj == "object" ? obj?.[subpath] : obj;
  return current == hash;
}

export async function generateScriptHash(
  rawReqs: string | undefined,
  scriptContent: string,
  newMetadataContent: string
) {
  return await generateHash(
    (rawReqs ?? "") + scriptContent + newMetadataContent
  );
}

export async function updateMetadataGlobalLock(
  path: string,
  hash: string,
  subpath?: string
): Promise<void> {
  const conf = await readLockfile();
  if (!conf?.locks) {
    conf.locks = {};
  }

  if (subpath) {
    let prev: any = conf.locks[path];
    if (!prev || typeof prev != "object") {
      prev = {};
      conf.locks[path] = prev;
    }
    prev[subpath] = hash;
  } else {
    conf.locks[path] = hash;
  }
  await Deno.writeTextFile(
    WMILL_LOCKFILE,
    yamlStringify(conf as Record<string, any>, yamlOptions)
  );
}
