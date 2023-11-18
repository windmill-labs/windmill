import {
  JobService,
  ResourceService,
  VariableService,
} from "./windmill-api/index.ts";
import { OpenAPI } from "./windmill-api/index.ts";

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
} from "./windmill-api/index.ts";

export type Sql = string;
export type Email = string;
export type Base64 = string;
export type Resource<S extends string> = any;

export const SHARED_FOLDER = "/shared";

export function setClient(token: string, baseUrl: string) {
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
}

setClient(
  Deno.env.get("WM_TOKEN") ?? "no_token",
  Deno.env.get("BASE_INTERNAL_URL") ??
    Deno.env.get("BASE_URL") ??
    "http://localhost:8000"
);

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
export function getWorkspace(): string {
  return Deno.env.get("WM_WORKSPACE") ?? "no_workspace";
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
      throw Error(`Resource not found at ${path} or not visible to you`);
    }
  }
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
  const state_path = Deno.env.get("WM_STATE_PATH_NEW") ?? Deno.env.get("WM_STATE_PATH");
  if (state_path === undefined) {
    throw Error("State path not set");
  }
  return state_path;
}

/**
 * Set a resource value by path
 * @param value new value of the resource to set
 * @param path path of the resource to set, default to state path
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
export async function setResource(
  value: any,
  path?: string,
  initializeToTypeIfNotExist?: string
): Promise<void> {
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

/**
 * Set the shared state
 * @param state state to set
 */
export async function setSharedState(
  state: any,
  path = "state.json"
): Promise<void> {
  await Deno.writeTextFile(SHARED_FOLDER + "/" + path, JSON.stringify(state));
}

/**
 * Get the shared state
 * @param state state to set
 */
export async function getSharedState(path = "state.json"): Promise<any> {
  return JSON.parse(await Deno.readTextFile(SHARED_FOLDER + "/" + path));
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
 */
export async function getState(): Promise<any> {
  return await getResource(getStatePath(), true);
}

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
export async function getVariable(path: string): Promise<string | undefined> {
  const workspace = getWorkspace();
  try {
    return await VariableService.getVariableValue({ workspace, path });
  } catch (e: any) {
    throw Error(`Variable not found at ${path} or not visible to you`);
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

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for resumeing the flow
 */
export async function getResumeUrls(approver?: string): Promise<{
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
    id: Deno.env.get("WM_JOB_ID") ?? "NO_JOB_ID",
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
