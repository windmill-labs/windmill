// Import only the services actually used in this file (not re-exported)
// This enables tree-shaking - importing setClient won't pull in all services
import {
  ResourceService,
  VariableService,
  JobService,
  HelpersService,
  AppService,
  MetricsService,
  OidcService,
  UserService,
} from "./services.gen";
import { OpenAPI } from "./core/OpenAPI";
// import type { DenoS3LightClientSettings } from "./index";
import {
  DenoS3LightClientSettings,
  S3ObjectRecord,
  type S3Object,
} from "./s3Types";

export {
  type S3Object,
  type S3ObjectRecord,
  type S3ObjectURI,
} from "./s3Types";
export {
  datatable,
  ducklake,
  type SqlTemplateFunction,
  type DatatableSqlTemplateFunction,
} from "./sqlUtils";

// Services are NOT re-exported here to enable tree-shaking
// Import services directly from "windmill-client" or use the default export

export type Sql = string;
export type Email = string;
export type Base64 = string;
export type Resource<S extends string> = any;

export const SHARED_FOLDER = "/shared";

let mockedApi: MockedApi | undefined = undefined;

/**
 * Initialize the Windmill client with authentication token and base URL
 * @param token - Authentication token (defaults to WM_TOKEN env variable)
 * @param baseUrl - API base URL (defaults to BASE_INTERNAL_URL or BASE_URL env variable)
 */
export function setClient(token?: string, baseUrl?: string) {
  if (baseUrl === undefined) {
    baseUrl =
      getEnv("BASE_INTERNAL_URL") ??
      getEnv("BASE_URL") ??
      "http://localhost:8000";
  }
  if (token === undefined) {
    token = getEnv("WM_TOKEN") ?? "no_token";
  }
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
}

function getPublicBaseUrl(): string {
  return getEnv("WM_BASE_URL") ?? "http://localhost:3000";
}

const getEnv = (key: string) => {
  if (typeof window === "undefined") {
    // node
    return process?.env?.[key];
  }
  // browser
  return window?.process?.env?.[key];
};

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
export function getWorkspace(): string {
  return getEnv("WM_WORKSPACE") ?? "no_workspace";
}

/**
 * Get a resource value by path
 * @param path path of the resource,  default to internal state path
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
export async function getResource(
  path?: string,
  undefinedIfEmpty?: boolean
): Promise<any> {
  path = parseResourceSyntax(path) ?? path ?? getStatePath();
  const mockedApi = await getMockedApi();
  if (mockedApi) {
    if (mockedApi.resources[path]) {
      return mockedApi.resources[path];
    } else {
      console.log(
        `MockedAPI present, but resource not found at ${path}, falling back to real API`
      );
    }
  }

  const workspace = getWorkspace();

  try {
    return await ResourceService.getResourceValueInterpolated({
      workspace,
      path,
    });
  } catch (e: any) {
    if (undefinedIfEmpty && e.status === 404) {
      return undefined;
    } else {
      throw Error(
        `Resource not found at ${path} or not visible to you: ${e.body}`
      );
    }
  }
}

/**
 * Get the true root job id
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
export async function getRootJobId(jobId?: string): Promise<string> {
  const workspace = getWorkspace();
  jobId = jobId ?? getEnv("WM_JOB_ID");
  if (jobId === undefined) {
    throw Error("Job ID not set");
  }
  return await JobService.getRootJobId({ workspace, id: jobId });
}

/**
 * @deprecated Use runScriptByPath or runScriptByHash instead
 */
export async function runScript(
  path: string | null = null,
  hash_: string | null = null,
  args: Record<string, any> | null = null,
  verbose: boolean = false
): Promise<any> {
  console.warn(
    "runScript is deprecated. Use runScriptByPath or runScriptByHash instead."
  );
  if (path && hash_) {
    throw new Error("path and hash_ are mutually exclusive");
  }
  return _runScriptInternal(path, hash_, args, verbose);
}

async function _runScriptInternal(
  path: string | null = null,
  hash_: string | null = null,
  args: Record<string, any> | null = null,
  verbose: boolean = false
): Promise<any> {
  args = args || {};

  if (verbose) {
    if (path) {
      console.info(`running \`${path}\` synchronously with args:`, args);
    } else if (hash_) {
      console.info(
        `running script with hash \`${hash_}\` synchronously with args:`,
        args
      );
    }
  }

  const jobId = await _runScriptAsyncInternal(path, hash_, args);
  return await waitJob(jobId, verbose);
}

/**
 * Run a script synchronously by its path and wait for the result
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @returns Script execution result
 */
export async function runScriptByPath(
  path: string,
  args: Record<string, any> | null = null,
  verbose: boolean = false
): Promise<any> {
  return _runScriptInternal(path, null, args, verbose);
}

/**
 * Run a script synchronously by its hash and wait for the result
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @returns Script execution result
 */
export async function runScriptByHash(
  hash_: string,
  args: Record<string, any> | null = null,
  verbose: boolean = false
): Promise<any> {
  return _runScriptInternal(null, hash_, args, verbose);
}

/**
 * Append a text to the result stream
 * @param text text to append to the result stream
 */
export function appendToResultStream(text: string) {
  console.log("WM_STREAM: " + text.replace(/\n/g, "\\n"));
}

/**
 * Stream to the result stream
 * @param stream stream to stream to the result stream
 */
export async function streamResult(stream: AsyncIterable<string>) {
  for await (const text of stream) {
    appendToResultStream(text);
  }
}

/**
 * Run a flow synchronously by its path and wait for the result
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param verbose - Enable verbose logging
 * @returns Flow execution result
 */
export async function runFlow(
  path: string | null = null,
  args: Record<string, any> | null = null,
  verbose: boolean = false
): Promise<any> {
  args = args || {};

  if (verbose) {
    console.info(`running \`${path}\` synchronously with args:`, args);
  }

  const jobId = await runFlowAsync(path, args, null, false);
  return await waitJob(jobId, verbose);
}

/**
 * Wait for a job to complete and return its result
 * @param jobId - ID of the job to wait for
 * @param verbose - Enable verbose logging
 * @returns Job result when completed
 */
export async function waitJob(
  jobId: string,
  verbose: boolean = false
): Promise<any> {
  while (true) {
    // Implement your HTTP request logic here to get job result
    const resultRes = await getResultMaybe(jobId);

    const started = resultRes.started;
    const completed = resultRes.completed;
    const success = resultRes.success;

    if (!started && verbose) {
      console.info(`job ${jobId} has not started yet`);
    }

    if (completed) {
      const result = resultRes.result;
      if (success) {
        return result;
      } else {
        const error = result.error;
        throw new Error(
          `Job ${jobId} was not successful: ${JSON.stringify(error)}`
        );
      }
    }

    if (verbose) {
      console.info(`sleeping 0.5 seconds for jobId: ${jobId}`);
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }
}

/**
 * Get the result of a completed job
 * @param jobId - ID of the completed job
 * @returns Job result
 */
export async function getResult(jobId: string): Promise<any> {
  const workspace = getWorkspace();
  return await JobService.getCompletedJobResult({ workspace, id: jobId });
}

/**
 * Get the result of a job if completed, or its current status
 * @param jobId - ID of the job
 * @returns Object with started, completed, success, and result properties
 */
export async function getResultMaybe(jobId: string): Promise<any> {
  const workspace = getWorkspace();
  return await JobService.getCompletedJobResultMaybe({ workspace, id: jobId });
}
const STRIP_COMMENTS =
  /(\/\/.*$)|(\/\*[\s\S]*?\*\/)|(\s*=[^,\)]*(('(?:\\'|[^'\r\n])*')|("(?:\\"|[^"\r\n])*"))|(\s*=[^,\)]*))/gm;
const ARGUMENT_NAMES = /([^\s,]+)/g;
function getParamNames(func: Function): string[] {
  const fnStr = func.toString().replace(STRIP_COMMENTS, "");
  let result: string[] | null = fnStr
    .slice(fnStr.indexOf("(") + 1, fnStr.indexOf(")"))
    .match(ARGUMENT_NAMES);
  if (result === null) result = [];
  return result;
}

/**
 * Wrap a function to execute as a Windmill task within a flow context
 * @param f - Function to wrap as a task
 * @returns Async wrapper function that executes as a Windmill job
 */
export function task<P, T>(f: (_: P) => T): (_: P) => Promise<T> {
  return async (...y) => {
    const args: Record<string, any> = {};
    const paramNames = getParamNames(f);
    y.forEach((x, i) => (args[paramNames[i]] = x));
    let req = await fetch(
      `${OpenAPI.BASE}/w/${getWorkspace()}/jobs/run/workflow_as_code/${getEnv(
        "WM_JOB_ID"
      )}/${f.name}`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${getEnv("WM_TOKEN")}`,
        },
        body: JSON.stringify({ args }),
      }
    );
    let jobId = await req.text();
    console.log(`Started task ${f.name} as job ${jobId}`);
    let r = await waitJob(jobId);
    console.log(`Task ${f.name} (${jobId}) completed`);
    return r;
  };
}

/**
 * @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
 */
export async function runScriptAsync(
  path: string | null,
  hash_: string | null,
  args: Record<string, any> | null,
  scheduledInSeconds: number | null = null
): Promise<string> {
  console.warn(
    "runScriptAsync is deprecated. Use runScriptByPathAsync or runScriptByHashAsync instead."
  );
  // Create a script job and return its job id.
  if (path && hash_) {
    throw new Error("path and hash_ are mutually exclusive");
  }
  return _runScriptAsyncInternal(path, hash_, args, scheduledInSeconds);
}

async function _runScriptAsyncInternal(
  path: string | null = null,
  hash_: string | null = null,
  args: Record<string, any> | null = null,
  scheduledInSeconds: number | null = null
): Promise<string> {
  // Create a script job and return its job id.
  args = args || {};
  const params: Record<string, any> = {};

  if (scheduledInSeconds) {
    params["scheduled_in_secs"] = scheduledInSeconds;
  }

  let parentJobId = getEnv("WM_JOB_ID");
  if (parentJobId !== undefined) {
    params["parent_job"] = parentJobId;
  }

  let rootJobId = getEnv("WM_ROOT_FLOW_JOB_ID");
  if (rootJobId != undefined && rootJobId != "") {
    params["root_job"] = rootJobId;
  }

  let endpoint: string;
  if (path) {
    endpoint = `/w/${getWorkspace()}/jobs/run/p/${path}`;
  } else if (hash_) {
    endpoint = `/w/${getWorkspace()}/jobs/run/h/${hash_}`;
  } else {
    throw new Error("path or hash_ must be provided");
  }

  let url = new URL(OpenAPI.BASE + endpoint);
  url.search = new URLSearchParams(params).toString();

  return fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${OpenAPI.TOKEN}`,
    },
    body: JSON.stringify(args),
  }).then((res) => res.text());
}

/**
 * Run a script asynchronously by its path
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @returns Job ID of the created job
 */
export async function runScriptByPathAsync(
  path: string,
  args: Record<string, any> | null = null,
  scheduledInSeconds: number | null = null
): Promise<string> {
  return _runScriptAsyncInternal(path, null, args, scheduledInSeconds);
}

/**
 * Run a script asynchronously by its hash
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @returns Job ID of the created job
 */
export async function runScriptByHashAsync(
  hash_: string,
  args: Record<string, any> | null = null,
  scheduledInSeconds: number | null = null
): Promise<string> {
  return _runScriptAsyncInternal(null, hash_, args, scheduledInSeconds);
}

/**
 * Run a flow asynchronously by its path
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @param doNotTrackInParent - If false, tracks state in parent job (only use when fully awaiting the job)
 * @returns Job ID of the created job
 */
export async function runFlowAsync(
  path: string | null,
  args: Record<string, any> | null,
  scheduledInSeconds: number | null = null,
  // can only be set to false if this the job will be fully await and not concurrent with any other job
  // as otherwise the child flow and its own child will store their state in the parent job which will
  // lead to incorrectness and failures
  doNotTrackInParent: boolean = true
): Promise<string> {
  // Create a script job and return its job id.

  args = args || {};
  const params: Record<string, any> = {};

  if (scheduledInSeconds) {
    params["scheduled_in_secs"] = scheduledInSeconds;
  }

  if (!doNotTrackInParent) {
    let parentJobId = getEnv("WM_JOB_ID");
    if (parentJobId !== undefined) {
      params["parent_job"] = parentJobId;
    }
    let rootJobId = getEnv("WM_ROOT_FLOW_JOB_ID");
    if (rootJobId != undefined && rootJobId != "") {
      params["root_job"] = rootJobId;
    }
  }

  let endpoint: string;
  if (path) {
    endpoint = `/w/${getWorkspace()}/jobs/run/f/${path}`;
  } else {
    throw new Error("path must be provided");
  }
  let url = new URL(OpenAPI.BASE + endpoint);
  url.search = new URLSearchParams(params).toString();

  return fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${OpenAPI.TOKEN}`,
    },
    body: JSON.stringify(args),
  }).then((res) => res.text());
}

/**
 * Resolve a resource value in case the default value was picked because the input payload was undefined
 * @param obj resource value or path of the resource under the format `$res:path`
 * @returns resource value
 */
export async function resolveDefaultResource(obj: any): Promise<any> {
  if (typeof obj === "string" && obj.startsWith("$res:")) {
    return await getResource(obj.substring(5), true);
  } else {
    return obj;
  }
}

/**
 * Get the state file path from environment variables
 * @returns State path string
 */
export function getStatePath(): string {
  const state_path = getEnv("WM_STATE_PATH_NEW") ?? getEnv("WM_STATE_PATH");
  if (state_path === undefined) {
    throw Error("State path not set");
  }
  return state_path;
}

/**
 * Set a resource value by path
 * @param path path of the resource to set, default to state path
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
export async function setResource(
  value: any,
  path?: string,
  initializeToTypeIfNotExist?: string
): Promise<void> {
  path = parseResourceSyntax(path) ?? path ?? getStatePath();
  const mockedApi = await getMockedApi();
  if (mockedApi) {
    mockedApi.resources[path] = value;
    return;
  }
  const workspace = getWorkspace();
  if (await ResourceService.existsResource({ workspace, path })) {
    await ResourceService.updateResourceValue({
      workspace,
      path,
      requestBody: { value },
    });
  } else if (initializeToTypeIfNotExist) {
    await ResourceService.createResource({
      workspace,
      requestBody: { path, value, resource_type: initializeToTypeIfNotExist },
    });
  } else {
    throw Error(
      `Resource at path ${path} does not exist and no type was provided to initialize it`
    );
  }
}

/**
 * Set the state
 * @param state state to set
 * @deprecated use setState instead
 */
export async function setInternalState(state: any): Promise<void> {
  await setResource(state, undefined, "state");
}

/**
 * Set the state
 * @param state state to set
 * @param path Optional state resource path override. Defaults to `getStatePath()`.
 */
export async function setState(state: any, path?: string): Promise<void> {
  await setResource(state, path ?? getStatePath(), "state");
}

/**
 * Set the progress
 * Progress cannot go back and limited to 0% to 99% range
 * @param percent Progress to set in %
 * @param jobId? Job to set progress for
 */
export async function setProgress(percent: number, jobId?: any): Promise<void> {
  const workspace = getWorkspace();
  let flowId = getEnv("WM_FLOW_JOB_ID");

  // If jobId specified we need to find if there is a parent/flow
  if (jobId) {
    const job = await JobService.getJob({
      id: jobId ?? "NO_JOB_ID",
      workspace,
      noLogs: true,
    });

    // Could be actual flowId or undefined
    flowId = job.parent_job;
  }

  await MetricsService.setJobProgress({
    id: jobId ?? getEnv("WM_JOB_ID") ?? "NO_JOB_ID",
    workspace,
    requestBody: {
      // In case user inputs float, it should be converted to int
      percent: Math.floor(percent),
      flow_job_id: flowId == "" ? undefined : flowId,
    },
  });
}

/**
 * Get the progress
 * @param jobId? Job to get progress from
 * @returns Optional clamped between 0 and 100 progress value
 */
export async function getProgress(jobId?: any): Promise<number | null> {
  // TODO: Delete or set to 100 completed job metrics
  return await MetricsService.getJobProgress({
    id: jobId ?? getEnv("WM_JOB_ID") ?? "NO_JOB_ID",
    workspace: getWorkspace(),
  });
}

/**
 * Set a flow user state
 * @param key key of the state
 * @param value value of the state

 */
export async function setFlowUserState(
  key: string,
  value: any,
  errorIfNotPossible?: boolean
): Promise<void> {
  if (value === undefined) {
    value = null;
  }
  const workspace = getWorkspace();
  try {
    await JobService.setFlowUserState({
      workspace,
      id: await getRootJobId(),
      key,
      requestBody: value,
    });
  } catch (e: any) {
    if (errorIfNotPossible) {
      throw Error(`Error setting flow user state at ${key}: ${e.body}`);
    } else {
      console.error(`Error setting flow user state at ${key}: ${e.body}`);
    }
  }
}

/**
 * Get a flow user state
 * @param path path of the variable

 */
export async function getFlowUserState(
  key: string,
  errorIfNotPossible?: boolean
): Promise<any> {
  const workspace = getWorkspace();
  try {
    return await JobService.getFlowUserState({
      workspace,
      id: await getRootJobId(),
      key,
    });
  } catch (e: any) {
    if (errorIfNotPossible) {
      throw Error(`Error setting flow user state at ${key}: ${e.body}`);
    } else {
      console.error(`Error setting flow user state at ${key}: ${e.body}`);
    }
  }
}

/**
 * Get the internal state
 * @deprecated use getState instead
 */
export async function getInternalState(): Promise<any> {
  return await getResource(getStatePath(), true);
}

/**
 * Get the state shared across executions
 * @param path Optional state resource path override. Defaults to `getStatePath()`.
 */
export async function getState(path?: string): Promise<any> {
  return await getResource(path ?? getStatePath(), true);
}

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
export async function getVariable(path: string): Promise<string> {
  path = parseVariableSyntax(path) ?? path;
  const mockedApi = await getMockedApi();
  if (mockedApi) {
    if (mockedApi.variables[path]) {
      return mockedApi.variables[path];
    } else {
      console.log(
        `MockedAPI present, but variable not found at ${path}, falling back to real API`
      );
    }
  }
  const workspace = getWorkspace();
  try {
    return await VariableService.getVariableValue({ workspace, path });
  } catch (e: any) {
    throw Error(
      `Variable not found at ${path} or not visible to you: ${e.body}`
    );
  }
}

/**
 * Set a variable by path, create if not exist
 * @param path path of the variable
 * @param value value of the variable
 * @param isSecretIfNotExist if the variable does not exist, create it as secret or not (default: false)
 * @param descriptionIfNotExist if the variable does not exist, create it with this description (default: "")
 */
export async function setVariable(
  path: string,
  value: string,
  isSecretIfNotExist?: boolean,
  descriptionIfNotExist?: string
): Promise<void> {
  path = parseVariableSyntax(path) ?? path;
  const mockedApi = await getMockedApi();
  if (mockedApi) {
    mockedApi.variables[path] = value;
    return;
  }
  const workspace = getWorkspace();
  if (await VariableService.existsVariable({ workspace, path })) {
    await VariableService.updateVariable({
      workspace,
      path,
      requestBody: { value },
    });
  } else {
    await VariableService.createVariable({
      workspace,
      requestBody: {
        path,
        value,
        is_secret: isSecretIfNotExist ?? false,
        description: descriptionIfNotExist ?? "",
      },
    });
  }
}

/**
 * Build a PostgreSQL connection URL from a database resource
 * @param path - Path to the database resource
 * @returns PostgreSQL connection URL string
 */
export async function databaseUrlFromResource(path: string): Promise<string> {
  const resource = await getResource(path);
  return `postgresql://${resource.user}:${resource.password}@${resource.host}:${resource.port}/${resource.dbname}?sslmode=${resource.sslmode}`;
}

// TODO(gb): need to investigate more how Polars and DuckDB work in TS
// export async function polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any> {
//   const workspace = getWorkspace();
//   return await HelpersService.polarsConnectionSettingsV2({
//     workspace: workspace,
// 		requestBody: {
// 			s3_resource_path: s3_resource_path
// 		}
//   });
// }

// export async function duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any> {
//   const workspace = getWorkspace();
//   return await HelpersService.duckdbConnectionSettingsV2({
//     workspace: workspace,
// 		requestBody: {
// 			s3_resource_path: s3_resource_path
// 		}
//   });
// }

/**
 * Get S3 client settings from a resource or workspace default
 * @param s3_resource_path - Path to S3 resource (uses workspace default if undefined)
 * @returns S3 client configuration settings
 */
export async function denoS3LightClientSettings(
  s3_resource_path: string | undefined
): Promise<DenoS3LightClientSettings> {
  const workspace = getWorkspace();
  const s3Resource = await HelpersService.s3ResourceInfo({
    workspace: workspace,
    requestBody: {
      s3_resource_path:
        parseResourceSyntax(s3_resource_path) ?? s3_resource_path,
    },
  });
  let settings: DenoS3LightClientSettings = {
    ...s3Resource,
  };
  return settings;
}

/**
 * Load the content of a file stored in S3. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * let fileContent = await wmill.loadS3FileContent(inputFile)
 * // if the file is a raw text file, it can be decoded and printed directly:
 * const text = new TextDecoder().decode(fileContentStream)
 * console.log(text);
 * ```
 */
export async function loadS3File(
  s3object: S3Object,
  s3ResourcePath: string | undefined = undefined
): Promise<Uint8Array | undefined> {
  const fileContentBlob = await loadS3FileStream(s3object, s3ResourcePath);
  if (fileContentBlob === undefined) {
    return undefined;
  }

  // we read the stream until completion and put the content in an Uint8Array
  const reader = fileContentBlob.stream().getReader();
  const chunks: Uint8Array[] = [];
  while (true) {
    const { value: chunk, done } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(chunk);
  }
  let fileContentLength = 0;
  chunks.forEach((item) => {
    fileContentLength += item.length;
  });
  let fileContent = new Uint8Array(fileContentLength);
  let offset = 0;
  chunks.forEach((chunk) => {
    fileContent.set(chunk, offset);
    offset += chunk.length;
  });
  return fileContent;
}

/**
 * Load the content of a file stored in S3 as a stream. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * let fileContentBlob = await wmill.loadS3FileStream(inputFile)
 * // if the content is plain text, the blob can be read directly:
 * console.log(await fileContentBlob.text());
 * ```
 */
export async function loadS3FileStream(
  s3object: S3Object,
  s3ResourcePath: string | undefined = undefined
): Promise<Blob | undefined> {
  let s3Obj = s3object && parseS3Object(s3object);
  let params: Record<string, string> = {};
  params["file_key"] = s3Obj.s3;
  if (s3ResourcePath !== undefined) {
    params["s3_resource_path"] = s3ResourcePath;
  }
  if (s3Obj.storage !== undefined) {
    params["storage"] = s3Obj.storage;
  }
  const queryParams = new URLSearchParams(params);

  // We use raw fetch here b/c OpenAPI generated client doesn't handle Blobs nicely
  const response = await fetch(
    `${
      OpenAPI.BASE
    }/w/${getWorkspace()}/job_helpers/download_s3_file?${queryParams}`,
    {
      method: "GET",
      headers: {
        Authorization: `Bearer ${OpenAPI.TOKEN}`,
      },
    }
  );

  // Check if the response was successful
  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(
      `Failed to load S3 file: ${response.status} ${response.statusText} - ${errorText}`
    );
  }

  return response.blob();
}

/**
 * Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * const s3object = await writeS3File(s3Object, "Hello Windmill!")
 * const fileContentAsUtf8Str = (await s3object.toArray()).toString('utf-8')
 * console.log(fileContentAsUtf8Str)
 * ```
 */
export async function writeS3File(
  s3object: S3Object | undefined,
  fileContent: string | Blob,
  s3ResourcePath: string | undefined = undefined,
  contentType: string | undefined = undefined,
  contentDisposition: string | undefined = undefined
): Promise<S3Object> {
  let fileContentBlob: Blob;
  if (typeof fileContent === "string") {
    fileContentBlob = new Blob([fileContent as string], {
      type: "text/plain",
    });
  } else {
    fileContentBlob = fileContent as Blob;
  }

  let s3Obj = s3object && parseS3Object(s3object);

  const response = await HelpersService.fileUpload({
    workspace: getWorkspace(),
    fileKey: s3Obj?.s3,
    fileExtension: undefined,
    s3ResourcePath: s3ResourcePath,
    requestBody: fileContentBlob,
    storage: s3Obj?.storage,
    contentType,
    contentDisposition,
  });
  return {
    s3: response.file_key,
    ...(s3Obj?.storage && { storage: s3Obj?.storage }),
  };
}

/**
 * Sign S3 objects to be used by anonymous users in public apps
 * @param s3objects s3 objects to sign
 * @returns signed s3 objects
 */
export async function signS3Objects(
  s3objects: S3Object[]
): Promise<S3Object[]> {
  const signedKeys = await AppService.signS3Objects({
    workspace: getWorkspace(),
    requestBody: {
      s3_objects: s3objects.map(parseS3Object),
    },
  });
  return signedKeys;
}
/**
 * Sign S3 object to be used by anonymous users in public apps
 * @param s3object s3 object to sign
 * @returns signed s3 object
 */
export async function signS3Object(s3object: S3Object): Promise<S3Object> {
  const [signedObject] = await signS3Objects([s3object]);
  return signedObject;
}

/**
 * Generate a presigned public URL for an array of S3 objects.
 * If an S3 object is not signed yet, it will be signed first.
 * @param s3Objects s3 objects to sign
 * @returns list of signed public URLs
 */
export async function getPresignedS3PublicUrls(
  s3Objects: S3Object[],
  { baseUrl }: { baseUrl?: string } = {}
): Promise<string[]> {
  baseUrl ??= getPublicBaseUrl();

  const s3Objs = s3Objects.map(parseS3Object);

  // Sign all S3 objects that need to be signed in one go
  const s3ObjsToSign: (readonly [S3ObjectRecord, number])[] = s3Objs
    .map((s3Obj, index) => [s3Obj, index] as const)
    .filter(([s3Obj, _]) => s3Obj.presigned === undefined);
  if (s3ObjsToSign.length > 0) {
    const signedS3Objs = await signS3Objects(
      s3ObjsToSign.map(([s3Obj, _]) => s3Obj)
    );
    for (let i = 0; i < s3ObjsToSign.length; i++) {
      const [_, originalIndex] = s3ObjsToSign[i];
      s3Objs[originalIndex] = parseS3Object(signedS3Objs[i]);
    }
  }

  const signedUrls: string[] = [];
  for (const s3Obj of s3Objs) {
    const { s3, presigned, storage = "_default_" } = s3Obj;
    const signedUrl = `${baseUrl}/api/w/${getWorkspace()}/s3_proxy/${storage}/${s3}?${presigned}`;
    signedUrls.push(signedUrl);
  }
  return signedUrls;
}

/**
 * Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
 * @param s3Object s3 object to sign
 * @returns signed public URL
 */
export async function getPresignedS3PublicUrl(
  s3Objects: S3Object,
  { baseUrl }: { baseUrl?: string } = {}
): Promise<string> {
  const [s3Object] = await getPresignedS3PublicUrls([s3Objects], { baseUrl });
  return s3Object;
}

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @param flowLevel if true, generate resume URLs for the parent flow instead of the specific step.
 *                  This allows pre-approvals that can be consumed by any later suspend step in the same flow.
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
export async function getResumeUrls(
  approver?: string,
  flowLevel?: boolean
): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}> {
  const nonce = Math.floor(Math.random() * 4294967295);
  const workspace = getWorkspace();
  return await JobService.getResumeUrls({
    workspace,
    resumeId: nonce,
    approver,
    flowLevel,
    id: getEnv("WM_JOB_ID") ?? "NO_JOB_ID",
  });
}

/**
 * @deprecated use getResumeUrls instead
 */
export function getResumeEndpoints(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}> {
  return getResumeUrls(approver);
}

/**
 * Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
 * @param audience audience of the token
 * @param expiresIn Optional number of seconds until the token expires
 * @returns jwt token
 */
export async function getIdToken(
  audience: string,
  expiresIn?: number
): Promise<string> {
  const workspace = getWorkspace();
  return await OidcService.getOidcToken({
    workspace,
    audience,
    expiresIn,
  });
}

/**
 * Convert a base64-encoded string to Uint8Array
 * @param data - Base64-encoded string
 * @returns Decoded Uint8Array
 */
export function base64ToUint8Array(data: string): Uint8Array {
  return Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
}

/**
 * Convert a Uint8Array to base64-encoded string
 * @param arrayBuffer - Uint8Array to encode
 * @returns Base64-encoded string
 */
export function uint8ArrayToBase64(arrayBuffer: Uint8Array): string {
  let base64 = "";
  const encodings =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

  const bytes = new Uint8Array(arrayBuffer);
  const byteLength = bytes.byteLength;
  const byteRemainder = byteLength % 3;
  const mainLength = byteLength - byteRemainder;

  let a, b, c, d;
  let chunk;

  // Main loop deals with bytes in chunks of 3
  for (let i = 0; i < mainLength; i = i + 3) {
    // Combine the three bytes into a single integer
    chunk = (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];

    // Use bitmasks to extract 6-bit segments from the triplet
    a = (chunk & 16515072) >> 18; // 16515072 = (2^6 - 1) << 18
    b = (chunk & 258048) >> 12; // 258048   = (2^6 - 1) << 12
    c = (chunk & 4032) >> 6; // 4032     = (2^6 - 1) << 6
    d = chunk & 63; // 63       = 2^6 - 1

    // Convert the raw binary segments to the appropriate ASCII encoding
    base64 += encodings[a] + encodings[b] + encodings[c] + encodings[d];
  }

  // Deal with the remaining bytes and padding
  if (byteRemainder == 1) {
    chunk = bytes[mainLength];

    a = (chunk & 252) >> 2; // 252 = (2^6 - 1) << 2

    // Set the 4 least significant bits to zero
    b = (chunk & 3) << 4; // 3   = 2^2 - 1

    base64 += encodings[a] + encodings[b] + "==";
  } else if (byteRemainder == 2) {
    chunk = (bytes[mainLength] << 8) | bytes[mainLength + 1];

    a = (chunk & 64512) >> 10; // 64512 = (2^6 - 1) << 10
    b = (chunk & 1008) >> 4; // 1008  = (2^6 - 1) << 4

    // Set the 2 least significant bits to zero
    c = (chunk & 15) << 2; // 15    = 2^4 - 1

    base64 += encodings[a] + encodings[b] + encodings[c] + "=";
  }

  return base64;
}

/**
 * Get email from workspace username
 * This method is particularly useful for apps that require the email address of the viewer.
 * Indeed, in the viewer context, WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
 * @param username
 * @returns email address
 */
export async function usernameToEmail(username: string): Promise<string> {
  const workspace = getWorkspace();
  return await UserService.usernameToEmail({ username, workspace });
}

interface SlackApprovalOptions {
  slackResourcePath: string;
  channelId: string;
  message?: string;
  approver?: string;
  defaultArgsJson?: Record<string, any>;
  dynamicEnumsJson?: Record<string, any>;
  resumeButtonText?: string;
  cancelButtonText?: string;
}

interface TeamsApprovalOptions {
  teamName: string;
  channelName: string;
  message?: string;
  approver?: string;
  defaultArgsJson?: Record<string, any>;
  dynamicEnumsJson?: Record<string, any>;
}

/**
 * Sends an interactive approval request via Slack, allowing optional customization of the message, approver, and form fields.
 *
 * **[Enterprise Edition Only]** To include form fields in the Slack approval request, go to **Advanced -> Suspend -> Form**
 * and define a form. Learn more at [Windmill Documentation](https://www.windmill.dev/docs/flows/flow_approval#form).
 *
 * @param {Object} options - The configuration options for the Slack approval request.
 * @param {string} options.slackResourcePath - The path to the Slack resource in Windmill.
 * @param {string} options.channelId - The Slack channel ID where the approval request will be sent.
 * @param {string} [options.message] - Optional custom message to include in the Slack approval request.
 * @param {string} [options.approver] - Optional user ID or name of the approver for the request.
 * @param {DefaultArgs} [options.defaultArgsJson] - Optional object defining or overriding the default arguments to a form field.
 * @param {Enums} [options.dynamicEnumsJson] - Optional object overriding the enum default values of an enum form field.
 * @param {string} [options.resumeButtonText] - Optional text for the resume button.
 * @param {string} [options.cancelButtonText] - Optional text for the cancel button.
 *
 * @returns {Promise<void>} Resolves when the Slack approval request is successfully sent.
 *
 * @throws {Error} If the function is not called within a flow or flow preview.
 * @throws {Error} If the `JobService.getSlackApprovalPayload` call fails.
 *
 * **Usage Example:**
 * ```typescript
 * await requestInteractiveSlackApproval({
 *   slackResourcePath: "/u/alex/my_slack_resource",
 *   channelId: "admins-slack-channel",
 *   message: "Please approve this request",
 *   approver: "approver123",
 *   defaultArgsJson: { key1: "value1", key2: 42 },
 *   dynamicEnumsJson: { foo: ["choice1", "choice2"], bar: ["optionA", "optionB"] },
 *   resumeButtonText: "Resume",
 *   cancelButtonText: "Cancel",
 * });
 * ```
 *
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
export async function requestInteractiveSlackApproval({
  slackResourcePath,
  channelId,
  message,
  approver,
  defaultArgsJson,
  dynamicEnumsJson,
  resumeButtonText,
  cancelButtonText,
}: SlackApprovalOptions): Promise<void> {
  const workspace = getWorkspace();
  const flowJobId = getEnv("WM_FLOW_JOB_ID");

  if (!flowJobId) {
    throw new Error(
      "You can't use this function in a standalone script or flow step preview. Please use it in a flow or a flow preview."
    );
  }

  const flowStepId = getEnv("WM_FLOW_STEP_ID");
  if (!flowStepId) {
    throw new Error("This function can only be called as a flow step");
  }

  // Only include non-empty parameters
  const params: {
    approver?: string;
    message?: string;
    slackResourcePath: string;
    channelId: string;
    flowStepId: string;
    defaultArgsJson?: string;
    dynamicEnumsJson?: string;
    resumeButtonText?: string;
    cancelButtonText?: string;
  } = {
    slackResourcePath,
    channelId,
    flowStepId,
  };

  if (message) {
    params.message = message;
  }
  if (approver) {
    params.approver = approver;
  }

  if (defaultArgsJson) {
    params.defaultArgsJson = JSON.stringify(defaultArgsJson);
  }

  if (dynamicEnumsJson) {
    params.dynamicEnumsJson = JSON.stringify(dynamicEnumsJson);
  }

  if (resumeButtonText) {
    params.resumeButtonText = resumeButtonText;
  }
  if (cancelButtonText) {
    params.cancelButtonText = cancelButtonText;
  }

  await JobService.getSlackApprovalPayload({
    workspace,
    ...params,
    id: getEnv("WM_JOB_ID") ?? "NO_JOB_ID",
  });
}

/**
 * Sends an interactive approval request via Teams, allowing optional customization of the message, approver, and form fields.
 *
 * **[Enterprise Edition Only]** To include form fields in the Teams approval request, go to **Advanced -> Suspend -> Form**
 * and define a form. Learn more at [Windmill Documentation](https://www.windmill.dev/docs/flows/flow_approval#form).
 *
 * @param {Object} options - The configuration options for the Teams approval request.
 * @param {string} options.teamName - The Teams team name where the approval request will be sent.
 * @param {string} options.channelName - The Teams channel name where the approval request will be sent.
 * @param {string} [options.message] - Optional custom message to include in the Teams approval request.
 * @param {string} [options.approver] - Optional user ID or name of the approver for the request.
 * @param {DefaultArgs} [options.defaultArgsJson] - Optional object defining or overriding the default arguments to a form field.
 * @param {Enums} [options.dynamicEnumsJson] - Optional object overriding the enum default values of an enum form field.
 *
 * @returns {Promise<void>} Resolves when the Teams approval request is successfully sent.
 *
 * @throws {Error} If the function is not called within a flow or flow preview.
 * @throws {Error} If the `JobService.getTeamsApprovalPayload` call fails.
 *
 * **Usage Example:**
 * ```typescript
 * await requestInteractiveTeamsApproval({
 *   teamName: "admins-teams",
 *   channelName: "admins-teams-channel",
 *   message: "Please approve this request",
 *   approver: "approver123",
 *   defaultArgsJson: { key1: "value1", key2: 42 },
 *   dynamicEnumsJson: { foo: ["choice1", "choice2"], bar: ["optionA", "optionB"] },
 * });
 * ```
 *
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
export async function requestInteractiveTeamsApproval({
  teamName,
  channelName,
  message,
  approver,
  defaultArgsJson,
  dynamicEnumsJson,
}: TeamsApprovalOptions): Promise<void> {
  const workspace = getWorkspace();
  const flowJobId = getEnv("WM_FLOW_JOB_ID");

  if (!flowJobId) {
    throw new Error(
      "You can't use this function in a standalone script or flow step preview. Please use it in a flow or a flow preview."
    );
  }

  const flowStepId = getEnv("WM_FLOW_STEP_ID");
  if (!flowStepId) {
    throw new Error("This function can only be called as a flow step");
  }

  // Only include non-empty parameters
  const params: {
    approver?: string;
    message?: string;
    teamName: string;
    channelName: string;
    flowStepId: string;
    defaultArgsJson?: string;
    dynamicEnumsJson?: string;
  } = {
    teamName,
    channelName,
    flowStepId,
  };

  if (message) {
    params.message = message;
  }
  if (approver) {
    params.approver = approver;
  }

  if (defaultArgsJson) {
    params.defaultArgsJson = JSON.stringify(defaultArgsJson);
  }

  if (dynamicEnumsJson) {
    params.dynamicEnumsJson = JSON.stringify(dynamicEnumsJson);
  }

  await JobService.getTeamsApprovalPayload({
    workspace,
    ...params,
    id: getEnv("WM_JOB_ID") ?? "NO_JOB_ID",
  });
}

async function getMockedApi(): Promise<MockedApi | undefined> {
  if (mockedApi) {
    return mockedApi;
  }

  const mockedPath = getEnv("WM_MOCKED_API_FILE");

  if (mockedPath) {
    console.info("Using mocked API from", mockedPath);
  } else {
    return undefined;
  }

  try {
    const fs = await import("node:fs/promises");
    const file = await fs.readFile(mockedPath, "utf-8");
    try {
      mockedApi = JSON.parse(file) as MockedApi;
      if (!mockedApi.variables) {
        mockedApi.variables = {};
      }
      if (!mockedApi.resources) {
        mockedApi.resources = {};
      }
      return mockedApi;
    } catch {
      console.warn("Error parsing mocked API file at path", mockedPath);
    }
  } catch {
    console.warn("Error reading mocked API file at path", mockedPath);
  }
  if (!mockedApi) {
    console.warn(
      "No mocked API file path provided at env variable WM_MOCKED_API_FILE. Using empty mocked API."
    );
    mockedApi = {
      variables: {},
      resources: {},
    };
    return mockedApi;
  }
}

interface MockedApi {
  variables: Record<string, string>;
  resources: Record<string, any>;
}

function parseResourceSyntax(s: string | undefined) {
  if (s?.startsWith("$res:")) return s.substring(5);
  if (s?.startsWith("res://")) return s.substring(6);
}

/**
 * Parse an S3 object from URI string or record format
 * @param s3Object - S3 object as URI string (s3://storage/key) or record
 * @returns S3 object record with storage and s3 key
 */
export function parseS3Object(s3Object: S3Object): S3ObjectRecord {
  if (typeof s3Object === "object") return s3Object;
  const match = s3Object.match(/^s3:\/\/([^/]*)\/(.*)$/);
  return { storage: match?.[1] || undefined, s3: match?.[2] ?? "" };
}

function parseVariableSyntax(s: string) {
  if (s.startsWith("var://")) return s.substring(6);
}
