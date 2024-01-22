// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  JobService,
  log,
  NewScript,
  readAll,
  Script,
  ScriptService,
  Table,
  writeAllSync,
  yamlParse,
  yamlStringify,
} from "./deps.ts";
import { deepEqual } from "./utils.ts";
import {
  ScriptMetadata,
  defaultScriptMetadata,
  scriptBootstrapCode,
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

export interface ScriptFile {
  parent_hash?: string;
  summary: string;
  description: string;
  schema?: any;
  is_template?: boolean;
  lock?: Array<string>;
  kind?: "script" | "failure" | "trigger" | "command" | "approval";
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string) {
  const workspace = await resolveWorkspace(opts);

  if (!validatePath(filePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    throw Error(
      "Cannot push a script metadata file, point to the script content file instead (.py, .ts, .go|.sh)"
    );
  }

  await requireLogin(opts);
  await handleFile(filePath, workspace.workspaceId, [], undefined, false, opts);
  log.info(colors.bold.underline.green(`Script ${filePath} pushed`));
}

export async function handleScriptMetadata(
  path: string,
  workspace: string,
  alreadySynced: string[],
  message: string | undefined,
  lockfileUseArray: boolean
): Promise<boolean> {
  if (path.endsWith(".script.json") || path.endsWith(".script.yaml")) {
    const contentPath = await findContentFile(path);
    return handleFile(
      contentPath,
      workspace,
      alreadySynced,
      message,
      lockfileUseArray,
      undefined
    );
  } else {
    return false;
  }
}

async function parseMetadataFile(
  scriptPath: string,
  generateMetadataIfMissing: (GlobalOptions & { path: string }) | undefined
): Promise<{ isJson: boolean; payload: any } | undefined> {
  let metadataFilePath = scriptPath + ".script.json";
  try {
    await Deno.stat(metadataFilePath);
    return {
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
        scriptInitialMetadata as Record<string, any>
      );
      Deno.writeTextFile(metadataFilePath, scriptInitialMetadataYaml, {
        createNew: true,
      });

      if (generateMetadataIfMissing) {
        log.info(
          colors.blue(`Generating lockfile and schema for ${metadataFilePath}`)
        );
        try {
          await generateMetadata(
            generateMetadataIfMissing,
            generateMetadataIfMissing.path
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
        payload: scriptInitialMetadata,
        isJson: false,
      };
    }
  }
}

export async function handleFile(
  path: string,
  workspace: string,
  alreadySynced: string[],
  message: string | undefined,
  lockfileUseArray: boolean,
  opts: GlobalOptions | undefined
): Promise<boolean> {
  if (
    !path.includes(".inline_script.") &&
    (path.endsWith(".ts") ||
      path.endsWith(".py") ||
      path.endsWith(".go") ||
      path.endsWith(".sh") ||
      path.endsWith(".sql") ||
      path.endsWith(".gql") ||
      path.endsWith(".ps1"))
  ) {
    if (alreadySynced.includes(path)) {
      return true;
    }
    log.debug(`Processing local script ${path}`);

    alreadySynced.push(path);
    const remotePath = path
      .substring(0, path.indexOf("."))
      .replaceAll("\\", "/");
    const typed = (
      await parseMetadataFile(remotePath, opts ? { ...opts, path } : undefined)
    )?.payload;
    const language = inferContentTypeFromFilePath(path);

    let remote = undefined;
    try {
      remote = await ScriptService.getScriptByPath({
        workspace,
        path: remotePath.replaceAll("\\", "/"),
      });
      log.debug(`Script ${remotePath} exists on remote`);
    } catch {
      log.debug(`Script ${remotePath} does not exist on remote`);
    }
    const content = await Deno.readTextFile(path);

    if (remote) {
      if (content === remote.content) {
        if (
          typed == undefined ||
          (typed.description === remote.description &&
            typed.summary === remote.summary &&
            (typed.is_template ?? false) === (remote.is_template ?? false) &&
            typed.kind == remote.kind &&
            !remote.archived &&
            (Array.isArray(remote?.lock)
              ? remote?.lock?.join("\n")
              : remote?.lock ?? ""
            ).trim() == (typed?.lock ?? "").trim() &&
            deepEqual(typed.schema, remote.schema) &&
            typed.tag == remote.tag &&
            (typed.ws_error_handler_muted ?? false) ==
              remote.ws_error_handler_muted &&
            typed.dedicated_worker == remote.dedicated_worker &&
            typed.cache_ttl == remote.cache_ttl &&
            typed.concurrency_time_window_s ==
              remote.concurrency_time_window_s &&
            typed.concurrent_limit == remote.concurrent_limit)
        ) {
          log.info(colors.green(`Script ${remotePath} is up to date`));
          return true;
        }
      }

      log.info(
        colors.yellow.bold(`Creating script with a parent ${remotePath}`)
      );
      await ScriptService.createScript({
        workspace,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language: language as NewScript.language,
          path: remotePath.replaceAll("\\", "/"),
          summary: typed?.summary ?? "",
          is_template: typed?.is_template,
          kind: typed?.kind,
          lock: lockfileUseArray ? typed?.lock.split("\n") : typed?.lock,
          parent_hash: remote.hash,
          schema: typed?.schema,
          tag: typed?.tag,
          ws_error_handler_muted: typed?.ws_error_handler_muted,
          dedicated_worker: typed?.dedicated_worker,
          cache_ttl: typed?.cache_ttl,
          concurrency_time_window_s: typed?.concurrency_time_window_s,
          concurrent_limit: typed?.concurrent_limit,
          deployment_message: message,
        },
      });
    } else {
      log.info(
        colors.yellow.bold(`Creating script without parent ${remotePath}`)
      );
      // no parent hash
      await ScriptService.createScript({
        workspace: workspace,
        requestBody: {
          content,
          description: typed?.description ?? "",
          language: language as NewScript.language,
          path: remotePath.replaceAll("\\", "/"),
          summary: typed?.summary ?? "",
          is_template: typed?.is_template,
          kind: typed?.kind,
          lock: lockfileUseArray ? typed?.lock.split("\n") : typed?.lock,
          parent_hash: undefined,
          schema: typed?.schema,
          tag: typed?.tag,
          ws_error_handler_muted: typed?.ws_error_handler_muted,
          dedicated_worker: typed?.dedicated_worker,
          cache_ttl: typed?.cache_ttl,
          concurrency_time_window_s: typed?.concurrency_time_window_s,
          concurrent_limit: typed?.concurrent_limit,
          deployment_message: message,
        },
      });
    }
    return true;
  }
  return false;
}

export async function findContentFile(filePath: string) {
  const candidates = filePath.endsWith("script.json")
    ? [
        filePath.replace(".script.json", ".fetch.ts"),
        filePath.replace(".script.json", ".bun.ts"),
        filePath.replace(".script.json", ".ts"),
        filePath.replace(".script.json", ".py"),
        filePath.replace(".script.json", ".go"),
        filePath.replace(".script.json", ".sh"),
        filePath.replace(".script.json", "pg.sql"),
        filePath.replace(".script.json", "my.sql"),
        filePath.replace(".script.json", "bq.sql"),
        filePath.replace(".script.json", "sf.sql"),
        filePath.replace(".script.json", ".gql"),
        filePath.replace(".script.json", ".ps1"),
      ]
    : [
        filePath.replace(".script.yaml", ".fetch.ts"),
        filePath.replace(".script.yaml", ".bun.ts"),
        filePath.replace(".script.yaml", ".ts"),
        filePath.replace(".script.yaml", ".py"),
        filePath.replace(".script.yaml", ".go"),
        filePath.replace(".script.yaml", ".sh"),
        filePath.replace(".script.yaml", "pg.sql"),
        filePath.replace(".script.yaml", "bq.sql"),
        filePath.replace(".script.yaml", "sf.sql"),
        filePath.replace(".script.yaml", ".gql"),
        filePath.replace(".script.yaml", ".ps1"),
      ];
  const validCandidates = (
    await Promise.all(
      candidates.map((x) => {
        return Deno.stat(x)
          .catch(() => undefined)
          .then((x) => x?.isFile)
          .then((e) => {
            return { path: x, file: e };
          });
      })
    )
  )
    .filter((x) => x.file)
    .map((x) => x.path);
  if (validCandidates.length > 1) {
    throw new Error(
      "No content path given and more than one candidate found: " +
        validCandidates.join(", ")
    );
  }
  if (validCandidates.length < 1) {
    throw new Error(
      `No content path given and no content file found for ${filePath}.`
    );
  }
  return validCandidates[0];
}

type ScriptLanguage =
  | "python3"
  | "deno"
  | "bun"
  | "nativets"
  | "go"
  | "bash"
  | "powershell"
  | "postgresql"
  | "mysql"
  | "bigquery"
  | "snowflake"
  | "mssql"
  | "graphql";

export function filePathExtensionFromContentType(
  language: ScriptLanguage
): string {
  if (language === "python3") {
    return ".py";
  } else if (language === "nativets") {
    return ".fetch.ts";
  } else if (language === "bun") {
    return ".bun.ts";
  } else if (language === "deno") {
    return ".ts";
  } else if (language === "go") {
    return ".go";
  } else if (language === "mysql") {
    return ".my.sql";
  } else if (language === "bigquery") {
    return ".bq.sql";
  } else if (language === "snowflake") {
    return ".sf.sql";
  } else if (language === "mssql") {
    return ".ms.sql";
  } else if (language === "postgresql") {
    return ".pg.sql";
  } else if (language === "graphql") {
    return ".gql";
  } else if (language === "bash") {
    return ".sh";
  } else if (language === "powershell") {
    return ".ps1";
  } else {
    throw new Error("Invalid language: " + language);
  }
}

export const listValidExtensions = [
  ".py",
  ".bun.ts",
  ".fetch.ts",
  ".ts",
  ".go",
  ".sh",
  ".my.sql",
  ".bq.sql",
  "ms.sql",
  ".pg.sql",
  ".sql",
  ".gql",
  ".ps1",
];

export function removeExtensionToPath(path: string): string {
  for (const ext of listValidExtensions) {
    if (path.endsWith(ext)) {
      return path.substring(0, path.length - ext.length);
    }
  }
  throw new Error("Invalid extension: " + path);
}

export function inferContentTypeFromFilePath(
  contentPath: string
): ScriptLanguage {
  if (contentPath.endsWith(".py")) {
    return "python3";
  } else if (contentPath.endsWith("fetch.ts")) {
    return "nativets";
  } else if (contentPath.endsWith("bun.ts")) {
    return "bun";
  } else if (contentPath.endsWith(".ts")) {
    return "deno";
  } else if (contentPath.endsWith(".go")) {
    return "go";
  } else if (contentPath.endsWith(".my.sql")) {
    return "mysql";
  } else if (contentPath.endsWith(".bq.sql")) {
    return "bigquery";
  } else if (contentPath.endsWith(".sf.sql")) {
    return "snowflake";
  } else if (contentPath.endsWith(".ms.sql")) {
    return "mssql";
  } else if (contentPath.endsWith(".pg.sql")) {
    return "postgresql";
  } else if (contentPath.endsWith(".gql")) {
    return "graphql";
  } else if (contentPath.endsWith(".sh")) {
    return "bash";
  } else if (contentPath.endsWith(".ps1")) {
    return "powershell";
  } else {
    throw new Error(
      "Invalid language: " + contentPath.substring(contentPath.lastIndexOf("."))
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

async function list(opts: GlobalOptions & { showArchived?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: Script[] = [];
  while (true) {
    const res = await ScriptService.listScripts({
      workspace: workspace.workspaceId,
      page,
      perPage,
      showArchived: opts.showArchived ?? false,
    });
    page += 1;
    total.push(...res);
    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["path", "summary", "language", "created by"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.summary, x.language, x.created_by]))
    .render();
}

export async function resolve(input: string): Promise<Record<string, any>> {
  if (!input) {
    throw new Error("No data given");
  }

  if (input == "@-") {
    input = new TextDecoder().decode(await readAll(Deno.stdin));
  }
  if (input[0] == "@") {
    input = await Deno.readTextFile(input.substring(1));
  }
  try {
    return JSON.parse(input);
  } catch (e) {
    console.error("Impossible to parse input as JSON", input);
    throw e;
  }
}

async function run(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  },
  path: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = opts.data ? await resolve(opts.data) : {};
  const id = await JobService.runScriptByPath({
    workspace: workspace.workspaceId,
    path,
    requestBody: input,
  });

  if (!opts.silent) {
    await track_job(workspace.workspaceId, id);
  }

  while (true) {
    try {
      const result =
        (
          await JobService.getCompletedJob({
            workspace: workspace.workspaceId,
            id,
          })
        ).result ?? {};
      log.info(result);

      break;
    } catch {
      new Promise((resolve, _) => setTimeout(() => resolve(undefined), 100));
    }
  }
}

export async function track_job(workspace: string, id: string) {
  try {
    const result = await JobService.getCompletedJob({ workspace, id });

    log.info(result.logs);
    log.info("\n");
    log.info(colors.bold.underline.green("Job Completed"));
    log.info("\n");
    return;
  } catch {
    /* ignore */
  }

  log.info(colors.yellow("Waiting for Job " + id + " to start..."));

  let logOffset = 0;
  let running = false;
  let retry = 0;
  while (true) {
    let updates: {
      running?: boolean | undefined;
      completed?: boolean | undefined;
      new_logs?: string | undefined;
    };
    try {
      updates = await JobService.getJobUpdates({
        workspace,
        id,
        logOffset,
        running,
      });
    } catch {
      retry++;
      if (retry > 3) {
        log.info("failed to get job updated. skipping log streaming.");
        break;
      }
      continue;
    }

    if (!running && updates.running === true) {
      running = true;
      log.info(colors.green("Job running. Streaming logs..."));
    }

    if (updates.new_logs) {
      writeAllSync(Deno.stdout, new TextEncoder().encode(updates.new_logs));
      logOffset += updates.new_logs.length;
    }

    if (updates.completed === true) {
      running = false;
      break;
    }

    if (running && updates.running === false) {
      running = false;
      log.info(colors.yellow("Job suspended. Waiting for it to continue..."));
    }
  }
  await new Promise((resolve, _) => setTimeout(() => resolve(undefined), 1000));

  try {
    const final_job = await JobService.getCompletedJob({ workspace, id });
    if ((final_job.logs?.length ?? -1) > logOffset) {
      log.info(final_job.logs!.substring(logOffset));
    }
    log.info("\n");
    if (final_job.success) {
      log.info(colors.bold.underline.green("Job Completed"));
    } else {
      log.info(colors.bold.underline.red("Job Completed"));
    }
    log.info("\n");
  } catch {
    log.info("Job appears to have completed, but no data can be retrieved");
  }
}

async function show(opts: GlobalOptions, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const s = await ScriptService.getScriptByPath({
    workspace: workspace.workspaceId,
    path,
  });
  log.info(colors.underline(s.path));
  if (s.description) log.info(s.description);
  log.info("");
  log.info(s.content);
}

async function bootstrap(
  opts: GlobalOptions & { summary: string; description: string },
  scriptPath: string,
  language: ScriptLanguage
) {
  if (!validatePath(scriptPath)) {
    return;
  }

  const scriptInitialCode = scriptBootstrapCode[language];
  if (scriptInitialCode === undefined) {
    throw new Error("Language unknown");
  }

  const extension = filePathExtensionFromContentType(language);
  const scriptCodeFileFullPath = scriptPath + extension;
  const scriptMetadataFileFullPath = scriptPath + ".script.yaml";

  try {
    await Deno.stat(scriptCodeFileFullPath);
    await Deno.stat(scriptMetadataFileFullPath);
    throw new Error("File already exists in repository");
  } catch {
    // file does not exist, we can continue
  }

  const scriptMetadata = defaultScriptMetadata();
  if (opts.summary !== undefined) {
    scriptMetadata.summary = opts.summary;
  }
  if (opts.description !== undefined) {
    scriptMetadata.description = opts.description;
  }

  const scriptInitialMetadataYaml = yamlStringify(
    scriptMetadata as Record<string, any>
  );

  Deno.writeTextFile(scriptCodeFileFullPath, scriptInitialCode, {
    createNew: true,
  });
  Deno.writeTextFile(scriptMetadataFileFullPath, scriptInitialMetadataYaml, {
    createNew: true,
  });
}

async function generateMetadata(
  opts: GlobalOptions & { lockOnly?: boolean; schemaOnly?: boolean },
  scriptPath: string
) {
  if (!validatePath(scriptPath)) {
    return;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // read script metadata file
  const remotePath = scriptPath
    .substring(0, scriptPath.indexOf("."))
    .replaceAll("\\", "/");
  const metadataWithType = await parseMetadataFile(remotePath, undefined);
  if (metadataWithType === undefined) {
    throw new Error("Script metadata file does not exist at this path");
  }

  // read script content
  const scriptContent = await Deno.readTextFile(scriptPath);

  const metadataParsedContent = metadataWithType?.payload as Record<
    string,
    any
  >;
  
  const language = inferContentTypeFromFilePath(scriptPath);
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
      metadataParsedContent
    );
  }

  let metaPath = remotePath + ".script.yaml";
  let newMetadataContent = yamlStringify(metadataParsedContent);
  if (metadataWithType.isJson) {
    metaPath = remotePath + ".script.json";
    newMetadataContent = JSON.stringify(metadataParsedContent);
  }
  await Deno.writeTextFile(metaPath, newMetadataContent);
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
  metadataContent: Record<string, any>
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
        entrypoint: remotePath,
      }),
    }
  );

  try {
    const response = await rawResponse.json();
    const lock = response.lock;
    if (lock === undefined) {
      throw new Error(
        `Failed to generate lockfile. Full response was: ${JSON.stringify(
          response
        )}`
      );
    }
    metadataContent.lock = lock;
  } catch {
    throw new Error(
      `Failed to generate lockfile. Status was: ${rawResponse.statusText}`
    );
  }
}

const command = new Command()
  .description("script related commands")
  .option("--show-archived", "Enable archived scripts in output")
  .action(list as any)
  .command(
    "push",
    "push a local script spec. This overrides any remote versions. Use the script file (.ts, .js, .py, .sh)"
  )
  .arguments("<path:file>")
  .action(push as any)
  .command("show", "show a scripts content")
  .arguments("<path:file>")
  .action(show as any)
  .command("run", "run a script by path")
  .arguments("<path:file>")
  .option(
    "-d --data <data:file>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not output anything other then the final output. Useful for scripting."
  )
  .action(run as any)
  .command("bootstrap", "create a new script")
  .arguments("<path:string> <language:string>")
  .option("--summary <summary:string>", "script summary")
  .option("--description <description:string>", "script description")
  .action(bootstrap as any)
  .command(
    "generate-metadata",
    "re-generate the metadata file updating the lock and the script schema"
  )
  .arguments("<path_to_script_file:string>")
  .option("--lock-only", "re-generate only the lock")
  .option("--schema-only", "re-generate only script schema")
  .action(generateMetadata as any);

export default command;
