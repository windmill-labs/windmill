# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

getWorkspace(): string - Create a client configuration from env variables
async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any> - Get a resource value by path
async getRootJobId(jobId?: string): Promise<string> - Get the true root job id
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any> - @deprecated Use runScriptByPath or runScriptByHash instead
appendToResultStream(text: string): void - Append a text to the result stream
async streamResult(stream: AsyncIterable<string>): Promise<void> - Stream to the result stream
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string> - @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
async resolveDefaultResource(obj: any): Promise<any> - Resolve a resource value in case the default value was picked because the input payload was undefined
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void> - Set a resource value by path
async setInternalState(state: any): Promise<void> - Set the state
async setState(state: any): Promise<void> - Set the state
async setProgress(percent: number, jobId?: any): Promise<void> - Set the progress
async getProgress(jobId?: any): Promise<number | null> - Get the progress
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void> - Set a flow user state
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any> - Get a flow user state
async getInternalState(): Promise<any> - Get the internal state
async getState(): Promise<any> - Get the state shared across executions
async getVariable(path: string): Promise<string> - Get a variable by path
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void> - Set a variable by path, create if not exist
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object> - Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]> - Sign S3 objects to be used by anonymous users in public apps
async signS3Object(s3object: S3Object): Promise<S3Object> - Sign S3 object to be used by anonymous users in public apps
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]> - Generate a presigned public URL for an array of S3 objects.
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string> - Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
async getResumeUrls(approver?: string): Promise< - Get URLs needed for resuming a flow after this step
getResumeEndpoints(approver?: string): Promise< - @deprecated use getResumeUrls instead
async getIdToken(audience: string, expiresIn?: number): Promise<string> - Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
async usernameToEmail(username: string): Promise<string> - Get email from workspace username
setClient(token?: string, baseUrl?: string): void
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async waitJob(jobId: string, verbose: boolean = false): Promise<any>
async getResult(jobId: string): Promise<any>
async getResultMaybe(jobId: string): Promise<any>
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>
getStatePath(): string
async setSharedState(// state: any, // path = "state.json" //): Promise<void>
async getSharedState(path = "state.json"): Promise<any>
async databaseUrlFromResource(path: string): Promise<string>
async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>
async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>
async denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>
async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>
async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>
base64ToUint8Array(data: string): Uint8Array
uint8ArrayToBase64(arrayBuffer: Uint8Array): string
async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>
async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>
parseS3Object(s3Object: S3Object): S3ObjectRecord
