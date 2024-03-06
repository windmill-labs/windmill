import {
  ResourceService,
  VariableService,
  JobService,
  HelpersService,
  OidcService,
} from "./index";
import { OpenAPI } from "./index";
// import type { DenoS3LightClientSettings } from "./index";
import { DenoS3LightClientSettings, type S3Object } from "./s3Types";

export {
  AdminService,
  AuditService,
  FlowService,
  GranularAclService,
  GroupService,
  JobService,
  ResourceService,
  VariableService,
  ScriptService,
  ScheduleService,
  SettingsService,
  UserService,
  WorkspaceService,
} from "./index";

export type Sql = string;
export type Email = string;
export type Base64 = string;
export type Resource<S extends string> = any;

export const SHARED_FOLDER = "/shared";

let clientSet = false;
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
  clientSet = true;
}

const getEnv = (key: string) => {
  if (typeof window === "undefined") {
    // node
    return process.env[key];
  }
  // browser
  return window.process.env[key];
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
  !clientSet && setClient();
  const workspace = getWorkspace();
  path = path ?? getStatePath();
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
 * Get a resource value by path
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
export async function getRootJobId(jobId?: string): Promise<string> {
  !clientSet && setClient();
  const workspace = getWorkspace();
  jobId = jobId ?? getEnv("WM_JOB_ID");
  if (jobId === undefined) {
    throw Error("Job ID not set");
  }
  return await JobService.getRootJobId({ workspace, id: jobId });
}

export async function runScript(
  path: string | null = null,
  hash_: string | null = null,
  args: Record<string, any> | null = null,
  verbose: boolean = false
): Promise<any> {
  args = args || {};

  if (verbose) {
    console.info(`running \`${path}\` synchronously with args:`, args);
  }

  const jobId = await runScriptAsync(path, hash_, args);
  return await waitJob(jobId, verbose);
}

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

export async function getResult(jobId: string): Promise<any> {
  !clientSet && setClient();
  const workspace = getWorkspace();
  return await JobService.getCompletedJobResult({ workspace, id: jobId });
}

export async function getResultMaybe(jobId: string): Promise<any> {
  !clientSet && setClient();
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

export function task<P, T>(f: (_: P) => T): (_: P) => Promise<T> {
  !clientSet && setClient();

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

export async function runScriptAsync(
  path: string | null,
  hash_: string | null,
  args: Record<string, any> | null,
  scheduledInSeconds: number | null = null
): Promise<string> {
  !clientSet && setClient();

  // Create a script job and return its job id.
  if (path && hash_) {
    throw new Error("path and hash_ are mutually exclusive");
  }
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
  if (rootJobId != undefined) {
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
  })
    .then((res) => res.json())
    .then((data) => data.id);
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
  !clientSet && setClient();
  path = path ?? getStatePath();
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
 */
export async function setState(state: any): Promise<void> {
  await setResource(state, undefined, "state");
}

// /**
//  * Set the shared state
//  * @param state state to set
//  */
// export async function setSharedState(
//   state: any,
//   path = "state.json"
// ): Promise<void> {
//   await Deno.writeTextFile(SHARED_FOLDER + "/" + path, JSON.stringify(state));
// }

// /**
//  * Get the shared state
//  * @param state state to set
//  */
// export async function getSharedState(path = "state.json"): Promise<any> {
//   return JSON.parse(await Deno.readTextFile(SHARED_FOLDER + "/" + path));
// }

/**
 * Get the internal state
 * @deprecated use getState instead
 */
export async function getInternalState(): Promise<any> {
  return await getResource(getStatePath(), true);
}

/**
 * Get the state shared across executions
 */
export async function getState(): Promise<any> {
  return await getResource(getStatePath(), true);
}

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
export async function getVariable(path: string): Promise<string> {
  !clientSet && setClient();
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
  !clientSet && setClient();
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

export async function denoS3LightClientSettings(
  s3_resource_path: string | undefined
): Promise<DenoS3LightClientSettings> {
  !clientSet && setClient();
  const workspace = getWorkspace();
  const s3Resource = await HelpersService.s3ResourceInfo({
    workspace: workspace,
    requestBody: {
      s3_resource_path: s3_resource_path,
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
  !clientSet && setClient();
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
  !clientSet && setClient();

  let params: Record<string, string> = {};
  params["file_key"] = s3object.s3;
  if (s3ResourcePath !== undefined) {
    params["s3_resource_path"] = s3ResourcePath;
  }
  const queryParams = new URLSearchParams(params);

  // We use raw fetch here b/c OpenAPI generated client doesn't handle Blobs nicely
  const fileContentBlob = await fetch(
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
  return fileContentBlob.blob();
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
  s3ResourcePath: string | undefined = undefined
): Promise<S3Object> {
  !clientSet && setClient();
  let fileContentBlob: Blob;
  if (typeof fileContent === "string") {
    fileContentBlob = new Blob([fileContent as string], {
      type: "text/plain",
    });
  } else {
    fileContentBlob = fileContent as Blob;
  }

  const response = await HelpersService.fileUpload({
    workspace: getWorkspace(),
    fileKey: s3object?.s3,
    fileExtension: undefined,
    s3ResourcePath: s3ResourcePath,
    requestBody: fileContentBlob,
  });
  return {
    s3: response.file_key,
  };
}

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
export async function getResumeUrls(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}> {
  const nonce = Math.floor(Math.random() * 4294967295);
  !clientSet && setClient();
  const workspace = getWorkspace();
  return await JobService.getResumeUrls({
    workspace,
    resumeId: nonce,
    approver,
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
 * @returns jwt token
 */
export async function getIdToken(audience: string): Promise<string> {
  !clientSet && setClient();
  const workspace = getWorkspace();
  return await OidcService.getOidcToken({
    workspace,
    audience,
  });
}

export function base64ToUint8Array(data: string): Uint8Array {
  return Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
}

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
