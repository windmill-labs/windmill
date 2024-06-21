// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import {
  FlowValue,
  SEP,
  colors,
  log,
  path,
  yamlParse,
  yamlStringify,
} from "./deps.ts";
import {
  ScriptMetadata,
  defaultScriptMetadata,
} from "./bootstrap/script_bootstrap.ts";
import {
  instantiate as instantiateWasm,
  parse_bash,
  parse_bigquery,
  parse_deno,
  parse_go,
  parse_graphql,
  parse_mssql,
  parse_mysql,
  parse_powershell,
  parse_python,
  parse_snowflake,
  parse_sql,
} from "./wasm/windmill_parser_wasm.generated.js";
import { Workspace } from "./workspace.ts";
import { SchemaProperty } from "./bootstrap/common.ts";
import { ScriptLanguage } from "./script_common.ts";
import { inferContentTypeFromFilePath } from "./script_common.ts";
import { GlobalDeps, exts } from "./script.ts";
import {
  FSFSElement,
  extractInlineScriptsForFlows,
  findCodebase,
  newPathAssigner,
  yamlOptions,
} from "./sync.ts";
import { generateHash, readInlinePathSync } from "./utils.ts";
import { SyncCodebase } from "./codebase.ts";
import { FlowFile, replaceInlineScripts } from "./flow.ts";

export async function generateAllMetadata() {}

function findClosestRawReqs(
  lang: "bun" | "python3" | "php" | undefined,
  remotePath: string,
  globalDeps: GlobalDeps
): string | undefined {
  let bestCandidate: { k: string; v: string } | undefined = undefined;
  if (lang == "bun") {
    Object.entries(globalDeps.pkgs).forEach(([k, v]) => {
      if (
        remotePath.startsWith(k) &&
        k.length >= (bestCandidate?.k ?? "").length
      ) {
        bestCandidate = { k, v };
      }
    });
  } else if (lang == "python3") {
    Object.entries(globalDeps.reqs).forEach(([k, v]) => {
      if (
        remotePath.startsWith(k) &&
        k.length >= (bestCandidate?.k ?? "").length
      ) {
        bestCandidate = { k, v };
      }
    });
  } else if (lang == "php") {
    Object.entries(globalDeps.composers).forEach(([k, v]) => {
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
async function generateFlowHash(folder: string) {
  const elems = await FSFSElement(path.join(Deno.cwd(), folder), []);
  const hashes: Record<string, string> = {};
  for await (const f of elems.getChildren()) {
    if (exts.some((e) => f.path.endsWith(e))) {
      hashes[f.path] = await generateHash(await f.getContentText());
    }
  }
  return { ...hashes, [TOP_HASH]: await generateHash(JSON.stringify(hashes)) };
}
export async function generateFlowLockInternal(
  folder: string,
  dryRun: boolean,
  workspace: Workspace,
  justUpdateMetadataLock?: boolean
): Promise<string | undefined> {
  if (folder.endsWith(SEP)) {
    folder = folder.substring(0, folder.length - 1);
  }
  log.info(folder);
  const remote_path = folder
    .replaceAll(SEP, "/")
    .substring(0, folder.length - ".flow".length);
  if (!justUpdateMetadataLock) {
    log.info(`Generating lock for flow ${folder} at ${remote_path}`);
  }

  let hashes = await generateFlowHash(folder);

  const conf = await readLockfile();
  if (await checkifMetadataUptodate(folder, hashes[TOP_HASH], conf, TOP_HASH)) {
    log.info(
      colors.green(`Flow ${remote_path} metadata is up-to-date, skipping`)
    );
    return;
  } else if (dryRun) {
    return remote_path;
  }

  const flowValue = yamlParse(
    await Deno.readTextFile(folder! + SEP + "flow.yaml")
  ) as FlowFile;

  if (!justUpdateMetadataLock) {
    const changedScripts = [];
    //find hashes that do not correspond to preivous hashes
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
      folder + SEP!,
      changedScripts
    );
    //removeChangedLocks
    flowValue.value = await updateFlow(workspace, flowValue.value, remote_path);

    const inlineScripts = extractInlineScriptsForFlows(
      flowValue.value.modules,
      newPathAssigner("bun")
    );
    inlineScripts
      .filter((s) => s.path.endsWith(".lock"))
      .forEach((s) => {
        Deno.writeTextFileSync(
          Deno.cwd() + SEP + folder + SEP + s.path,
          s.content
        );
      });
  }

  hashes = await generateFlowHash(folder);

  for (const [path, hash] of Object.entries(hashes)) {
    await updateMetadataLock(folder, hash, path);
  }
  log.info(colors.green(`Flow ${remote_path} lockfiles updated`));
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

  const rawReqs = findClosestRawReqs(
    language as "bun" | "python3" | "php" | undefined,
    scriptPath,
    globalDeps
  );
  if (rawReqs) {
    log.info(
      colors.blue(
        `Found raw requirements (package.json/requirements.txt/composer.json) for ${scriptPath}, using it`
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
    const c = findCodebase(scriptPath, codebases);
    if (!c) {
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
    metadataParsedContent.lock = "!inline " + remotePath + ".script.lock";
  }

  let metaPath = remotePath + ".script.yaml";
  let newMetadataContent = yamlStringify(metadataParsedContent, yamlOptions);
  if (metadataWithType.isJson) {
    metaPath = remotePath + ".script.json";
    newMetadataContent = JSON.stringify(metadataParsedContent);
  }
  hash = await generateScriptHash(rawReqs, scriptContent, metadataContent);
  await updateMetadataLock(remotePath, hash);
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
  await instantiateWasm();
  const newSchema = inferSchema(
    language,
    scriptContent,
    metadataContent.schema,
    path
  );
  metadataContent.schema = newSchema;
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
      language == "bun" ||
      language == "python3" ||
      language == "go" ||
      language == "deno" ||
      language == "php"
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
      throw new Error(
        `Failed to generate lockfile. Full response was: ${JSON.stringify(
          response
        )}`
      );
    }
    const lockPath = remotePath + ".script.lock";
    await Deno.writeTextFile(lockPath, lock);
    metadataContent.lock = "!inline " + lockPath.replaceAll(SEP, "/");
  } catch (e) {
    throw new Error(
      `Failed to generate lockfile. Status was: ${rawResponse.statusText}, ${responseText}, ${e}`
    );
  }
}

export async function updateFlow(
  workspace: Workspace,
  flow_value: FlowValue,
  remotePath: string
): Promise<FlowValue | undefined> {
  // generate the script lock running a dependency job in Windmill and update it inplace
  // TODO: update this once the client is released
  const rawResponse = await fetch(
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

  let responseText = "reading response failed";
  try {
    const res = await rawResponse.json();
    return res?.["updated_flow_value"];
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
export function inferSchema(
  language: ScriptLanguage,
  content: string,
  currentSchema: any,
  path: string
) {
  let inferedSchema: any;
  if (language === "python3") {
    inferedSchema = JSON.parse(parse_python(content));
  } else if (language === "nativets") {
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "bun") {
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "deno") {
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "go") {
    inferedSchema = JSON.parse(parse_go(content));
  } else if (language === "mysql") {
    inferedSchema = JSON.parse(parse_mysql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "mysql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "bigquery") {
    inferedSchema = JSON.parse(parse_bigquery(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "bigquery" } },
      ...inferedSchema.args,
    ];
  } else if (language === "snowflake") {
    inferedSchema = JSON.parse(parse_snowflake(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "snowflake" } },
      ...inferedSchema.args,
    ];
  } else if (language === "mssql") {
    inferedSchema = JSON.parse(parse_mssql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "ms_sql_server" } },
      ...inferedSchema.args,
    ];
  } else if (language === "postgresql") {
    inferedSchema = JSON.parse(parse_sql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "postgresql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "graphql") {
    inferedSchema = JSON.parse(parse_graphql(content));
    inferedSchema.args = [
      { name: "api", typ: { resource: "graphql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "bash") {
    inferedSchema = JSON.parse(parse_bash(content));
  } else if (language === "powershell") {
    inferedSchema = JSON.parse(parse_powershell(content));
  } else {
    throw new Error("Invalid language: " + language);
  }
  if (inferedSchema.type == "Invalid") {
    log.info(
      colors.yellow(
        `Script ${path} invalid, it cannot be parsed to infer schema.`
      )
    );
    return defaultScriptMetadata().schema;
  }

  currentSchema.required = [];
  const oldProperties = JSON.parse(JSON.stringify(currentSchema.properties));
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

  return currentSchema;
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

export function argSigToJsonSchemaType(
  t:
    | string
    | { resource: string | null }
    | {
        list:
          | string
          | { str: any }
          | { object: { key: string; typ: any }[] }
          | null;
      }
    | { str: string[] | null }
    | { object: { key: string; typ: any }[] },
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
  } else if (t === "datetime") {
    newS.type = "string";
    newS.format = "date-time";
  } else if (typeof t !== "string" && `object` in t) {
    newS.type = "object";
    if (t.object) {
      const properties: Record<string, any> = {};
      for (const prop of t.object) {
        properties[prop.key] = {};
        argSigToJsonSchemaType(prop.typ, properties[prop.key]);
      }
      newS.properties = properties;
    }
  } else if (typeof t !== "string" && `str` in t) {
    newS.type = "string";
    if (t.str) {
      newS.enum = t.str;
    }
  } else if (typeof t !== "string" && `resource` in t) {
    newS.type = "object";
    newS.format = `resource-${t.resource}`;
  } else if (typeof t !== "string" && `list` in t) {
    newS.type = "array";
    if (t.list === "int" || t.list === "float") {
      newS.items = { type: "number" };
    } else if (t.list === "bytes") {
      newS.items = { type: "string", contentEncoding: "base64" };
    } else if (t.list == "string") {
      newS.items = { type: "string" };
    } else if (t.list && typeof t.list == "object" && "str" in t.list) {
      newS.items = { type: "string", enum: t.list.str };
    } else {
      newS.items = { type: "object" };
    }
  } else {
    newS.type = "object";
  }

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

  Object.assign(oldS, newS);

  // if (sameItems && savedItems != undefined && savedItems.enum != undefined) {
  // 	sendUserToast(JSON.stringify(savedItems))
  // 	oldS.items = savedItems
  // }

  if (oldS.format?.startsWith("resource-") && newS.type != "object") {
    oldS.format = undefined;
  }
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
      const payload: any = yamlParse(await Deno.readTextFile(metadataFilePath));
      replaceLock(payload);

      return {
        path: metadataFilePath,
        payload,
        isJson: false,
      };
    } catch {
      // no metadata file at all. Create it
      log.info(
        colors.blue(`Creating script metadata file for ${metadataFilePath}`)
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
          colors.blue(`Generating lockfile and schema for ${metadataFilePath}`)
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
          scriptInitialMetadata = yamlParse(
            await Deno.readTextFile(metadataFilePath)
          ) as ScriptMetadata;
          replaceLock(scriptInitialMetadata);
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
    const lockfile = await Deno.readTextFile(WMILL_LOCKFILE);
    const read = yamlParse(lockfile);
    if (typeof read == "object") {
      return read as Lock;
    } else {
      throw new Error("Invalid lockfile");
    }
  } catch {
    const lock = { locks: {} };
    await Deno.writeTextFile(WMILL_LOCKFILE, yamlStringify(lock, yamlOptions));
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

export async function updateMetadataLock(
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
