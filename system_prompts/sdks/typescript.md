# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

// Initialize the Windmill client with authentication token and base URL
setClient(token?: string, baseUrl?: string): void

// Create a client configuration from env variables
getWorkspace(): string

// Get a resource value by path
async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any>

// Get the true root job id
async getRootJobId(jobId?: string): Promise<string>

// @deprecated Use runScriptByPath or runScriptByHash instead
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

// Run a script synchronously by its path and wait for the result
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

// Run a script synchronously by its hash and wait for the result
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

// Append a text to the result stream
appendToResultStream(text: string): void

// Stream to the result stream
async streamResult(stream: AsyncIterable<string>): Promise<void>

// Run a flow synchronously by its path and wait for the result
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

// Wait for a job to complete and return its result
async waitJob(jobId: string, verbose: boolean = false): Promise<any>

// Get the result of a completed job
async getResult(jobId: string): Promise<any>

// Get the result of a job if completed, or its current status
async getResultMaybe(jobId: string): Promise<any>

// @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string>

// Run a script asynchronously by its path
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>

// Run a script asynchronously by its hash
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>

// Run a flow asynchronously by its path
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>

// Resolve a resource value in case the default value was picked because the input payload was undefined
async resolveDefaultResource(obj: any): Promise<any>

// Get the state file path from environment variables
getStatePath(): string

// Set a resource value by path
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void>

// Set the state
async setInternalState(state: any): Promise<void>

// Set the state
async setState(state: any): Promise<void>

// Set the progress
async setProgress(percent: number, jobId?: any): Promise<void>

// Get the progress
async getProgress(jobId?: any): Promise<number | null>

// Set a flow user state
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void>

// Get a flow user state
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any>

// Get the internal state
async getInternalState(): Promise<any>

// Get the state shared across executions
async getState(): Promise<any>

// Get a variable by path
async getVariable(path: string): Promise<string>

// Set a variable by path, create if not exist
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void>

// Build a PostgreSQL connection URL from a database resource
async databaseUrlFromResource(path: string): Promise<string>

// Get S3 client settings from a resource or workspace default
async denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>

// Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object>

// Sign S3 objects to be used by anonymous users in public apps
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]>

// Sign S3 object to be used by anonymous users in public apps
async signS3Object(s3object: S3Object): Promise<S3Object>

// Generate a presigned public URL for an array of S3 objects.
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]>

// Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string>

// Get URLs needed for resuming a flow after this step
async getResumeUrls(approver?: string): Promise<

// @deprecated use getResumeUrls instead
getResumeEndpoints(approver?: string): Promise<

// Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
async getIdToken(audience: string, expiresIn?: number): Promise<string>

// Convert a base64-encoded string to Uint8Array
base64ToUint8Array(data: string): Uint8Array

// Convert a Uint8Array to base64-encoded string
uint8ArrayToBase64(arrayBuffer: Uint8Array): string

// Get email from workspace username
async usernameToEmail(username: string): Promise<string>

// Create a SQL template function for DuckDB/ducklake queries
ducklake(name: string = "main"): SqlTemplateFunction

async setSharedState(// state: any, // path = "state.json" //): Promise<void>

async getSharedState(path = "state.json"): Promise<any>

async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>

async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>

async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>

async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>

async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>

async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>

parseS3Object(s3Object: S3Object): S3ObjectRecord

datatable(name: string = "main"): SqlTemplateFunction
