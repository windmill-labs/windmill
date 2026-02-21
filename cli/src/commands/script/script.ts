import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { readFile, writeFile, stat } from "node:fs/promises";
import { Buffer } from "node:buffer";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Table } from "@cliffy/table";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import { stringify as yamlStringify } from "@std/yaml";
import { deepEqual } from "../../utils/utils.ts";
import * as wmill from "../../../gen/services.gen.ts";
import * as specificItems from "../../core/specific_items.ts";
import { getCurrentGitBranch } from "../../utils/git.ts";

import {
  defaultScriptMetadata,
  scriptBootstrapCode,
} from "../../../bootstrap/script_bootstrap.ts";

import { Workspace } from "../workspace/workspace.ts";
import {
  generateScriptMetadataInternal,
  getRawWorkspaceDependencies,
  parseMetadataFile,
} from "../../utils/metadata.ts";
import {
  WorkspaceDependenciesLanguage,
  ScriptLanguage,
  inferContentTypeFromFilePath,
  workspaceDependenciesLanguages,
} from "../../utils/script_common.ts";
import {
  elementsToMap,
  findCodebase,
  readDirRecursiveWithIgnore,
  Skips,
  yamlOptions,
} from "../sync/sync.ts";
import { ignoreF } from "../sync/sync.ts";
import { FSFSElement } from "../sync/sync.ts";
import {
  SyncOptions,
  mergeConfigWithConfigFile,
  readConfigFile,
} from "../../core/conf.ts";
import { SyncCodebase, listSyncCodebases } from "../../utils/codebase.ts";
import fs from "node:fs";
import { type Tarball } from "@ayonli/jsext/archive";

import { execSync } from "node:child_process";
import { NewScript, Script } from "../../../gen/types.gen.ts";
import {
  isRawAppBackendPath as isRawAppBackendPathInternal,
  isAppInlineScriptPath as isAppInlineScriptPathInternal,
  isFlowInlineScriptPath as isFlowInlineScriptPathInternal,
  isFlowPath,
  isAppPath,
} from "../../utils/resource_folders.ts";

export interface ScriptFile {
  parent_hash?: string;
  summary: string;
  description: string;
  schema?: any;
  is_template?: boolean;
  lock?: Array<string>;
  kind?: "script" | "failure" | "trigger" | "command" | "approval";
}

/**
 * Checks if a path is inside a raw app backend folder.
 * Matches patterns like: .../myApp.raw_app/backend/...
 */
export function isRawAppBackendPath(filePath: string): boolean {
  return isRawAppBackendPathInternal(filePath);
}

/**
 * Checks if a path is inside a normal app folder (inline script).
 * Matches patterns like: .../myApp.app/... or .../myApp__app/...
 */
export function isAppInlineScriptPath(filePath: string): boolean {
  return isAppInlineScriptPathInternal(filePath);
}

/**
 * Checks if a path is inside a flow folder (inline script).
 * Matches patterns like: .../myFlow.flow/... or .../myFlow__flow/...
 */
export function isFlowInlineScriptPath(filePath: string): boolean {
  return isFlowInlineScriptPathInternal(filePath);
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);

  if (!validatePath(filePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    throw Error(
      "Cannot push a script metadata file, point to the script content file instead (.py, .ts, .go|.sh)"
    );
  }

  await requireLogin(opts);
  const codebases = await listSyncCodebases(opts as SyncOptions);

  await handleFile(
    filePath,
    workspace,
    [],
    undefined,
    opts,
    await getRawWorkspaceDependencies(),
    codebases
  );
  log.info(colors.bold.underline.green(`Script ${filePath} pushed`));
}

export async function findResourceFile(path: string) {
  const splitPath = path.split(".");

  let contentBasePathJSON = splitPath[0] + "." + splitPath[1] + ".json";
  let contentBasePathYAML = splitPath[0] + "." + splitPath[1] + ".yaml";

  // Check for branch-specific metadata files first
  const currentBranch = getCurrentGitBranch();

  const candidates = [contentBasePathJSON, contentBasePathYAML];

  if (currentBranch) {
    // Add branch-specific candidates at the beginning (higher priority)
    const branchSpecificJSON = specificItems.toBranchSpecificPath(
      contentBasePathJSON,
      currentBranch
    );
    const branchSpecificYAML = specificItems.toBranchSpecificPath(
      contentBasePathYAML,
      currentBranch
    );
    candidates.unshift(branchSpecificJSON, branchSpecificYAML);
  }

  const validCandidates = (
    await Promise.all(
      candidates.map((x) => {
        return stat(x)
          .catch(() => undefined)
          .then((x) => x?.isFile())
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
  rawWorkspaceDependencies: Record<string, string>,
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
      rawWorkspaceDependencies,
      codebases
    );
  } else {
    return false;
  }
}

export interface OutputFile {
  path: string;
  contents: Uint8Array;
  hash: string;
  /** "contents" as text (changes automatically with "contents") */
  readonly text: string;
}

export async function handleFile(
  path: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  opts: (GlobalOptions & { defaultTs?: "bun" | "deno" } & Skips) | undefined,
  rawWorkspaceDependencies: Record<string, string>,
  codebases: SyncCodebase[]
): Promise<boolean> {
  if (
    !isAppInlineScriptPath(path) &&
    !isFlowInlineScriptPath(path) &&
    !isRawAppBackendPath(path) &&
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

    let forceTar = false;
    if (codebase) {
      let outputFiles: OutputFile[] = [];
      if (codebase.customBundler) {
        log.info(`Using custom bundler ${codebase.customBundler} for ${path}`);
        bundleContent = execSync(codebase.customBundler + " " + path, {
          maxBuffer: 1024 * 1024 * 50,
        }).toString();
        log.info("Custom bundler executed for " + path);
      } else {
        const esbuild = await import("esbuild");

        log.info(`Started bundling ${path} ...`);
        const startTime = performance.now();
        const format = codebase.format ?? "cjs";
        const out = await esbuild.build({
          entryPoints: [path],
          format: format,
          bundle: true,
          write: false,
          external: codebase.external,
          inject: codebase.inject,
          define: codebase.define,
          loader: codebase.loader ?? { ".node": "file" },
          outdir: "/",
          platform: "node",
          packages: "bundle",
          target: format == "cjs" ? "node20.15.1" : "esnext",
          banner: codebase.banner,
          // ...(codebase.banner != null && { banner: codebase.banner }),
        });
        const endTime = performance.now();
        bundleContent = out.outputFiles[0].text;
        outputFiles = out.outputFiles ?? [];
        if (outputFiles.length == 0) {
          throw new Error(`No output files found for ${path}`);
        }
        log.info(
          `Finished bundling ${path}: ${(bundleContent.length / 1024).toFixed(
            0
          )}kB (${(endTime - startTime).toFixed(0)}ms)`
        );
      }
      if (outputFiles.length > 1) {
        const archiveNpm = await import("@ayonli/jsext/archive");
        log.info(
          `Found multiple output files for ${path}, creating a tarball... ${outputFiles
            .map((file) => file.path)
            .join(", ")}`
        );
        forceTar = true;
        const startTime = performance.now();
        const tarball = new archiveNpm.Tarball();
        const mainPath = path.split(SEP).pop()?.split(".")[0] + ".js";
        const content =
          outputFiles.find((file) => file.path == "/" + mainPath)?.text ?? "";
        log.info(`Main content: ${content.length}chars`);
        tarball.append(new File([content], "main.js", { type: "text/plain" }));
        for (const file of outputFiles) {
          if (file.path == "/" + mainPath) {
            continue;
          }
          log.info(`Adding file: ${file.path.substring(1)}`);
        
          const fil = new File([file.contents as any], file.path.substring(1));
          tarball.append(fil);
        }
        const endTime = performance.now();
        log.info(
          `Finished creating tarball for ${path}: ${(
            tarball.size / 1024
          ).toFixed(0)}kB (${(endTime - startTime).toFixed(0)}ms)`
        );
        bundleContent = tarball;
      } else {
        if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
          const archiveNpm = await import("@ayonli/jsext/archive");
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
                  rawWorkspaceDependencies,
                  codebases,
                }
              : undefined
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
    const content = await readFile(path, "utf-8");

    if (opts?.skipScriptsMetadata) {
      // if (codebase) {
      //   const typedBefore = JSON.parse(JSON.stringify(typed.schema));
      //   await updateScriptSchema(content, language, typed, path);
      //   if (typedBefore != typed.schema) {
      //     log.info(`Updated metadata for bundle ${path}`);
      typed = structuredClone(remote);
      // }
    }

    if (typed && codebase) {
      typed.codebase = await codebase.getDigest(forceTar);
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
      debounce_key: typed?.debounce_key,
      debounce_delay_s: typed?.debounce_delay_s,
      codebase: await codebase?.getDigest(forceTar),
      timeout: typed?.timeout,
      on_behalf_of_email: typed?.on_behalf_of_email,
      envs: typed?.envs,
    };

    // console.log(requestBodyCommon.codebase);
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
            typed.debounce_key == remote["debounce_key"] &&
            typed.debounce_delay_s == remote["debounce_delay_s"] &&
            typed.codebase == remote.codebase &&
            typed.on_behalf_of_email == remote.on_behalf_of_email &&
            deepEqual(typed.envs, remote.envs))
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


  const blob = new Blob(chunks as any);
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
        return stat(x)
          .catch(() => undefined)
          .then((x) => x?.isFile())
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
  } else if (language === "ruby") {
    return ".rb";
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
  ".rb",
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
    const chunks: Buffer[] = [];
    for await (const chunk of process.stdin) chunks.push(chunk);
    input = new TextDecoder().decode(Buffer.concat(chunks));
  }
  if (input[0] == "@") {
    input = await readFile(input.substring(1), "utf-8");
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
        log.info(JSON.stringify(result, null, 2));
      }

      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 100));
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
      await new Promise((resolve) => setTimeout(resolve, 500));
      continue;
    }

    if (!running && updates.running === true) {
      running = true;
      log.info(colors.green("Job running. Streaming logs..."));
    }

    if (updates.new_logs) {
      process.stdout.write(updates.new_logs);
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
    await stat(scriptCodeFileFullPath);
    await stat(scriptMetadataFileFullPath);
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

  await writeFile(scriptCodeFileFullPath, scriptInitialCode, {
    flag: 'wx', encoding: 'utf-8',
  });
  await writeFile(
    scriptMetadataFileFullPath,
    scriptInitialMetadataYaml,
    {
      flag: 'wx', encoding: 'utf-8',
    }
  );
}

export type GlobalDeps = Map<
  WorkspaceDependenciesLanguage,
  Record<string, string>
>;

async function generateMetadata(
  opts: GlobalOptions & {
    lockOnly?: boolean;
    schemaOnly?: boolean;
    yes?: boolean;
  } & SyncOptions,
  scriptPath: string | undefined
) {
  log.info(
    "This command only works for workspace scripts, for flows inline scripts use `wmill flow generate-locks`"
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

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies();
  if (scriptPath) {
    // read script metadata file
    await generateScriptMetadataInternal(
      scriptPath,
      workspace,
      opts,
      false,
      false,
      rawWorkspaceDependencies,
      codebases,
      false
    );
  } else {
    // TODO: test this as well.
    const ignore = await ignoreF(opts);
    const elems = await elementsToMap(
      await FSFSElement(process.cwd(), codebases, false),
      (p, isD) => {
        return (
          (!isD && !exts.some((ext) => p.endsWith(ext))) ||
          ignore(p, isD) ||
          isFlowPath(p) ||
          isAppPath(p)
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
        rawWorkspaceDependencies,
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
    // TODO: test this
    for (const e of Object.keys(elems)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        false,
        true,
        rawWorkspaceDependencies,
        codebases,
        false
      );
    }
  }
}

async function preview(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  } & SyncOptions,
  filePath: string
) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(filePath)) {
    return;
  }

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    throw Error(
      "Cannot preview a script metadata file, point to the script content file instead (.py, .ts, .go, .sh)"
    );
  }

  const codebases = await listSyncCodebases(opts);
  const language = inferContentTypeFromFilePath(filePath, opts?.defaultTs);
  const content = await readFile(filePath, "utf-8");
  const input = opts.data ? await resolve(opts.data) : {};

  // Check if this is a codebase script
  const codebase =
    language == "bun" ? findCodebase(filePath, codebases) : undefined;

  let bundledContent: string | Blob | undefined = undefined;
  let isTar = false;

  if (codebase) {
    if (codebase.customBundler) {
      if (!opts.silent) {
        log.info(`Using custom bundler ${codebase.customBundler} for preview`);
      }
      bundledContent = execSync(codebase.customBundler + " " + filePath, {
        maxBuffer: 1024 * 1024 * 50,
      }).toString();
    } else {
      const esbuild = await import("esbuild");

      if (!opts.silent) {
        log.info(`Bundling ${filePath} for preview...`);
      }
      const startTime = performance.now();
      const format = codebase.format ?? "cjs";
      const out = await esbuild.build({
        entryPoints: [filePath],
        format: format,
        bundle: true,
        write: false,
        external: codebase.external,
        inject: codebase.inject,
        define: codebase.define,
        loader: codebase.loader ?? { ".node": "file" },
        outdir: "/",
        platform: "node",
        packages: "bundle",
        target: format == "cjs" ? "node20.15.1" : "esnext",
        banner: codebase.banner,
      });
      const endTime = performance.now();
      bundledContent = out.outputFiles[0].text;

      // Handle multiple output files (create tarball)
      if (out.outputFiles.length > 1) {
        const archiveNpm = await import("@ayonli/jsext/archive");
        if (!opts.silent) {
          log.info(`Creating tarball for multiple output files...`);
        }
        const tarball = new archiveNpm.Tarball();
        const mainPath = filePath.split(SEP).pop()?.split(".")[0] + ".js";
        const mainContent =
          out.outputFiles.find((file: OutputFile) => file.path == "/" + mainPath)?.text ?? "";
        tarball.append(new File([mainContent], "main.js", { type: "text/plain" }));
        for (const file of out.outputFiles) {
          if (file.path == "/" + mainPath) continue;
        
          const fil = new File([file.contents as any], file.path.substring(1));
          tarball.append(fil);
        }
        bundledContent = await streamToBlob(tarball.stream());
        isTar = true;
      } else if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
        // Handle assets
        const archiveNpm = await import("@ayonli/jsext/archive");
        if (!opts.silent) {
          log.info(`Adding assets to tarball...`);
        }
        const tarball = new archiveNpm.Tarball();
        tarball.append(new File([bundledContent], "main.js", { type: "text/plain" }));
        for (const asset of codebase.assets) {
          const data = fs.readFileSync(asset.from);
          const blob = new Blob([data], { type: "text/plain" });
          const file = new File([blob], asset.to);
          tarball.append(file);
        }
        bundledContent = await streamToBlob(tarball.stream());
        isTar = true;
      }

      if (!opts.silent) {
        const size = typeof bundledContent === "string" ? bundledContent.length : bundledContent.size;
        log.info(
          `Bundled ${filePath}: ${(size / 1024).toFixed(0)}kB (${(
            endTime - startTime
          ).toFixed(0)}ms)`
        );
      }
    }
  }

  if (!opts.silent) {
    log.info(colors.yellow(`Running preview for ${filePath}...`));
  }

  // For codebase scripts with bundles, we need to use a multipart form upload
  if (bundledContent) {
    const form = new FormData();
    const previewPayload = {
      content: content, // Pass the original content (frontend does this too)
      path: filePath.substring(0, filePath.indexOf(".")).replaceAll(SEP, "/"),
      args: input,
      language: language,
      kind: isTar ? "tarbundle" : "bundle",
      format: codebase?.format ?? "cjs",
    };
    form.append("preview", JSON.stringify(previewPayload));
    form.append(
      "file",
      typeof bundledContent === "string"
        ? new Blob([bundledContent], { type: "application/javascript" })
        : bundledContent
    );

    const url =
      workspace.remote +
      "api/w/" +
      workspace.workspaceId +
      "/jobs/run/preview_bundle";

    const response = await fetch(url, {
      method: "POST",
      headers: { Authorization: `Bearer ${workspace.token}` },
      body: form,
    });

    if (!response.ok) {
      throw new Error(
        `Preview failed: ${response.status} - ${response.statusText} - ${await response.text()}`
      );
    }

    const jobId = await response.text();
    if (!opts.silent) {
      await track_job(workspace.workspaceId, jobId);
    }

    // Wait for the job to complete and get the result
    while (true) {
      try {
        const completedJob = await wmill.getCompletedJob({
          workspace: workspace.workspaceId,
          id: jobId,
        });

        const result = completedJob.result ?? {};
        if (opts.silent) {
          console.log(JSON.stringify(result, null, 2));
        } else {
          log.info(JSON.stringify(result, null, 2));
        }
        break;
      } catch {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    }
  } else {
    // For regular scripts, use the standard preview API
    const result = await wmill.runScriptPreviewAndWaitResult({
      workspace: workspace.workspaceId,
      requestBody: {
        content,
        path: filePath.substring(0, filePath.indexOf(".")).replaceAll(SEP, "/"),
        args: input,
        language: language as any,
      },
    });

    if (opts.silent) {
      console.log(JSON.stringify(result, null, 2));
    } else {
      log.info(colors.bold.underline.green("Preview completed"));
      log.info(JSON.stringify(result, null, 2));
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
  .command(
    "preview",
    "preview a local script without deploying it. Supports both regular and codebase scripts."
  )
  .arguments("<path:file>")
  .option(
    "-d --data <data:file>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not output anything other than the final output. Useful for scripting."
  )
  .action(preview as any)
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
