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
export {
  type S3Object,
  type S3ObjectRecord,
  type S3ObjectURI,
} from "./s3Types";
export { datatable, ducklake, type SqlTemplateFunction } from "./sqlUtils";
export type Sql = string;
export type Email = string;
export type Base64 = string;
export type Resource<S extends string> = any;
export declare const SHARED_FOLDER = "/shared";
export declare function setClient(token?: string, baseUrl?: string): void;
/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
export declare function getWorkspace(): string;
/**
 * Get a resource value by path
 * @param path path of the resource,  default to internal state path
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
export declare function getResource(
  path?: string,
  undefinedIfEmpty?: boolean
): Promise<any>;
/**
 * Get a resource value by path
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
export declare function getRootJobId(jobId?: string): Promise<string>;
export declare function runScript(
  path?: string | null,
  hash_?: string | null,
  args?: Record<string, any> | null,
  verbose?: boolean
): Promise<any>;
export declare function waitJob(jobId: string, verbose?: boolean): Promise<any>;
export declare function getResult(jobId: string): Promise<any>;
export declare function getResultMaybe(jobId: string): Promise<any>;
export declare function task<P, T>(f: (_: P) => T): (_: P) => Promise<T>;
export declare function runScriptAsync(
  path: string | null,
  hash_: string | null,
  args: Record<string, any> | null,
  scheduledInSeconds?: number | null
): Promise<string>;
/**
 * Resolve a resource value in case the default value was picked because the input payload was undefined
 * @param obj resource value or path of the resource under the format `$res:path`
 * @returns resource value
 */
export declare function resolveDefaultResource(obj: any): Promise<any>;
export declare function getStatePath(): string;
/**
 * Set a resource value by path
 * @param path path of the resource to set, default to state path
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
export declare function setResource(
  value: any,
  path?: string,
  initializeToTypeIfNotExist?: string
): Promise<void>;
/**
 * Set the state
 * @param state state to set
 * @deprecated use setState instead
 */
export declare function setInternalState(state: any): Promise<void>;
/**
 * Set the state
 * @param state state to set
 */
export declare function setState(state: any): Promise<void>;
/**
 * Set a flow user state
 * @param key key of the state
 * @param value value of the state

 */
export declare function setFlowUserState(
  key: string,
  value: any,
  errorIfNotPossible?: boolean
): Promise<void>;
/**
 * Get a flow user state
 * @param path path of the variable

 */
export declare function getFlowUserState(
  key: string,
  errorIfNotPossible?: boolean
): Promise<any>;
/**
 * Get the internal state
 * @deprecated use getState instead
 */
export declare function getInternalState(): Promise<any>;
/**
 * Get the state shared across executions
 */
export declare function getState(): Promise<any>;
/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
export declare function getVariable(path: string): Promise<string>;
/**
 * Set a variable by path, create if not exist
 * @param path path of the variable
 * @param value value of the variable
 * @param isSecretIfNotExist if the variable does not exist, create it as secret or not (default: false)
 * @param descriptionIfNotExist if the variable does not exist, create it with this description (default: "")
 */
export declare function setVariable(
  path: string,
  value: string,
  isSecretIfNotExist?: boolean,
  descriptionIfNotExist?: string
): Promise<void>;
export declare function databaseUrlFromResource(path: string): Promise<string>;
export declare function denoS3LightClientSettings(
  s3_resource_path: string | undefined
): Promise<DenoS3LightClientSettings>;
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
export declare function loadS3File(
  s3object: S3Object,
  s3ResourcePath?: string | undefined
): Promise<Uint8Array | undefined>;
/**
 * Load the content of a file stored in S3 as a stream. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * let fileContentBlob = await wmill.loadS3FileStream(inputFile)
 * // if the content is plain text, the blob can be read directly:
 * console.log(await fileContentBlob.text());
 * ```
 */
export declare function loadS3FileStream(
  s3object: S3Object,
  s3ResourcePath?: string | undefined
): Promise<Blob | undefined>;
/**
 * Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 *
 * ```typescript
 * const s3object = await writeS3File(s3Object, "Hello Windmill!")
 * const fileContentAsUtf8Str = (await s3object.toArray()).toString('utf-8')
 * console.log(fileContentAsUtf8Str)
 * ```
 */
export declare function writeS3File(
  s3object: S3Object | undefined,
  fileContent: string | Blob,
  s3ResourcePath?: string | undefined
): Promise<S3Object>;

/**
 * Sign S3 objects to be used by anonymous users in public apps
 * @param s3objects s3 objects to sign
 * @returns signed s3 objects
 */
export declare function signS3Objects(
  s3objects: S3Object[]
): Promise<S3Object[]>;
/**
 * Sign S3 object to be used by anonymous users in public apps
 * @param s3object s3 object to sign
 * @returns signed s3 object
 */
export declare function signS3Object(s3object: S3Object): Promise<S3Object>;

/**
 * Generate a presigned public URL for an array of S3 objects.
 * If an S3 object is not signed yet, it will be signed first.
 * @param s3Objects s3 objects to sign
 * @returns list of signed public URLs
 */
export declare function getPresignedS3PublicUrls(
  s3Objects: S3Object[],
  { baseUrl }: { baseUrl?: string }
): Promise<string[]>;

/**
 * Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
 * @param s3Object s3 object to sign
 * @returns signed public URL
 */
export declare function getPresignedS3PublicUrl(
  s3Objects: S3Object,
  { baseUrl }: { baseUrl?: string }
): Promise<string>;

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
export declare function getResumeUrls(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>;
/**
 * @deprecated use getResumeUrls instead
 */
export declare function getResumeEndpoints(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>;
/**
 * Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
 * @param audience audience of the token
 * @returns jwt token
 */
export declare function getIdToken(audience: string): Promise<string>;
export declare function base64ToUint8Array(data: string): Uint8Array;
export declare function uint8ArrayToBase64(arrayBuffer: Uint8Array): string;
/**
 * Get email from workspace username
 * This method is particularly useful for apps that require the email address of the viewer.
 * Indeed, in the viewer context, WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
 * @param username
 * @returns email address
 */
export declare function usernameToEmail(username: string): Promise<string>;
