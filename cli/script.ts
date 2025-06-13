// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  colors,
  Command,
  Confirm,
  log,
  readAll,
  SEP,
  Table,
  writeAllSync,
  yamlStringify,
} from "./deps.ts";
import { deepEqual } from "./utils.ts";
import * as wmill from "./gen/services.gen.ts";

import {
  defaultScriptMetadata,
  scriptBootstrapCode,
} from "./bootstrap/script_bootstrap.ts";

import { Workspace } from "./workspace.ts";
import {
  generateScriptMetadataInternal,
  parseMetadataFile,
} from "./metadata.ts";
import {
  ScriptLanguage,
  inferContentTypeFromFilePath,
} from "./script_common.ts";
import {
  elementsToMap,
  findCodebase,
  readDirRecursiveWithIgnore,
  Skips,
  yamlOptions,
} from "./sync.ts";
import { ignoreF } from "./sync.ts";
import { FSFSElement } from "./sync.ts";
import {
  SyncOptions,
  mergeConfigWithConfigFile,
  readConfigFile,
} from "./conf.ts";
import { SyncCodebase, listSyncCodebases } from "./codebase.ts";
import fs from "node:fs";
import { type Tarball } from "npm:@ayonli/jsext/archive";

import { execSync } from "node:child_process";
import { NewScript, Script } from "./gen/types.gen.ts";

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
  opts = await mergeConfigWithConfigFile(opts);
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
  const codebases = await listSyncCodebases(opts as SyncOptions);

  const globalDeps = await findGlobalDeps();
  await handleFile(
    filePath,
    workspace,
    [],
    undefined,
    opts,
    globalDeps,
    codebases
  );
  log.info(colors.bold.underline.green(`Script ${filePath} pushed`));
}

export async function findResourceFile(path: string) {
  const splitPath = path.split(".");

  const contentBasePathJSON = splitPath[0] + "." + splitPath[1] + ".json";
  const contentBasePathYAML = splitPath[0] + "." + splitPath[1] + ".yaml";

  const validCandidates = (
    await Promise.all(
      [contentBasePathJSON, contentBasePathYAML].map((x) => {
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
      "Found two resource files for the same resource" +
        validCandidates.join(", ")
    );
  }
  if (validCandidates.length < 1) {
    throw new Error(`No resource matching file resource: ${path}.`);
  }
  return validCandidates[0];
}

export async function handleScriptMetadata(
  path: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  globalDeps: GlobalDeps,
  codebases: SyncCodebase[],
  opts: GlobalOptions
): Promise<boolean> {
  if (
    path.endsWith(".script.json") ||
    path.endsWith(".script.yaml") ||
    path.endsWith(".script.lock")
  ) {
    const contentPath = await findContentFile(path);
    return handleFile(
      contentPath,
      workspace,
      alreadySynced,
      message,
      opts,
      globalDeps,
      codebases
    );
  } else {
    return false;
  }
}

export async function handleFile(
  path: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  opts: (GlobalOptions & { defaultTs?: "bun" | "deno" } & Skips) | undefined,
  globalDeps: GlobalDeps,
  codebases: SyncCodebase[]
): Promise<boolean> {
  if (
    !path.includes(".inline_script.") &&
    exts.some((exts) => path.endsWith(exts))
  ) {
    if (alreadySynced.includes(path)) {
      return true;
    }
    log.debug(`Processing local script ${path}`);

    alreadySynced.push(path);
    const remotePath = path
      .substring(0, path.indexOf("."))
      .replaceAll(SEP, "/");

    const language = inferContentTypeFromFilePath(path, opts?.defaultTs);

    const codebase =
      language == "bun" ? findCodebase(path, codebases) : undefined;

    let bundleContent: string | Tarball | undefined = undefined;

    if (codebase) {
      if (codebase.customBundler) {
        log.info(`Using custom bundler ${codebase.customBundler} for ${path}`);
        bundleContent = execSync(
          codebase.customBundler + " " + path
        ).toString();
        log.info("Custom bundler executed for " + path);
      } else {
        const esbuild = await import("npm:esbuild");

        log.info(`Started bundling ${path} ...`);
        const startTime = performance.now();
        const out = await esbuild.build({
          entryPoints: [path],
          format: "cjs",
          bundle: true,
          write: false,
          external: codebase.external,
          inject: codebase.inject,
          define: codebase.define,
          platform: "node",
          packages: "bundle",
          target: "node20.15.1",
        });
        const endTime = performance.now();
        bundleContent = out.outputFiles[0].text;
        log.info(
          `Finished bundling ${path}: ${(bundleContent.length / 1024).toFixed(
            0
          )}kB (${(endTime - startTime).toFixed(0)}ms)`
        );
      }
      if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
        const archiveNpm = await import("npm:@ayonli/jsext/archive");
        log.info(
          `Using the following asset configuration for ${path}: ${JSON.stringify(
            codebase.assets
          )}`
        );
        const startTime = performance.now();
        const tarball = new archiveNpm.Tarball();
        tarball.append(
          new File([bundleContent], "main.js", { type: "text/plain" })
        );
        for (const asset of codebase.assets) {
          const data = fs.readFileSync(asset.from);
          const blob = new Blob([data], { type: "text/plain" });
          const file = new File([blob], asset.to);
          tarball.append(file);
        }
        const endTime = performance.now();
        log.info(
          `Finished creating tarball for ${path}: ${(
            tarball.size / 1024
          ).toFixed(0)}kB (${(endTime - startTime).toFixed(0)}ms)`
        );
        bundleContent = tarball;
      }
    }
    let typed = opts?.skipScriptsMetadata
      ? undefined
      : (
          await parseMetadataFile(
            remotePath,
            opts
              ? {
                  ...opts,
                  path,
                  workspaceRemote: workspace,
                  schemaOnly: codebase ? true : undefined,
                }
              : undefined,
            globalDeps,
            codebases
          )
        )?.payload;

    const workspaceId = workspace.workspaceId;

    let remote = undefined;
    try {
      remote = await wmill.getScriptByPath({
        workspace: workspaceId,
        path: remotePath,
      });
      log.debug(`Script ${remotePath} exists on remote`);
    } catch {
      log.debug(`Script ${remotePath} does not exist on remote`);
    }
    const content = await Deno.readTextFile(path);

    if (opts?.skipScriptsMetadata) {
      // if (codebase) {
      //   const typedBefore = JSON.parse(JSON.stringify(typed.schema));
      //   await updateScriptSchema(content, language, typed, path);
      //   if (typedBefore != typed.schema) {
      //     log.info(`Updated metadata for bundle ${path}`);
      //     showDiff(
      //       yamlStringify(typedBefore, yamlOptions),
      //       yamlStringify(typed.schema, yamlOptions)
      //     );
      //     await Deno.writeTextFile(
      //       remotePath + ".script.yaml",
      //       yamlStringify(typed as Record<string, any>, yamlOptions)
      //     );
      //   }
      // }
      // else {
      typed = structuredClone(remote);
      // }
    }

    if (typed && codebase) {
      typed.codebase = await codebase.getDigest();
    }

    const requestBodyCommon: NewScript = {
      content,
      description: typed?.description ?? "",
      language: language as NewScript["language"],
      path: remotePath.replaceAll(SEP, "/"),
      summary: typed?.summary ?? "",
      kind: typed?.kind,
      lock: typed?.lock,
      schema: typed?.schema,
      tag: typed?.tag,
      ws_error_handler_muted: typed?.ws_error_handler_muted,
      dedicated_worker: typed?.dedicated_worker,
      cache_ttl: typed?.cache_ttl,
      concurrency_time_window_s: typed?.concurrency_time_window_s,
      concurrent_limit: typed?.concurrent_limit,
      deployment_message: message,
      restart_unless_cancelled: typed?.restart_unless_cancelled,
      visible_to_runner_only: typed?.visible_to_runner_only,
      no_main_func: typed?.no_main_func,
      has_preprocessor: typed?.has_preprocessor,
      priority: typed?.priority,
      concurrency_key: typed?.concurrency_key,
      codebase: await codebase?.getDigest(),
      timeout: typed?.timeout,
      on_behalf_of_email: typed?.on_behalf_of_email,
    };

    // log.info(JSON.stringify(requestBodyCommon, null, 2))
    // log.info(JSON.stringify(opts, null, 2))
    if (remote) {
      if (content === remote.content) {
        if (
          typed == undefined ||
          (typed.description === remote.description &&
            typed.summary === remote.summary &&
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
            typed.concurrent_limit == remote.concurrent_limit &&
            Boolean(typed.restart_unless_cancelled) ==
              Boolean(remote.restart_unless_cancelled) &&
            Boolean(typed.visible_to_runner_only) ==
              Boolean(remote.visible_to_runner_only) &&
            Boolean(typed.no_main_func) == Boolean(remote.no_main_func) &&
            Boolean(typed.has_preprocessor) ==
              Boolean(remote.has_preprocessor) &&
            typed.priority == Boolean(remote.priority) &&
            typed.timeout == remote.timeout &&
            //@ts-ignore
            typed.concurrency_key == remote["concurrency_key"] &&
            typed.codebase == remote.codebase &&
            typed.on_behalf_of_email == remote.on_behalf_of_email)
        ) {
          log.info(colors.green(`Script ${remotePath} is up to date`));
          return true;
        }
      }

      log.info(`Updating script ${remotePath} ...`);
      const body = {
        ...requestBodyCommon,
        parent_hash: remote.hash,
      };
      const execTime = await createScript(
        bundleContent,
        workspaceId,
        body,
        workspace
      );
      log.info(
        colors.yellow.bold(
          `Updated script ${remotePath} (${execTime.toFixed(0)}ms)`
        )
      );
    } else {
      log.info(`Creating new script ${remotePath} ...`);
      const body = {
        ...requestBodyCommon,
        parent_hash: undefined,
      };
      const execTime = await createScript(
        bundleContent,
        workspaceId,
        body,
        workspace
      );
      log.info(
        colors.yellow.bold(
          `Created new script ${remotePath} (${execTime.toFixed(0)}ms)`
        )
      );
    }
    return true;
  }
  return false;
}

async function streamToBlob(stream: ReadableStream<Uint8Array>): Promise<Blob> {
  // Create a reader from the stream
  const reader = stream.getReader();
  const chunks = [];

  // Read the data from the stream
  while (true) {
    const { done, value } = await reader.read();

    if (done) {
      // If stream is finished, break the loop
      break;
    }

    // Push the chunk to the array
    chunks.push(value);
  }

  // Create a Blob from the chunks
  const blob = new Blob(chunks);
  return blob;
}

async function createScript(
  bundleContent: string | Tarball | undefined,
  workspaceId: string,
  body: NewScript,
  workspace: Workspace
): Promise<number> {
  const start = performance.now();
  if (!bundleContent) {
    try {
      // no parent hash
      await wmill.createScript({
        workspace: workspaceId,
        requestBody: body,
      });
    } catch (e: any) {
      throw Error(
        `Script creation for ${body.path} with parent ${
          body.parent_hash
        }  was not successful: ${e.body ?? e.message} `
      );
    }
  } else {
    const form = new FormData();
    form.append("script", JSON.stringify(body));
    form.append(
      "file",
      typeof bundleContent == "string"
        ? bundleContent
        : await streamToBlob(bundleContent.stream())
    );

    const url =
      workspace.remote +
      "api/w/" +
      workspace.workspaceId +
      "/scripts/create_snapshot";
    const req = await fetch(url, {
      method: "POST",
      headers: { Authorization: `Bearer ${workspace.token} ` },
      body: form,
    });
    if (req.status != 201) {
      throw Error(
        `Script snapshot creation was not successful: ${req.status} - ${
          req.statusText
        } - ${await req.text()} `
      );
    }
  }
  return performance.now() - start;
}

export async function findContentFile(filePath: string) {
  const candidates = filePath.endsWith("script.json")
    ? exts.map((x) => filePath.replace(".script.json", x))
    : filePath.endsWith("script.lock")
    ? exts.map((x) => filePath.replace(".script.lock", x))
    : exts.map((x) => filePath.replace(".script.yaml", x));

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

export function filePathExtensionFromContentType(
  language: ScriptLanguage,
  defaultTs: "bun" | "deno" | undefined
): string {
  if (language === "python3") {
    return ".py";
  } else if (language === "nativets") {
    return ".fetch.ts";
  } else if (language === "bun") {
    if (defaultTs == "deno") {
      return ".bun.ts";
    } else {
      return ".ts";
    }
  } else if (language === "deno") {
    if (defaultTs == undefined || defaultTs == "bun") {
      return ".deno.ts";
    } else {
      return ".ts";
    }
  } else if (language === "go") {
    return ".go";
  } else if (language === "mysql") {
    return ".my.sql";
  } else if (language === "bigquery") {
    return ".bq.sql";
  } else if (language === "duckdb") {
    return ".duckdb.sql";
  } else if (language === "oracledb") {
    return ".odb.sql";
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
  } else if (language === "php") {
    return ".php";
  } else if (language === "rust") {
    return ".rs";
  } else if (language === "ansible") {
    return ".playbook.yml";
  } else if (language === "csharp") {
    return ".cs";
  } else if (language === "nu") {
    return ".nu";
  } else if (language === "java") {
    return ".java";
    // for related places search: ADD_NEW_LANG
  } else {
    throw new Error("Invalid language: " + language);
  }
}

export const exts = [
  ".fetch.ts",
  ".deno.ts",
  ".bun.ts",
  ".ts",
  ".py",
  ".go",
  ".sh",
  ".pg.sql",
  ".my.sql",
  ".bq.sql",
  ".odb.sql",
  ".sf.sql",
  ".ms.sql",
  ".duckdb.sql",
  ".sql",
  ".gql",
  ".ps1",
  ".php",
  ".rs",
  ".cs",
  ".nu",
  ".playbook.yml",
  ".java",
  // for related places search: ADD_NEW_LANG
];

export function removeExtensionToPath(path: string): string {
  for (const ext of exts) {
    if (path.endsWith(ext)) {
      return path.substring(0, path.length - ext.length);
    }
  }
  throw new Error("Invalid extension: " + path);
}

async function list(
  opts: GlobalOptions & {
    showArchived?: boolean;
    includeWithoutMain?: boolean;
    includeDraftOnly?: boolean;
  }
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: Script[] = [];
  while (true) {
    const res = await wmill.listScripts({
      workspace: workspace.workspaceId,
      page,
      perPage,
      showArchived: opts.showArchived ?? false,
      includeWithoutMain: opts.includeWithoutMain ?? false,
      includeDraftOnly: opts.includeDraftOnly ?? true,
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
  const id = await wmill.runScriptByPath({
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
          await wmill.getCompletedJob({
            workspace: workspace.workspaceId,
            id,
          })
        ).result ?? {};

      if (opts.silent) {
        console.log(result);
      } else {
        log.info(result);
      }

      break;
    } catch {
      new Promise((resolve, _) => setTimeout(() => resolve(undefined), 100));
    }
  }
}

export async function track_job(workspace: string, id: string) {
  try {
    const result = await wmill.getCompletedJob({ workspace, id });

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
      updates = await wmill.getJobUpdates({
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
    const final_job = await wmill.getCompletedJob({ workspace, id });
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
  const s = await wmill.getScriptByPath({
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

  const config = await readConfigFile();

  const extension = filePathExtensionFromContentType(
    language,
    config.defaultTs
  );
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
    scriptMetadata as Record<string, any>,
    yamlOptions
  );

  await Deno.writeTextFile(scriptCodeFileFullPath, scriptInitialCode, {
    createNew: true,
  });
  await Deno.writeTextFile(
    scriptMetadataFileFullPath,
    scriptInitialMetadataYaml,
    {
      createNew: true,
    }
  );
}

export type GlobalDeps = {
  pkgs: Record<string, string>;
  reqs: Record<string, string>;
  composers: Record<string, string>;
  goMods: Record<string, string>;
};
export async function findGlobalDeps(): Promise<GlobalDeps> {
  const pkgs: { [key: string]: string } = {};
  const reqs: { [key: string]: string } = {};
  const composers: { [key: string]: string } = {};
  const goMods: { [key: string]: string } = {};
  const els = await FSFSElement(Deno.cwd(), [], false);
  for await (const entry of readDirRecursiveWithIgnore((p, isDir) => {
    p = SEP + p;
    return (
      !isDir &&
      !(
        p.endsWith(SEP + "package.json") ||
        p.endsWith(SEP + "requirements.txt") ||
        p.endsWith(SEP + "composer.json") ||
        p.endsWith(SEP + "go.mod")
      )
    );
  }, els)) {
    if (entry.isDirectory || entry.ignored) continue;
    const content = await entry.getContentText();
    if (entry.path.endsWith("package.json")) {
      pkgs[entry.path.substring(0, entry.path.length - "package.json".length)] = content;
    } else if (entry.path.endsWith("requirements.txt")) {
      reqs[entry.path.substring(0, entry.path.length - "requirements.txt".length)] = content;
    } else if (entry.path.endsWith("composer.json")) {
      composers[entry.path.substring(0, entry.path.length - "composer.json".length)] = content;
    } else if (entry.path.endsWith("go.mod")) {
      goMods[entry.path.substring(0, entry.path.length - "go.mod".length)] = content;
    }
  }
  return { pkgs, reqs, composers, goMods };
}
async function generateMetadata(
  opts: GlobalOptions & {
    lockOnly?: boolean;
    schemaOnly?: boolean;
    yes?: boolean;
  } & SyncOptions,
  scriptPath: string | undefined
) {
  log.info(
    "This command only works for workspace scripts, for flows inline scripts use `wmill flow generate - locks`"
  );
  if (scriptPath == "") {
    scriptPath = undefined;
  }
  if (scriptPath && !validatePath(scriptPath)) {
    return;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);
  const codebases = await listSyncCodebases(opts);

  const globalDeps = await findGlobalDeps();
  if (scriptPath) {
    // read script metadata file
    await generateScriptMetadataInternal(
      scriptPath,
      workspace,
      opts,
      false,
      false,
      globalDeps,
      codebases,
      false
    );
  } else {
    const ignore = await ignoreF(opts);
    const elems = await elementsToMap(
      await FSFSElement(Deno.cwd(), codebases, false),
      (p, isD) => {
        return (
          (!isD && !exts.some((ext) => p.endsWith(ext))) ||
          ignore(p, isD) ||
          p.includes(".flow" + SEP) ||
          p.includes(".app" + SEP)
        );
      },
      false,
      {}
    );
    let hasAny = false;
    log.info("Generating metadata for all stale scripts:");
    for (const e of Object.keys(elems)) {
      const candidate = await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        true,
        true,
        globalDeps,
        codebases,
        false
      );
      if (candidate) {
        hasAny = true;
        log.info(colors.green(`+ ${candidate} `));
      }
    }
    if (hasAny) {
      if (opts.dryRun) {
        log.info(colors.gray(`Dry run complete.`));
        return;
      }
      if (
        !opts.yes &&
        !(await Confirm.prompt({
          message: "Update the metadata of the above scripts?",
          default: true,
        }))
      ) {
        return;
      }
    } else {
      log.info(colors.green.bold("No metadata to update"));
      return;
    }
    for (const e of Object.keys(elems)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        false,
        true,
        globalDeps,
        codebases,
        false
      );
    }
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
  .arguments("<path:file> <language:string>")
  .option("--summary <summary:string>", "script summary")
  .option("--description <description:string>", "script description")
  .action(bootstrap as any)
  .command(
    "generate-metadata",
    "re-generate the metadata file updating the lock and the script schema (for flows, use `wmill flow generate-locks`)"
  )
  .arguments("[script:file]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Perform a dry run without making changes")
  .option("--lock-only", "re-generate only the lock")
  .option("--schema-only", "re-generate only script schema")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account."
  )
  .action(generateMetadata as any);

export default command;
