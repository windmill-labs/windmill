# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any>
async getRootJobId(jobId?: string): Promise<string>
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async streamResult(stream: AsyncIterable<string>): Promise<void>
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
async waitJob(jobId: string, verbose: boolean = false): Promise<any>
async getResult(jobId: string): Promise<any>
async getResultMaybe(jobId: string): Promise<any>
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string>
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>
async resolveDefaultResource(obj: any): Promise<any>
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void>
async setInternalState(state: any): Promise<void>
async setState(state: any): Promise<void>
async setProgress(percent: number, jobId?: any): Promise<void>
async getProgress(jobId?: any): Promise<number | null>
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void>
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any>
async setSharedState(// state: any, // path = "state.json" //): Promise<void>
async getSharedState(path = "state.json"): Promise<any>
async getInternalState(): Promise<any>
async getState(): Promise<any>
async getVariable(path: string): Promise<string>
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void>
async databaseUrlFromResource(path: string): Promise<string>
async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>
async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>
async denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>
async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>
async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object>
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]>
async signS3Object(s3object: S3Object): Promise<S3Object>
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]>
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string>
async getResumeUrls(approver?: string): Promise<
async getIdToken(audience: string, expiresIn?: number): Promise<string>
async usernameToEmail(username: string): Promise<string>
async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>
async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>
setClient(token?: string, baseUrl?: string): void
getWorkspace(): string
appendToResultStream(text: string): void
getStatePath(): string
getResumeEndpoints(approver?: string): Promise<
base64ToUint8Array(data: string): Uint8Array
uint8ArrayToBase64(arrayBuffer: Uint8Array): string
parseS3Object(s3Object: S3Object): S3ObjectRecord
