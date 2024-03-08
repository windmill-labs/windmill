// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { colors, encodeHex, log, yamlParse, yamlStringify } from "./deps.ts";
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
import { GlobalDeps } from "./script.ts";
import { yamlOptions } from "./sync.ts";

export async function generateAllMetadata() {}

function findClosestRawReqs(
  lang: "bun" | "python3" | undefined,
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
  }
  // @ts-ignore
  return bestCandidate?.v;
}

export async function generateMetadataInternal(
  scriptPath: string,
  workspace: Workspace,
  opts: GlobalOptions & {
    lockOnly?: boolean | undefined;
    schemaOnly?: boolean | undefined;
    defaultTs?: "bun" | "deno";
  },
  dryRun: boolean,
  noStaleMessage: boolean,
  globalDeps: GlobalDeps
): Promise<string | undefined> {
  const remotePath = scriptPath
    .substring(0, scriptPath.indexOf("."))
    .replaceAll("\\", "/");

  const language = inferContentTypeFromFilePath(scriptPath, opts.defaultTs);

  const rawReqs = findClosestRawReqs(
    language as "bun" | "python3" | undefined,
    scriptPath,
    globalDeps
  );
  if (rawReqs) {
    log.info(
      colors.blue(
        `Found raw requirements (package.json/requirements.txt) for ${scriptPath}, using it`
      )
    );
  }
  const metadataWithType = await parseMetadataFile(
    remotePath,
    undefined,
    globalDeps
  );

  // read script content
  const scriptContent = await Deno.readTextFile(scriptPath);
  const metadataContent = await Deno.readTextFile(metadataWithType.path);
  if (
    await checkifMetadataUptodate(
      remotePath,
      (rawReqs ?? "") + scriptContent + metadataContent
    )
  ) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`Script ${remotePath} metadata is up-to-date, skipping`)
      );
    }
    return;
  } else if (dryRun) {
    return `${remotePath} (${language})`;
  }

  log.info(colors.gray(`Generating metadata for ${scriptPath}`));

  const metadataParsedContent = metadataWithType?.payload as Record<
    string,
    any
  >;

  if (!opts.lockOnly) {
    await updateScriptSchema(
      scriptContent,
      language,
      metadataParsedContent,
      scriptPath
    );
  }

  if (!opts.schemaOnly) {
    await updateScriptLock(
      workspace,
      scriptContent,
      language,
      remotePath,
      metadataParsedContent,
      rawReqs
    );
  }

  let metaPath = remotePath + ".script.yaml";
  let newMetadataContent = yamlStringify(metadataParsedContent, yamlOptions);
  if (metadataWithType.isJson) {
    metaPath = remotePath + ".script.json";
    newMetadataContent = JSON.stringify(metadataParsedContent);
  }
  await updateMetadataLock(
    remotePath,
    (rawReqs ?? "") + scriptContent + newMetadataContent
  );
  await Deno.writeTextFile(metaPath, newMetadataContent);
  return `${remotePath} (${language})`;
}

async function updateScriptSchema(
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
    metadataContent.lock = lock;
  } catch (e) {
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

function argSigToJsonSchemaType(
  typ:
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
  if (typ === "int") {
    newS.type = "integer";
  } else if (typ === "float") {
    newS.type = "number";
  } else if (typ === "bool") {
    newS.type = "boolean";
  } else if (typ === "email") {
    newS.type = "string";
    newS.format = "email";
  } else if (typ === "sql") {
    newS.type = "string";
    newS.format = "sql";
  } else if (typ === "yaml") {
    newS.type = "string";
    newS.format = "yaml";
  } else if (typ === "bytes") {
    newS.type = "string";
    newS.contentEncoding = "base64";
  } else if (typ === "datetime") {
    newS.type = "string";
    newS.format = "date-time";
  } else if (typeof typ !== "string" && `object` in typ) {
    newS.type = "object";
    if (typ.object) {
      const properties: Record<string, SchemaProperty> = {};
      for (const prop of typ.object) {
        properties[prop.key] = { type: undefined };
        argSigToJsonSchemaType(prop.typ, properties[prop.key]);
      }
      newS.properties = properties;
    }
  } else if (typeof typ !== "string" && `str` in typ) {
    newS.type = "string";
    if (typ.str) {
      newS.enum = typ.str;
    }
  } else if (typeof typ !== "string" && `resource` in typ) {
    newS.type = "object";
    newS.format = `resource-${typ.resource}`;
  } else if (typeof typ !== "string" && `list` in typ) {
    newS.type = "array";
    if (typ.list === "int" || typ.list === "float") {
      newS.items = { type: "number" };
    } else if (typ.list === "bytes") {
      newS.items = { type: "string", contentEncoding: "base64" };
    } else if (typ.list == "string") {
      newS.items = { type: "string" };
    } else if (typ.list && typeof typ.list == "object" && "str" in typ.list) {
      newS.items = { type: "string", enum: typ.list.str };
    } else {
      newS.items = { type: "object" };
    }
  } else {
    newS.type = "object";
  }

  if (oldS.type != newS.type) {
    for (const prop of Object.getOwnPropertyNames(newS)) {
      if (prop != "description") {
        // @ts-ignore: fix
        delete oldS[prop];
      }
    }
  } else if (oldS.format == "date-time" && newS.format != "date-time") {
    delete oldS.format;
  } else if (oldS.items?.type != newS.items?.type) {
    delete oldS.items;
  }

  Object.assign(oldS, newS);
  if (oldS.format?.startsWith("resource-") && newS.type != "object") {
    oldS.format = undefined;
  }
}
////////////////////////////////////////////////////////////////////////////////////////////
// end of refactoring TODO                                                                //
////////////////////////////////////////////////////////////////////////////////////////////

export async function parseMetadataFile(
  scriptPath: string,
  generateMetadataIfMissing:
    | (GlobalOptions & { path: string; workspaceRemote: Workspace })
    | undefined,
  globalDeps: GlobalDeps
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
      if (Array.isArray(payload?.["lock"])) {
        payload["lock"] = payload["lock"].join("\n");
      }
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
      Deno.writeTextFile(metadataFilePath, scriptInitialMetadataYaml, {
        createNew: true,
      });

      if (generateMetadataIfMissing) {
        log.info(
          colors.blue(`Generating lockfile and schema for ${metadataFilePath}`)
        );
        try {
          await generateMetadataInternal(
            generateMetadataIfMissing.path,
            generateMetadataIfMissing.workspaceRemote,
            generateMetadataIfMissing,
            false,
            false,
            globalDeps
          );

          scriptInitialMetadata = yamlParse(
            await Deno.readTextFile(metadataFilePath)
          ) as ScriptMetadata;
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
  locks?: { [path: string]: string };
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

async function generateHash(content: string): Promise<string> {
  const messageBuffer = new TextEncoder().encode(content);
  const hashBuffer = await crypto.subtle.digest("SHA-256", messageBuffer);
  return encodeHex(hashBuffer);
}

export async function checkifMetadataUptodate(
  path: string,
  requirement: string
) {
  const conf = await readLockfile();
  if (!conf.locks) {
    return false;
  }
  const hash = await generateHash(requirement);
  return conf?.locks?.[path] == hash;
}

export async function updateMetadataLock(
  path: string,
  requirement: string
): Promise<void> {
  const conf = await readLockfile();
  const hash = await generateHash(requirement);
  if (!conf?.locks) {
    conf.locks = {};
  }

  conf.locks[path] = hash;
  await Deno.writeTextFile(
    WMILL_LOCKFILE,
    yamlStringify(conf as Record<string, any>, yamlOptions)
  );
}
