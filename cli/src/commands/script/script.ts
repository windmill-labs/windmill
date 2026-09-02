import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import {
  assertRemotePath,
  resolveWorkspace,
  toSyncRootRelativePath,
  validatePath,
} from "../../core/context.ts";
import type { PermissionedAsContext } from "../../core/permissioned_as.ts";
import { applyExtraPermsDiff } from "../../core/extra_perms.ts";
import { writeFile, stat, mkdir } from "node:fs/promises";
import { Buffer } from "node:buffer";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import * as path from "node:path";
import { stringify as yamlStringify } from "yaml";
import { deepEqual, getHeaders, isFileResource, isFilesetResource, readTextFile, readTextFileSync } from "../../utils/utils.ts";
import { detectAuthGatewayChallenge } from "../../utils/http_guards.ts";
import * as wmill from "../../../gen/services.gen.ts";
import * as specificItems from "../../core/specific_items.ts";
import { getCurrentGitBranch } from "../../utils/git.ts";

import {
  defaultScriptMetadata,
  scriptBootstrapCode,
} from "../../../bootstrap/script_bootstrap.ts";

import { Workspace } from "../workspace/workspace.ts";
import {
  checkifMetadataUptodate,
  generateScriptHash,
  generateScriptMetadataInternal,
  getRawWorkspaceDependencies,
  parseMetadataFile,
  readLockfile,
} from "../../utils/metadata.ts";
import { validateRequiredArgs } from "../../utils/utils.ts";
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
import { pollJobWithQueueLogging } from "../../utils/job_polling.ts";
import fs from "node:fs";
import { createTarBlob, type TarEntry } from "../../utils/tar.ts";
import { getEsbuild } from "../../utils/esbuild_loader.ts";

import { execSync } from "node:child_process";
import { NewScript, Script, ScriptModule } from "../../../gen/types.gen.ts";
import {
  isRawAppBackendPath as isRawAppBackendPathInternal,
  isAppInlineScriptPath as isAppInlineScriptPathInternal,
  isFlowInlineScriptPath as isFlowInlineScriptPathInternal,
  isFlowPath,
  isAppPath,
  isScriptModulePath,
  buildModuleFolderPath,
  getModuleFolderSuffix,
  dbtGeneratedDirs,
  isUnderGeneratedDir,
  isLocalSecretFile,
  moduleFileExclusion,
  oversizedModuleFileError,
  MAX_MODULE_BYTES,
  isModuleEntryPoint,
  getScriptBasePathFromModulePath,
  scriptPathToRemotePath,
  isRawAppPath,
  DBT_DESCRIPTOR_NAME,
  isDbtDescriptorPath,
  isMissingDbtDescriptor,
} from "../../utils/resource_folders.ts";

export interface ScriptFile {
  parent_hash?: string;
  summary: string;
  description: string;
  schema?: any;
  is_template?: boolean;
  lock?: Array<string>;
  kind?: "script" | "failure" | "trigger" | "command" | "approval";
  // Mirrors granular ACLs on the script path. Omitted from .script.yaml when
  // no perms are set. The CLI applies diffs through /acls/add and /acls/remove
  // (see applyExtraPermsDiff) — never through create_script — so a perm-only
  // change never bumps the script hash/version.
  extra_perms?: Record<string, boolean>;
}

/**
 * Checks if a path is inside a raw app backend folder.
 * Matches patterns like: .../myApp.raw_app/backend/...
 */
export function isRawAppBackendPath(filePath: string): boolean {
  return isRawAppBackendPathInternal(filePath);
}

/**
 * The positive-only runnable settings (concurrent_limit, timeout, ...) treat any `<= 0`
 * value as "unset": the backend coerces it to null (a 0-slot concurrency limit bricks the
 * runnable, a 0s timeout kills every run). Coerce to undefined so it is serialized as
 * omitted, never as 0, and redeploys don't churn against the backend-normalized value.
 */
export function nonePositiveInt(
  v: number | undefined | null
): number | undefined {
  return v != null && v > 0 ? v : undefined;
}

/**
 * Normalize a concurrent_limit + its time window together: when the limit is disabled
 * (<= 0) the window is dropped too. Returns [concurrent_limit, concurrency_time_window_s].
 */
export function normalizeConcurrency(
  concurrentLimit: number | undefined | null,
  concurrencyTimeWindowS?: number | undefined | null
): [number | undefined, number | undefined] {
  const limit = nonePositiveInt(concurrentLimit);
  return limit === undefined ? [undefined, undefined] : [limit, concurrencyTimeWindowS ?? undefined];
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

type PushOptions = GlobalOptions & { message?: string };
export async function computePushMetadataHash(
  filePath: string,
  content: string
): Promise<string> {
  const remotePath = removeExtensionToPath(filePath).replaceAll(SEP, "/");
  const metadataWithType = await parseMetadataFile(remotePath, undefined);
  const metadataContent = await readTextFile(metadataWithType.path);
  return await generateScriptHash({}, content, metadataContent);
}

async function push(opts: PushOptions, filePath: string) {
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);

  if (!validatePath(filePath)) {
    return;
  }

  // A dbt project's descriptor is optional, so the one content path a
  // descriptor-less project has is deliberately not on disk. The project beside
  // it is what says the script is real.
  const absentDescriptor = await stat(filePath).then(
    () => false,
    (e) => isMissingDbtDescriptor(filePath, e)
  );
  if (!absentDescriptor) {
    const fstat = await stat(filePath);
    if (!fstat.isFile()) {
      throw new Error("file path must refer to a file.");
    }
  }

  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    throw Error(
      "Cannot push a script metadata file, point to the script content file instead (.py, .ts, .go|.sh)"
    );
  }

  if (isFileResource(filePath) || isFilesetResource(filePath)) {
    throw Error(
      "Cannot push a file/fileset resource content file as a script, push its .resource.yaml with 'wmill resource push' instead"
    );
  }

  await requireLogin(opts);

  // Warn about metadata state before pushing
  try {
    const content = await readScriptContent(filePath);
    const remotePath = removeExtensionToPath(filePath).replaceAll(SEP, "/");
    const contentHash = await computePushMetadataHash(filePath, content);
    const conf = await readLockfile();
    const hasLockEntry = conf.locks && (conf.locks[remotePath] !== undefined || conf.locks[`${remotePath}.ts`] !== undefined);
    if (!hasLockEntry) {
      log.warn(colors.yellow(
        `No metadata generated yet for ${filePath}. Run 'wmill generate-metadata' to generate schema and lock.`
      ));
    } else if (!(await checkifMetadataUptodate(remotePath, contentHash, conf))) {
      log.warn(colors.yellow(
        `Metadata for ${filePath} appears stale (content changed since last 'wmill generate-metadata').\n` +
        `The schema and lock may not match the current code. Consider running 'wmill generate-metadata' first.`
      ));
    }
  } catch {
    // Don't block push if check fails
  }

  const codebases = await listSyncCodebases(opts as SyncOptions);

  await handleFile(
    filePath,
    workspace,
    [],
    opts.message,
    opts,
    await getRawWorkspaceDependencies(true),
    codebases
  );
  log.info(colors.bold.underline.green(`Script ${filePath} pushed`));
}

export async function findResourceFile(path: string) {
  const splitPath = path.split(".");

  let contentBasePathJSON = splitPath[0] + "." + splitPath[1] + ".json";
  let contentBasePathYAML = splitPath[0] + "." + splitPath[1] + ".yaml";

  // Check for workspace-specific metadata files first, using the wmill.yaml
  // config key for the current git branch as the filename suffix (falls back
  // to the branch name when no matching workspace entry exists).
  const currentBranch = getCurrentGitBranch();
  const wsName = currentBranch
    ? await specificItems.resolveWsNameForGitBranch(currentBranch)
    : null;

  const candidates = [contentBasePathJSON, contentBasePathYAML];

  if (wsName) {
    // Add workspace-specific candidates at the beginning (higher priority)
    const branchSpecificJSON = specificItems.toWorkspaceSpecificPath(
      contentBasePathJSON,
      wsName
    );
    const branchSpecificYAML = specificItems.toWorkspaceSpecificPath(
      contentBasePathYAML,
      wsName
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

// The separator is whatever the local filesystem uses, so both are accepted:
// on Windows these paths reach us as `my_script__mod\script.yaml`.
const MODULE_ENTRY_META_RE = /([\\/])script\.(yaml|json|lock)$/;

/**
 * Whether a path is a module folder's own metadata file (`__mod/script.yaml`).
 * `isModuleEntryPoint` already pins the file to `script.*` directly under
 * `__mod/`, so this only narrows it to the metadata extensions: a `script.yaml`
 * nested deeper in the module tree is a module file, not the script's metadata.
 */
export function isModuleEntryMetadata(p: string): boolean {
  return isModuleEntryPoint(p) && MODULE_ENTRY_META_RE.test(p);
}

export async function handleScriptMetadata(
  path: string,
  workspace: Workspace,
  alreadySynced: string[],
  message: string | undefined,
  rawWorkspaceDependencies: Record<string, string>,
  codebases: SyncCodebase[],
  opts: GlobalOptions,
  permissionedAsContext?: PermissionedAsContext
): Promise<boolean> {
  // Flat layout: my_script.script.yaml
  const isFlatMeta = path.endsWith(".script.json") ||
    path.endsWith(".script.yaml") ||
    path.endsWith(".script.lock");
  const isFolderMeta = !isFlatMeta && isModuleEntryMetadata(path);
  if (isFlatMeta || isFolderMeta) {
    const contentPath = await findContentFile(path);
    return handleFile(
      contentPath,
      workspace,
      alreadySynced,
      message,
      opts,
      rawWorkspaceDependencies,
      codebases,
      permissionedAsContext
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
  codebases: SyncCodebase[],
  permissionedAsContext?: PermissionedAsContext
): Promise<boolean> {
  // A file/fileset resource's content file can carry a script extension
  // (.sql, .ts, …) but belongs to its parent resource, never to a
  // standalone script.
  if (isFileResource(path) || isFilesetResource(path)) {
    return false;
  }
  // Detect module entry point: e.g., my_script__mod/script.ts
  const moduleEntryPoint = isModuleEntryPoint(path);
  if (
    !isAppInlineScriptPath(path) &&
    !isFlowInlineScriptPath(path) &&
    // Raw-app files (frontend included) belong to the app bundle, never
    // standalone scripts — pushed via pushRawApp, not here.
    !isRawAppPath(path) &&
    (!isScriptModulePath(path) || moduleEntryPoint) &&
    hasScriptExt(path)
  ) {
    if (alreadySynced.includes(path)) {
      return true;
    }
    log.debug(`Processing local script ${path}`);

    alreadySynced.push(path);
    const remotePath = scriptPathToRemotePath(path);

    // Before anything is written: `<base>.py` and `<base>__dbt/` deploy to ONE
    // remote path, so whichever is pushed last replaces the other's script.
    // Refused from either side — the descriptor is exempt only from finding its
    // OWN project (it is that project's content file, so its base resolves to
    // the same `dbt_project.yml`), never from an ordinary sibling.
    // A folder-layout script is `<base>__mod/script.ts`, so stripping its
    // extension yields `<base>__mod/script`, not the base both layouts deploy
    // to. Wrong base, and the probe below looks in a directory that cannot
    // exist — which is how a `__mod` script and a dbt project at one path were
    // both pushed, each replacing the other.
    const base = isScriptModulePath(path)
      ? getScriptBasePathFromModulePath(path) ?? removeExtensionToPath(path)
      : removeExtensionToPath(path);
    const isDescriptor = isDbtDescriptorPath(path);
    const other = isDescriptor
      ? await collidingOrdinaryScript(base)
      : await collidingDbtProject(base);
    if (other) {
      throw isDescriptor
        ? dbtPathCollisionError(path, other)
        : dbtPathCollisionError(other, path);
    }

    const language = inferContentTypeFromFilePath(path, opts?.defaultTs);

    const codebase =
      language == "bun" ? findCodebase(path, codebases) : undefined;

    let bundleContent: string | Blob | undefined = undefined;

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
        const esbuild = await getEsbuild();

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
        log.info(
          `Found multiple output files for ${path}, creating a tarball... ${outputFiles
            .map((file) => file.path)
            .join(", ")}`
        );
        forceTar = true;
        const startTime = performance.now();
        const mainPath = path.split(SEP).pop()?.split(".")[0] + ".js";
        const mainContent =
          outputFiles.find((file) => file.path == "/" + mainPath)?.text ?? "";
        log.info(`Main content: ${mainContent.length}chars`);
        const entries: TarEntry[] = [
          { name: "main.js", content: mainContent },
        ];
        for (const file of outputFiles) {
          if (file.path == "/" + mainPath) {
            continue;
          }
          log.info(`Adding file: ${file.path.substring(1)}`);
          entries.push({ name: file.path.substring(1), content: file.contents });
        }
        bundleContent = await createTarBlob(entries);
        const endTime = performance.now();
        log.info(
          `Finished creating tarball for ${path}: ${(
            bundleContent.size / 1024
          ).toFixed(0)}kB (${(endTime - startTime).toFixed(0)}ms)`
        );
      } else {
        if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
          log.info(
            `Using the following asset configuration for ${path}: ${JSON.stringify(
              codebase.assets
            )}`
          );
          const startTime = performance.now();
          const entries: TarEntry[] = [
            { name: "main.js", content: bundleContent },
          ];
          for (const asset of codebase.assets) {
            const data = fs.readFileSync(asset.from);
            entries.push({ name: asset.to, content: data });
          }
          bundleContent = await createTarBlob(entries);
          const endTime = performance.now();
          log.info(
            `Finished creating tarball for ${path}: ${(
              bundleContent.size / 1024
            ).toFixed(0)}kB (${(endTime - startTime).toFixed(0)}ms)`
          );
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
    const content = await readScriptContent(path);

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

    // Scan for modules: folder layout (entry point inside __mod/) or flat layout
    const scriptBasePath = moduleEntryPoint
      ? getScriptBasePathFromModulePath(path)!
      : path.substring(0, path.indexOf("."));
    const isDbt = language === "dbt";
    const moduleFolderPath = scriptBasePath + getModuleFolderSuffix(language);
    const modules = await readModulesFromDisk(
      moduleFolderPath,
      opts?.defaultTs,
      moduleEntryPoint,
      isDbt,
    );

    // A concurrent_limit of <= 0 means "concurrency disabled", not "zero slots" (which
    // would brick the runnable at the queue's concurrency gate). Emit it as omitted rather
    // than 0 so a redeploy never re-persists a zero-slot limit, and drop the now-meaningless
    // time window alongside it. Mirrors the backend's ConcurrencySettings::normalized.
    const [normConcurrentLimit, normConcurrencyTimeWindowS] = normalizeConcurrency(
      typed?.concurrent_limit,
      typed?.concurrency_time_window_s
    );

    const requestBodyCommon: NewScript = {
      content,
      description: typed?.description ?? "",
      language: language as NewScript["language"],
      path: remotePath.replaceAll(SEP, "/"),
      summary: typed?.summary ?? "",
      kind: typed?.kind,
      // A dbt lock pins a resolved commit and engine versions that only a
      // dependency job can determine, and that job is also what publishes the
      // script's manifest graph. Sending one suppresses that job, so the push
      // would deploy a stale lock AND leave the graph unpublished.
      lock: language === "dbt" ? undefined : typed?.lock,
      schema: typed?.schema,
      tag: typed?.tag,
      ws_error_handler_muted: typed?.ws_error_handler_muted,
      dedicated_worker: typed?.dedicated_worker,
      cache_ttl: typed?.cache_ttl,
      cache_ignore_s3_path: typed?.cache_ignore_s3_path,
      concurrency_time_window_s: normConcurrencyTimeWindowS,
      concurrent_limit: normConcurrentLimit,
      deployment_message: message,
      restart_unless_cancelled: typed?.restart_unless_cancelled,
      visible_to_runner_only: typed?.visible_to_runner_only,
      has_preprocessor: typed?.has_preprocessor,
      priority: typed?.priority,
      concurrency_key: typed?.concurrency_key,
      debounce_key: typed?.debounce_key,
      debounce_delay_s: typed?.debounce_delay_s,
      debounce_args_to_accumulate: typed?.debounce_args_to_accumulate,
      max_total_debouncing_time: typed?.max_total_debouncing_time,
      max_total_debounces_amount: typed?.max_total_debounces_amount,
      codebase: await codebase?.getDigest(forceTar),
      timeout: nonePositiveInt(typed?.timeout),
      // 0 means "delete immediately after completion", so it must survive as 0
      // rather than being folded into "unset" the way the positive-only settings are.
      delete_after_secs: typed?.delete_after_secs,
      on_behalf_of_email: typed?.on_behalf_of_email,
      envs: typed?.envs,
      modules: modules,
      labels: typed?.labels,
    };

    const hasOnBehalfOf = (typed as any)?.has_on_behalf_of ?? !!typed?.on_behalf_of_email;
    delete (typed as any)?.has_on_behalf_of;
    // The authorization half of the identity is never exported to the repo (the
    // workspace tarball strips it); it only ever travels back from the remote row.
    delete (typed as any)?.on_behalf_of;

    if (permissionedAsContext?.userIsAdminOrDeployer && hasOnBehalfOf) {
      if (remote && remote.on_behalf_of_email) {
        requestBodyCommon.on_behalf_of_email = remote.on_behalf_of_email;
        (requestBodyCommon as any).on_behalf_of = (
          remote as any
        ).on_behalf_of;
        (requestBodyCommon as any).preserve_on_behalf_of = true;
        log.info(`Preserving ${remote.on_behalf_of_email} as on_behalf_of for script ${remotePath}`);
      }
      // On create: backend applies folder defaults — no client-side resolution needed
    }

    if (remote) {
      if (content === remote.content) {
        if (
          typed == undefined ||
          (typed.description === remote.description &&
            typed.summary === remote.summary &&
            typed.kind == remote.kind &&
            // A `.ts` file changes language when defaultTs flips, content untouched.
            // bun and bunnative share that extension, so the inferred language is always
            // bun; the server derives bunnative back from the `//native` annotation in
            // the content, which is compared above.
            language ==
              (remote.language === "bunnative" ? "bun" : remote.language) &&
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
            Boolean(typed.cache_ignore_s3_path) ==
              Boolean(remote.cache_ignore_s3_path) &&
            normConcurrencyTimeWindowS ==
              normalizeConcurrency(
                remote.concurrent_limit,
                remote.concurrency_time_window_s
              )[1] &&
            normConcurrentLimit ==
              normalizeConcurrency(remote.concurrent_limit)[0] &&
            Boolean(typed.restart_unless_cancelled) ==
              Boolean(remote.restart_unless_cancelled) &&
            Boolean(typed.visible_to_runner_only) ==
              Boolean(remote.visible_to_runner_only) &&
            Boolean(typed.has_preprocessor) ==
              Boolean(remote.has_preprocessor) &&
            typed.priority == remote.priority &&
            nonePositiveInt(typed.timeout) == nonePositiveInt(remote.timeout) &&
            typed.delete_after_secs == remote.delete_after_secs &&
            //@ts-ignore
            typed.concurrency_key == remote["concurrency_key"] &&
            typed.debounce_key == remote["debounce_key"] &&
            typed.debounce_delay_s == remote["debounce_delay_s"] &&
            deepEqual(
              typed.debounce_args_to_accumulate ?? null,
              remote.debounce_args_to_accumulate ?? null
            ) &&
            typed.max_total_debouncing_time == remote.max_total_debouncing_time &&
            typed.max_total_debounces_amount == remote.max_total_debounces_amount &&
            typed.codebase == remote.codebase &&
            (hasOnBehalfOf ? true : typed.on_behalf_of_email == remote.on_behalf_of_email) &&
            deepEqual(typed.envs, remote.envs) &&
            deepEqual(typed.labels ?? null, remote.labels ?? null) &&
            deepEqual(modules ?? null, remote.modules ?? null))
        ) {
          log.info(colors.green(`Script ${remotePath} is up to date`));
          // Even when the body is unchanged, perms may still drift — sync them
          // independently before returning.
          await applyExtraPermsDiff(
            workspaceId,
            "script",
            remotePath.replaceAll(SEP, "/"),
            (typed as any)?.extra_perms,
            (remote as any)?.extra_perms,
          );
          return true;
        }
      }

      log.info(`Updating script ${remotePath} ...`);
      const body = {
        ...requestBodyCommon,
        parent_hash: remote.hash,
        auto_parent: true,
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

    // Sync granular ACLs as an independent step — perm-only edits never reach
    // create_script (which would bump the script hash) and instead route
    // through /acls/* via applyExtraPermsDiff.
    //
    // No refetch is needed:
    //  - folder perms are additive at auth time, never merged onto item rows;
    //  - the body sent to create_script doesn't carry extra_perms, so a fresh
    //    deploy of an existing path inherits the previous version's perms
    //    unchanged. The diff against `remote` (captured before the deploy)
    //    therefore matches what `wmill acl remove` would do — and the granular
    //    ACL endpoint updates every matching row, so the inheritance on the
    //    new version doesn't leave ghost entries.
    await applyExtraPermsDiff(
      workspaceId,
      "script",
      remotePath.replaceAll(SEP, "/"),
      (typed as any)?.extra_perms,
      (remote as any)?.extra_perms,
    );

    return true;
  }
  return false;
}

/**
 * Read module files from a __mod/ directory on disk.
 * Returns the modules record for the API, or undefined if no module folder exists.
 */
export async function readModulesFromDisk(
  moduleFolderPath: string,
  defaultTs: "bun" | "deno" | undefined,
  folderLayout: boolean = false,
  // A dbt project rides in its module folder as-is: `.sql` models (which the
  // language inference below rejects as an ambiguous dialect), `.yml` schemas
  // and `.csv` seeds are all part of the project and none is a Windmill script.
  // Verbatim, or dbt receives a project missing exactly the files it needs.
  verbatim: boolean = false,
): Promise<Record<string, ScriptModule> | undefined> {
  if (!fs.existsSync(moduleFolderPath) || !fs.statSync(moduleFolderPath).isDirectory()) {
    return undefined;
  }

  const modules: Record<string, ScriptModule> = {};

  const skipDirs = verbatim
    ? dbtGeneratedDirs(moduleFolderPath)
    : new Set<string>();

  // In folder layout mode, skip the entry point files (script.*, script.yaml, etc.)
  const isEntryPointFile = (name: string, isTopLevel: boolean) => {
    if (!isTopLevel) return false;
    // A dbt project's descriptor is the script's CONTENT, so it must not also
    // ride along as a module: the push would send the same text twice and dbt
    // would find a stray file at its project root.
    if (verbatim) return name === DBT_DESCRIPTOR_NAME;
    if (!folderLayout) return false;
    return (
      name.startsWith("script.") ||
      name === "script.lock" ||
      name === "script.yaml" ||
      name === "script.json"
    );
  };

  function readDir(dirPath: string, relPrefix: string) {
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dirPath, entry.name);
      const relPath = relPrefix ? relPrefix + "/" + entry.name : entry.name;
      const isTopLevel = relPrefix === "";

      if (entry.isDirectory()) {
        // A configured `target-path` may be nested (`build/target`), so the
        // comparison is on the project-relative path, not the entry name.
        if (skipDirs.size > 0 && isUnderGeneratedDir(relPath, skipDirs)) continue;
        readDir(fullPath, relPath);
        // `.lock` is the script's own lockfile in a `__mod` bundle (the `lock`
        // field on ScriptModule) — but a dbt project's files are its author's,
        // and one may legitimately be named `uv.lock`. Dropping it would break
        // the unmodified-project round trip this bundle exists to keep.
      } else if (
        entry.isFile() &&
        (verbatim || !entry.name.endsWith(".lock")) &&
        !isEntryPointFile(entry.name, isTopLevel)
      ) {
        if (verbatim) {
          // Secrets stay on the machine that holds them. Skipped before the
          // read, and loudly: a `.env` swept into the bundle is a credential
          // stored in every version of the script and handed back on pull.
          if (isLocalSecretFile(entry.name)) {
            log.warn(
              `Skipping ${relPath}: a local secrets file is not part of the dbt project — ` +
                `dbt reads its values from the environment, so set them in the script's ` +
                `environment variables or the descriptor's \`env\``,
            );
            continue;
          }
          // A dbt project's authored files are text. A binary one -- an image
          // under `docs/`, a `.DS_Store`, a parquet seed -- would be read as
          // mojibake and, if it carries a NUL, rejected by Postgres with an
          // opaque `unsupported Unicode escape sequence`, which the push then
          // reports as success. Skip it, loudly: dbt does not read it either.
          //
          // Asked BEFORE reading: the predicate only stats the file and reads
          // its first 8 KB, so a multi-gigabyte seed next to the project costs
          // that rather than being loaded whole just to be rejected.
          const exclusion = moduleFileExclusion(fullPath);
          if (exclusion !== undefined) {
            // Over the limit but readable as text — a large seed CSV is the
            // realistic case — is refused rather than skipped: dbt WOULD have
            // read it, so shipping the project without it deploys something that
            // compiles here and fails at run time with a missing relation.
            if (exclusion === "oversized") {
              throw oversizedModuleFileError(relPath, fs.statSync(fullPath).size);
            }
            log.warn(
              `Skipping ${relPath}: not a text file, so it is not part of the dbt project the ` +
                `bundle carries — dbt does not read it either`,
            );
            continue;
          }
          // `language` is a required field of the API type and is not used for
          // these: the worker writes them to their relative path and dbt reads
          // the tree.
          modules[relPath] = {
            content: fs.readFileSync(fullPath).toString("utf-8"),
            language: "dbt" as ScriptModule["language"],
          };
        } else if (exts.some((ext) => entry.name.endsWith(ext))) {
          const content = readTextFileSync(fullPath);
          const language = inferContentTypeFromFilePath(entry.name, defaultTs);

          // Check for an accompanying lock file (helper.lock)
          const baseName = entry.name.replace(/\.[^.]+$/, '');
          const lockPath = path.join(dirPath, baseName + ".lock");
          let lock: string | undefined;
          if (fs.existsSync(lockPath)) {
            lock = readTextFileSync(lockPath);
          }

          modules[relPath] = {
            content,
            language: language as ScriptModule["language"],
            lock: lock ?? undefined,
          };
        }
      }
    }
  }

  readDir(moduleFolderPath, "");

  if (Object.keys(modules).length === 0) {
    return undefined;
  }

  log.debug(`Found ${Object.keys(modules).length} module(s) in ${moduleFolderPath}`);
  return modules;
}

/**
 * Write module files to a __mod/ directory on disk during pull.
 */
export async function writeModulesToDisk(
  moduleFolderPath: string,
  modules: Record<string, ScriptModule>,
  defaultTs: "bun" | "deno" | undefined
): Promise<void> {
  // Ensure the module folder exists
  fs.mkdirSync(moduleFolderPath, { recursive: true });

  // Clean up stale module files that are no longer in the modules map
  const expectedFiles = new Set<string>();
  for (const [relPath, mod] of Object.entries(modules)) {
    expectedFiles.add(relPath);
    if (mod.lock) {
      expectedFiles.add(relPath.replace(/\.[^.]+$/, '') + ".lock");
    }
  }

  function cleanDir(dirPath: string, relPrefix: string) {
    if (!fs.existsSync(dirPath) || !fs.statSync(dirPath).isDirectory()) return;
    const entries = fs.readdirSync(dirPath, { withFileTypes: true });
    for (const entry of entries) {
      const relPath = relPrefix ? relPrefix + "/" + entry.name : entry.name;
      if (entry.isDirectory()) {
        cleanDir(path.join(dirPath, entry.name), relPath);
        // Remove empty directories after cleaning
        try {
          const remaining = fs.readdirSync(path.join(dirPath, entry.name));
          if (remaining.length === 0) {
            fs.rmdirSync(path.join(dirPath, entry.name));
          }
        } catch {}
      } else if (!expectedFiles.has(relPath)) {
        fs.unlinkSync(path.join(dirPath, entry.name));
      }
    }
  }
  cleanDir(moduleFolderPath, "");

  for (const [relPath, mod] of Object.entries(modules)) {
    const fullPath = path.join(moduleFolderPath, relPath);
    const dir = path.dirname(fullPath);
    fs.mkdirSync(dir, { recursive: true });

    // Write the module content
    fs.writeFileSync(fullPath, mod.content, "utf-8");

    // Write the lock file if present
    if (mod.lock) {
      const baseName = relPath.replace(/\.[^.]+$/, '');
      const lockPath = path.join(moduleFolderPath, baseName + ".lock");
      const lockDir = path.dirname(lockPath);
      fs.mkdirSync(lockDir, { recursive: true });
      fs.writeFileSync(lockPath, mod.lock, "utf-8");
    }
  }
}

async function createScript(
  bundleContent: string | Blob | undefined,
  workspaceId: string,
  body: NewScript,
  workspace: Workspace
): Promise<number> {
  const start = performance.now();
  // Preserve any user draft at this path: a CLI / git-sync deploy must not wipe
  // an in-progress draft the way a UI "deploy from draft" intentionally does.
  body = { ...body, skip_draft_deletion: true };
  // skip_if_noop asks the backend to treat deploys identical to the parent
  // (same content, lockfile, and metadata) as a no-op, so the CLI does not
  // produce phantom git-sync / promotion commits on re-pushes.
  const skipIfNoop = "skip_if_noop=true";
  const extraHeaders = getHeaders();
  if (!bundleContent) {
    try {
      const url =
        workspace.remote +
        "api/w/" +
        workspaceId +
        "/scripts/create?" +
        skipIfNoop;
      const req = await fetch(url, {
        method: "POST",
        headers: {
          Authorization: `Bearer ${workspace.token}`,
          "Content-Type": "application/json",
          ...extraHeaders,
        },
        body: JSON.stringify(body),
      });
      await detectAuthGatewayChallenge(req, url);
      if (req.status != 201) {
        throw Error(
          `${req.status} - ${req.statusText} - ${await req.text()}`
        );
      }
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
        : bundleContent
    );

    const url =
      workspace.remote +
      "api/w/" +
      workspace.workspaceId +
      "/scripts/create_snapshot?" +
      skipIfNoop;
    const req = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${workspace.token} `,
        ...extraHeaders,
      },
      body: form,
    });
    await detectAuthGatewayChallenge(req, url);
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

/**
 * A script metadata file could not be paired with exactly one script file on
 * disk, so nothing can be deployed for it. Distinct from a deploy that reached
 * the remote and was rejected: callers that can carry on with the rest of a
 * changeset catch this specifically.
 */
export class UnresolvableScriptContentFileError extends Error {}

/**
 * A path claimed by both a dbt project and an ordinary script.
 *
 * Its own class because the module push tolerates "no parent found" and must
 * NOT tolerate this: swallowed, the command reports success while deploying
 * nothing.
 */
export class DbtPathCollisionError extends UnresolvableScriptContentFileError {}

/**
 * The dbt project a path would collide with, if there is one.
 *
 * `<base>.py` and `<base>__dbt/` deploy to the SAME remote path, so whichever
 * is pushed last wins and replaces the other's script. The descriptor is
 * optional, so `dbt_project.yml` — not the descriptor — is what says a project
 * is there. Asked on BOTH push paths: an ordinary file goes straight to
 * `handleFile`, a model reaches its parent through `findContentFile`, and a
 * guard on one of them leaves the other silently overwriting.
 */
export async function collidingDbtProject(
  basePath: string
): Promise<string | undefined> {
  const project = basePath + "__dbt/dbt_project.yml";
  return (await stat(project).then(() => true).catch(() => false))
    ? project
    : undefined;
}

/**
 * The ordinary script file sharing a base with a dbt project, if there is one —
 * the same collision as [`collidingDbtProject`], seen from the dbt side.
 *
 * Needed because a descriptor may be pushed DIRECTLY (`wmill script push
 * <base>__dbt/wm_dbt.yaml`), which never passes through the metadata resolution
 * that would otherwise catch it.
 */
export async function collidingOrdinaryScript(
  basePath: string
): Promise<string | undefined> {
  for (const ext of exts) {
    if (ext === "__dbt/" + DBT_DESCRIPTOR_NAME) continue;
    // Both layouts, because both deploy to `basePath`: the flat file, and the
    // folder layout's entry point.
    for (const candidate of [
      basePath + ext,
      `${basePath}${getModuleFolderSuffix()}/script${ext}`,
    ]) {
      const isFile = await stat(candidate)
        .then((s) => s.isFile())
        .catch(() => false);
      if (isFile) return candidate;
    }
  }
  return undefined;
}

export function dbtPathCollisionError(
  project: string,
  other: string
): DbtPathCollisionError {
  return new DbtPathCollisionError(
    `${project} and ${other} deploy to the same path, so pushing either one ` +
      `replaces the other's script. Keep one: move the dbt project to a path ` +
      `of its own, or remove ${other}.`
  );
}


/**
 * A script's content, tolerating the one content file that may not exist: a dbt
 * project's descriptor is optional, and absent means an empty descriptor.
 */
async function readScriptContent(filePath: string): Promise<string> {
  try {
    return await readTextFile(filePath);
  } catch (e) {
    // ONLY a missing file is an empty descriptor. A permission or I/O error on a
    // descriptor that does exist would otherwise deploy the defaults — the
    // `main` warehouse and the whole project — in place of what the file says.
    if (isMissingDbtDescriptor(filePath, e)) return "";
    throw e;
  }
}

export async function findContentFile(filePath: string) {
  // Folder layout: __mod/script.yaml -> __mod/script.ts
  const isModuleFolderMeta = isModuleEntryMetadata(filePath);
  const toCandidate = (ext: string) =>
    isModuleFolderMeta
      ? filePath.replace(MODULE_ENTRY_META_RE, "$1script" + ext)
      : filePath.endsWith("script.json")
      ? filePath.replace(".script.json", ext)
      : filePath.endsWith("script.lock")
      ? filePath.replace(".script.lock", ext)
      : filePath.replace(".script.yaml", ext);
  // Every branch above is a no-op on a path that is neither flat nor
  // module-entry metadata, which would make toCandidate the identity function
  // and "resolve" the input to itself.
  if (!isModuleFolderMeta && !/\.script\.(yaml|json|lock)$/.test(filePath)) {
    throw new UnresolvableScriptContentFileError(
      `${filePath} is not a script metadata file — no script file can be resolved from it.`
    );
  }
  const candidates = exts.map(toCandidate);

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
  // A dbt project's descriptor is OPTIONAL, so `dbt_project.yml` is what says a
  // dbt script lives at this path — the descriptor is often absent from the
  // candidates above while the project is perfectly real. Asked BEFORE the
  // counts below: a project beside an ordinary script is not "one candidate",
  // it is two scripts claiming one remote path, and returning the ordinary one
  // deploys it OVER the dbt script on the next push of any model.
  const dbtCandidate = toCandidate("__dbt/" + DBT_DESCRIPTOR_NAME);
  const dbtProject = await collidingDbtProject(
    dbtCandidate.slice(0, -("__dbt/" + DBT_DESCRIPTOR_NAME).length),
  );
  const nonDbtCandidates = validCandidates.filter((c) => c !== dbtCandidate);
  if (dbtProject && nonDbtCandidates.length > 0) {
    throw dbtPathCollisionError(dbtProject, nonDbtCandidates.join(", "));
  }
  if (validCandidates.length > 1) {
    throw new UnresolvableScriptContentFileError(
      `Multiple script files found next to ${filePath}: ${validCandidates.join(", ")} — ` +
        `cannot tell which one the metadata belongs to. Keep exactly one.`
    );
  }
  if (validCandidates.length < 1) {
    // Resolving to the absent descriptor keeps one content path for every
    // caller; reading it yields an empty descriptor.
    if (dbtProject) {
      return dbtCandidate;
    }
    throw new UnresolvableScriptContentFileError(
      `No script file found next to ${filePath} — a script cannot be deployed from its metadata alone. ` +
        `Add the matching script file (e.g. ${toCandidate(".ts")} or ${toCandidate(
          ".py"
        )}) or remove ${filePath}.`
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
  } else if (language === "rlang") {
    return ".r";
  } else if (language === "dbt") {
    // Not an extension but a path suffix: a dbt script's content file lives
    // inside the project folder, so `<base> + this` is where it belongs.
    return "__dbt/" + DBT_DESCRIPTOR_NAME;
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
  ".r",
  // Not an extension: a dbt script's content file is its descriptor, inside
  // the project folder. `<base>.script.yaml` -> `<base>__dbt/wm_dbt.yaml`.
  "__dbt/" + DBT_DESCRIPTOR_NAME,
  // for related places search: ADD_NEW_LANG
];

/**
 * Whether a path is a script's content file.
 *
 * Separators are normalized first: one "extension" is the path suffix
 * `__dbt/wm_dbt.yaml`, which on Windows is spelled `__dbt\wm_dbt.yaml` and
 * would match nothing — silently skipping every dbt project on that platform.
 */
export function hasScriptExt(p: string): boolean {
  const norm = p.replaceAll("\\", "/");
  return exts.some((ext) => norm.endsWith(ext));
}

export function removeExtensionToPath(path: string): string {
  const norm = path.replaceAll("\\", "/");
  for (const ext of exts) {
    if (norm.endsWith(ext)) {
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
    json?: boolean;
  }
) {
  if (opts.json) log.setSilent(true);
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

  if (opts.json) {
    console.log(JSON.stringify(total));
  } else {
    new Table()
      .header(["path", "summary", "language", "created by"])
      .padding(2)
      .border(true)
      .body(total.map((x) => [x.path, x.summary, x.language, x.created_by]))
      .render();
  }
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
    input = await readTextFile(input.substring(1));
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
    tag?: string;
  },
  path: string
) {
  if (opts.silent) {
    log.setSilent(true);
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = opts.data ? await resolve(opts.data) : {};

  // Validate required args against schema when no data provided
  if (!opts.data) {
    try {
      const script = await wmill.getScriptByPath({
        workspace: workspace.workspaceId,
        path,
      });
      validateRequiredArgs(script.schema as Record<string, unknown>);
    } catch (e: any) {
      if (e.message?.startsWith("Missing required")) throw e;
      log.warn(`Could not fetch schema to validate args: ${e.message}`);
    }
  }

  let id: string;
  try {
    id = await wmill.runScriptByPath({
      workspace: workspace.workspaceId,
      path,
      tag: opts.tag,
      requestBody: input,
    });
  } catch (e: any) {
    if (e?.status === 404) {
      // Script might exist but have a lock/deployment error — check before giving up
      try {
        const script = await wmill.getScriptByPath({
          workspace: workspace.workspaceId,
          path,
        });
        if (script.lock_error_logs) {
          throw new Error(
            `Script '${path}' has a deployment error and cannot be run:\n${script.lock_error_logs}`
          );
        }
      } catch (lookupErr: any) {
        if (lookupErr?.message?.includes("deployment error")) throw lookupErr;
        // Re-throw non-404 lookup errors (e.g. auth/network issues)
        if (lookupErr?.status && lookupErr.status !== 404) throw lookupErr;
      }
      throw new Error(
        `Script '${path}' not found. Run 'wmill script list' to see available scripts.`
      );
    }
    throw e;
  }

  if (!opts.silent) {
    await track_job(workspace.workspaceId, id);
  }

  const MAX_RETRIES = 600; // ~60 seconds at 100ms intervals
  let retries = 0;
  while (retries < MAX_RETRIES) {
    try {
      const completedJob = await wmill.getCompletedJob({
        workspace: workspace.workspaceId,
        id,
      });

      if (completedJob.success === false) {
        process.exitCode = 1;
      }

      const result = completedJob.result ?? {};
      if (opts.silent) {
        console.log(JSON.stringify(result));
      } else {
        log.info(JSON.stringify(result, null, 2));
      }

      break;
    } catch {
      retries++;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  if (retries >= MAX_RETRIES) {
    throw new Error(`Timed out waiting for job ${id} to complete`);
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

export async function pollForJobResult(
  workspace: string,
  jobId: string,
): Promise<{ result: unknown; success: boolean }> {
  return await pollJobWithQueueLogging(workspace, jobId);
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

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const s = await wmill.getScriptByPath({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(s));
  } else {
    console.log(colors.bold("Path:") + " " + s.path);
    console.log(colors.bold("Summary:") + " " + (s.summary ?? ""));
    console.log(colors.bold("Description:") + " " + (s.description ?? ""));
    console.log(colors.bold("Language:") + " " + s.language);
    console.log(colors.bold("Kind:") + " " + (s.kind ?? "script"));
    console.log(colors.bold("Created by:") + " " + (s.created_by ?? ""));
    console.log(colors.bold("Created at:") + " " + (s.created_at ?? ""));
  }
}

const languageAliases: Record<string, ScriptLanguage> = {
  python: "python3",
};

async function bootstrap(
  opts: GlobalOptions & { summary: string; description: string },
  scriptPath: string,
  language: ScriptLanguage | string
) {
  if (!validatePath(scriptPath)) {
    return;
  }

  const resolvedLanguage = (languageAliases[language] ?? language) as ScriptLanguage;

  const scriptInitialCode = scriptBootstrapCode[resolvedLanguage];
  if (scriptInitialCode === undefined) {
    const validLanguages = Object.keys(scriptBootstrapCode).sort().join(", ");
    throw new Error(
      `Unknown language '${language}'. Valid languages: ${validLanguages}`
    );
  }

  const config = await readConfigFile();

  const extension = filePathExtensionFromContentType(
    resolvedLanguage,
    config.defaultTs
  );
  const scriptCodeFileFullPath = scriptPath + extension;
  const scriptMetadataFileFullPath = scriptPath + ".script.yaml";

  try {
    await stat(scriptCodeFileFullPath);
    throw new Error("File already exists: " + scriptCodeFileFullPath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  try {
    await stat(scriptMetadataFileFullPath);
    throw new Error("File already exists: " + scriptMetadataFileFullPath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
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

  const parentDir = path.dirname(scriptCodeFileFullPath);
  await mkdir(parentDir, { recursive: true });

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

export async function generateMetadata(
  opts: GlobalOptions & {
    lockOnly?: boolean;
    schemaOnly?: boolean;
    yes?: boolean;
  } & SyncOptions,
  scriptPath: string | undefined
) {
  log.warn(
    colors.yellow('This command is deprecated. Use "wmill generate-metadata" instead.')
  );
  log.info(
    "This command only works for workspace scripts. For flows or apps, run `wmill generate-metadata` from the affected folder."
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

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies(true);
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
          (!isD && !hasScriptExt(p)) ||
          ignore(p, isD) ||
          isFlowPath(p) ||
          isAppPath(p) ||
          isRawAppPath(p) ||
          // Skip module helper files; only entry points (script.{ext}) are processed
          (isScriptModulePath(p) && !isModuleEntryPoint(p))
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

    // Build a DoubleLinkedDependencyTree and upload mismatched scripts to
    // raw_script_temp before the actual generation pass. Without this,
    // dep jobs for scripts that import other not-yet-deployed scripts via
    // relative paths would 404 on the import target (the very bug this
    // alias was introducing on fresh-DB pushes).
    const { DoubleLinkedDependencyTree, uploadScripts } = await import(
      "../../utils/dependency_tree.ts"
    );
    const tree = new DoubleLinkedDependencyTree();
    tree.setWorkspaceDeps(rawWorkspaceDependencies);
    for (const e of Object.keys(elems)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        true, // dryRun: populate tree
        true,
        rawWorkspaceDependencies,
        codebases,
        false,
        tree,
      );
    }
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
    for (const e of Object.keys(elems)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        false,
        true,
        rawWorkspaceDependencies,
        codebases,
        false,
        tree,
      );
    }
  }
}

async function preview(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
    tag?: string;
  } & SyncOptions,
  filePath: string
) {
  if (opts.silent) {
    log.setSilent(true);
  }
  // Captured before the config read, which chdirs to the wmill.yaml root.
  const cwdBeforeConfig = process.cwd();
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const argPath = filePath;
  filePath = toSyncRootRelativePath(filePath, cwdBeforeConfig);
  const remotePath = scriptPathToRemotePath(filePath);
  assertRemotePath(remotePath, argPath);

  // Same as push: a descriptor-less dbt project's content path is deliberately
  // absent, and the project beside it is what says the script is real.
  const absentDescriptor = await stat(filePath).then(
    () => false,
    (e) => isMissingDbtDescriptor(filePath, e)
  );
  if (!absentDescriptor) {
    const fstat = await stat(filePath);
    if (!fstat.isFile()) {
      throw new Error("file path must refer to a file.");
    }
  }

  if (filePath.endsWith(".script.json") || filePath.endsWith(".script.yaml")) {
    throw Error(
      "Cannot preview a script metadata file, point to the script content file instead (.py, .ts, .go, .sh)"
    );
  }

  const codebases = await listSyncCodebases(opts);
  const language = inferContentTypeFromFilePath(filePath, opts?.defaultTs);
  const content = await readScriptContent(filePath);
  const input = opts.data ? await resolve(opts.data) : {};

  // Read modules from the bundle folder if present. Same suffix and same
  // verbatim read as deploy: a dbt project lives in `__dbt/`, and parsing its
  // files as scripts would drop the `dbt_project.yml` the executor looks for.
  const isFolderLayout = isModuleEntryPoint(filePath);
  const isDbt = language === "dbt";
  const moduleFolderPath = isFolderLayout
    ? path.dirname(filePath)
    : filePath.substring(0, filePath.indexOf(".")) + getModuleFolderSuffix(language);
  const modules = await readModulesFromDisk(
    moduleFolderPath,
    opts?.defaultTs,
    isFolderLayout,
    isDbt
  );

  // Check if this is a codebase script
  const codebase =
    language == "bun" ? findCodebase(filePath, codebases) : undefined;

  // Resolve relative imports from local (not-yet-deployed) content so previewing
  // a script that imports other locally-edited scripts uses the local versions
  // instead of the deployed ones. Shared with `wmill flow preview` so both
  // entry points behave identically; degrades gracefully on older backends.
  // Short-circuit when the script has no relative imports: the full-workspace
  // dependency walk + diff round-trip is pure overhead in that (common) case.
  let tempScriptRefs: Record<string, string> | undefined = undefined;
  const { extractRelativeImports } = await import(
    "../../utils/relative_imports.ts"
  );
  const relImports = await extractRelativeImports(content, remotePath, language);
  if (relImports.length > 0) {
    const { buildPreviewTempScriptRefs } = await import(
      "../generate-metadata/generate-metadata.ts"
    );
    tempScriptRefs = await buildPreviewTempScriptRefs(
      workspace,
      opts,
      codebases,
      { kind: "script", path: filePath }
    );
  }

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
      const esbuild = await getEsbuild();

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
        if (!opts.silent) {
          log.info(`Creating tarball for multiple output files...`);
        }
        const mainPath = filePath.split(SEP).pop()?.split(".")[0] + ".js";
        const mainContent =
          out.outputFiles.find((file: OutputFile) => file.path == "/" + mainPath)?.text ?? "";
        const entries: TarEntry[] = [
          { name: "main.js", content: mainContent },
        ];
        for (const file of out.outputFiles) {
          if (file.path == "/" + mainPath) continue;
          entries.push({ name: file.path.substring(1), content: file.contents });
        }
        bundledContent = await createTarBlob(entries);
        isTar = true;
      } else if (Array.isArray(codebase.assets) && codebase.assets.length > 0) {
        // Handle assets
        if (!opts.silent) {
          log.info(`Adding assets to tarball...`);
        }
        const entries: TarEntry[] = [
          { name: "main.js", content: bundledContent },
        ];
        for (const asset of codebase.assets) {
          const data = fs.readFileSync(asset.from);
          entries.push({ name: asset.to, content: data });
        }
        bundledContent = await createTarBlob(entries);
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
      path: remotePath,
      args: input,
      language: language,
      tag: opts.tag,
      kind: isTar ? "tarbundle" : "bundle",
      format: codebase?.format ?? "cjs",
      temp_script_refs: tempScriptRefs,
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

    const extraHeaders = getHeaders();
    const response = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${workspace.token}`,
        ...extraHeaders,
      },
      body: form,
    });

    await detectAuthGatewayChallenge(response, url);

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
    // For regular scripts, start the preview job then poll for completion
    const jobId = await wmill.runScriptPreview({
      workspace: workspace.workspaceId,
      requestBody: {
        content,
        path: remotePath,
        args: input,
        language: language as any,
        tag: opts.tag,
        modules: modules ?? undefined,
        temp_script_refs: tempScriptRefs,
      },
    });

    const { result, success } = await pollForJobResult(workspace.workspaceId, jobId);

    if (!success) {
      if (opts.silent) {
        console.log(JSON.stringify(result));
      } else {
        log.info(colors.red.bold("Preview failed"));
        log.info(JSON.stringify(result, null, 2));
      }
      process.exitCode = 1;
      return;
    }

    if (opts.silent) {
      console.log(JSON.stringify(result));
    } else {
      log.info(colors.bold.underline.green("Preview completed"));
      log.info(JSON.stringify(result, null, 2));
    }
  }
}

async function history(
  opts: GlobalOptions & { json?: boolean },
  scriptPath: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const versions = await wmill.getScriptHistoryByPath({
    workspace: workspace.workspaceId,
    path: scriptPath,
  });

  if (opts.json) {
    console.log(JSON.stringify(versions));
  } else {
    if (versions.length === 0) {
      log.info("No version history found for " + scriptPath);
      return;
    }
    new Table()
      .header(["#", "Hash", "Created At", "Deployment Message"])
      .padding(2)
      .border(true)
      .body(
        versions.map((v, i) => [
          String(versions.length - i),
          v.script_hash,
          v.created_at ? new Date(v.created_at).toLocaleString() : "-",
          v.deployment_msg ?? "-",
        ])
      )
      .render();
  }
}

async function setPermissionedAs(
  opts: GlobalOptions,
  scriptPath: string,
  email: string,
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const remote = await wmill.getScriptByPath({
    workspace: workspace.workspaceId,
    path: scriptPath,
  });
  if (!remote) throw new Error(`Script ${scriptPath} not found`);

  await wmill.createScript({
    workspace: workspace.workspaceId,
    requestBody: {
      ...(remote as any),
      lock: Array.isArray(remote.lock) ? remote.lock.join("\n") : remote.lock ?? undefined,
      parent_hash: remote.hash,
      on_behalf_of_email: email,
      // The principal is derived server-side from the email, which resolves workspace
      // members, groups and superadmins acting outside their workspaces alike — a
      // client-side `usr` lookup would see only the first of those.
      on_behalf_of: undefined,
      preserve_on_behalf_of: true,
      // Preserve any user draft at this path (see backend skip_draft_deletion).
      skip_draft_deletion: true,
    },
  });
  log.info(colors.green(`Updated permissioned_as for script ${scriptPath} to ${email}`));
}

const command = new Command()
  .description("script related commands")
  .option("--show-archived", "Show archived scripts instead of active ones")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all scripts")
  .option("--show-archived", "Show archived scripts instead of active ones")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command(
    "push",
    "push a local script spec. This overrides any remote versions. Use the script file (.ts, .js, .py, .sh)"
  )
  .arguments("<path:file>")
  .option("--message <message:string>", "Deployment message")
  .action(push as any)
  .command("get", "get a script's details")
  .arguments("<path:file>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("show", "show a script's content (alias for get)")
  .arguments("<path:file>")
  .action(show as any)
  .command("run", "run a script by path")
  .arguments("<path:file>")
  .option(
    "-d --data <data:file>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-. A resource argument is the bare string $res:<path> as its whole value, and a variable argument is the bare string $var:<path> — not an object wrapper keyed on $res/$var, and not a plain path."
  )
  .option(
    "-s --silent",
    "Do not output anything other then the final output. Useful for scripting."
  )
  .option(
    "--tag <tag:string>",
    "Override the worker tag the run is dispatched to (e.g. to route it to dev workers instead of the script's default tag)."
  )
  .action(run as any)
  .command(
    "preview",
    "preview a local script without deploying it. Supports both regular and codebase scripts."
  )
  .arguments("<path:file>")
  .option(
    "-d --data <data:file>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-. A resource argument is the bare string $res:<path> as its whole value, and a variable argument is the bare string $var:<path> — not an object wrapper keyed on $res/$var, and not a plain path."
  )
  .option(
    "-s --silent",
    "Do not output anything other than the final output. Useful for scripting."
  )
  .option(
    "--tag <tag:string>",
    "Override the worker tag the preview is dispatched to (e.g. to route it to dev workers instead of the script's default tag)."
  )
  .action(preview as any)
  .command("new", "create a new script")
  .arguments("<path:file> <language:string>")
  .option("--summary <summary:string>", "script summary")
  .option("--description <description:string>", "script description")
  .action(bootstrap as any)
  .command("bootstrap", "create a new script (alias for new)")
  .arguments("<path:file> <language:string>")
  .option("--summary <summary:string>", "script summary")
  .option("--description <description:string>", "script description")
  .action(bootstrap as any)
  .command(
    "generate-metadata",
    'DEPRECATED: re-generate script metadata. Use top-level "wmill generate-metadata" instead.'
  )
  // Deprecated compatibility command. Keep it working for older repos, but
  // exclude it from generated system prompt docs.
  // @deprecated use `wmill generate-metadata`
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
  .action(generateMetadata as any)
  .command(
    "set-permissioned-as",
    "Set the on_behalf_of_email for a script (requires admin or wm_deployers group)"
  )
  .arguments("<path:string> <email:string>")
  .action(setPermissionedAs as any)
  .command(
    "history",
    "show version history for a script"
  )
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(history as any);

export default command;
