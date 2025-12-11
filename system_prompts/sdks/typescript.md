# TypeScript SDK (windmill-client)

Import: `import * as wmill from 'windmill-client'`

## Functions

### Scripts & Flows

```typescript
async function runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
```

```typescript
async function runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
```

```typescript
async function runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
```

```typescript
async function runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>
```

```typescript
async function waitJob(jobId: string, verbose: boolean = false): Promise<any>
```

```typescript
async function getResult(jobId: string): Promise<any>
```

```typescript
async function getResultMaybe(jobId: string): Promise<any>
```

```typescript
async function runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string>
```

```typescript
async function runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
```

```typescript
async function runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>
```

```typescript
async function runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>
```

### Variables

```typescript
async function getVariable(path: string): Promise<string>
```

```typescript
async function setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void>
```

### Resources

```typescript
async function getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any>
```

```typescript
async function setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void>
```

### State

```typescript
async function setState(state: any): Promise<void>
```

```typescript
async function setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void>
```

```typescript
async function getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any>
```

```typescript
async function getState(): Promise<any>
```

```typescript
function getStatePath(): string
```

### S3 Operations

```typescript
async function loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>
```

```typescript
async function loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>
```

```typescript
async function writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object>
```

### Progress & Logging

```typescript
async function setProgress(percent: number, jobId?: any): Promise<void>
```

```typescript
async function getProgress(jobId?: any): Promise<number | null>
```

### Approval & Resume

```typescript
async function getResumeUrls(approver?: string): Promise<
```

```typescript
function getResumeEndpoints(approver?: string): Promise<
```

### Utilities

```typescript
async function getRootJobId(jobId?: string): Promise<string>
```

```typescript
function getWorkspace(): string
```

### Other

```typescript
async function streamResult(stream: AsyncIterable<string>): Promise<void>
```

```typescript
async function resolveDefaultResource(obj: any): Promise<any>
```

```typescript
async function setInternalState(state: any): Promise<void>
```

```typescript
async function setSharedState(// state: any, // path = "state.json" //): Promise<void>
```

```typescript
async function getSharedState(path = "state.json"): Promise<any>
```

```typescript
async function getInternalState(): Promise<any>
```

```typescript
async function databaseUrlFromResource(path: string): Promise<string>
```

```typescript
async function polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>
```

```typescript
async function duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>
```

```typescript
async function denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>
```

```typescript
async function signS3Objects(s3objects: S3Object[]): Promise<S3Object[]>
```

```typescript
async function signS3Object(s3object: S3Object): Promise<S3Object>
```

```typescript
async function getIdToken(audience: string, expiresIn?: number): Promise<string>
```

```typescript
async function usernameToEmail(username: string): Promise<string>
```

```typescript
async function requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>
```

```typescript
async function requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>
```

```typescript
function setClient(token?: string, baseUrl?: string): void
```

```typescript
function appendToResultStream(text: string): void
```

```typescript
function base64ToUint8Array(data: string): Uint8Array
```

```typescript
function uint8ArrayToBase64(arrayBuffer: Uint8Array): string
```

```typescript
function parseS3Object(s3Object: S3Object): S3ObjectRecord
```

## Types

```typescript
type Sql = string
```

```typescript
type Email = string
```

```typescript
type Base64 = string
```

