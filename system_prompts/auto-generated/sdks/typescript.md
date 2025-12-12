# TypeScript SDK (windmill-client)

Import: import * as wmill from 'windmill-client'

/**
 * Initialize the Windmill client with authentication token and base URL
 * @param token - Authentication token (defaults to WM_TOKEN env variable)
 * @param baseUrl - API base URL (defaults to BASE_INTERNAL_URL or BASE_URL env variable)
 */
setClient(token?: string, baseUrl?: string): void

/**
 * Create a client configuration from env variables
 * @returns client configuration
 */
getWorkspace(): string

/**
 * Get a resource value by path
 * @param path path of the resource,  default to internal state path
 * @param undefinedIfEmpty if the resource does not exist, return undefined instead of throwing an error
 * @returns resource value
 */
async getResource(path?: string, undefinedIfEmpty?: boolean): Promise<any>

/**
 * Get the true root job id
 * @param jobId job id to get the root job id from (default to current job)
 * @returns root job id
 */
async getRootJobId(jobId?: string): Promise<string>

/**
 * @deprecated Use runScriptByPath or runScriptByHash instead
 */
async runScript(path: string | null = null, hash_: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Run a script synchronously by its path and wait for the result
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @returns Script execution result
 */
async runScriptByPath(path: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Run a script synchronously by its hash and wait for the result
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param verbose - Enable verbose logging
 * @returns Script execution result
 */
async runScriptByHash(hash_: string, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Append a text to the result stream
 * @param text text to append to the result stream
 */
appendToResultStream(text: string): void

/**
 * Stream to the result stream
 * @param stream stream to stream to the result stream
 */
async streamResult(stream: AsyncIterable<string>): Promise<void>

/**
 * Run a flow synchronously by its path and wait for the result
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param verbose - Enable verbose logging
 * @returns Flow execution result
 */
async runFlow(path: string | null = null, args: Record<string, any> | null = null, verbose: boolean = false): Promise<any>

/**
 * Wait for a job to complete and return its result
 * @param jobId - ID of the job to wait for
 * @param verbose - Enable verbose logging
 * @returns Job result when completed
 */
async waitJob(jobId: string, verbose: boolean = false): Promise<any>

/**
 * Get the result of a completed job
 * @param jobId - ID of the completed job
 * @returns Job result
 */
async getResult(jobId: string): Promise<any>

/**
 * Get the result of a job if completed, or its current status
 * @param jobId - ID of the job
 * @returns Object with started, completed, success, and result properties
 */
async getResultMaybe(jobId: string): Promise<any>

/**
 * Wrap a function to execute as a Windmill task within a flow context
 * @param f - Function to wrap as a task
 * @returns Async wrapper function that executes as a Windmill job
 */
task<P, T>(f: (_: P) => T): (_: P) => Promise<T>

/**
 * @deprecated Use runScriptByPathAsync or runScriptByHashAsync instead
 */
async runScriptAsync(path: string | null, hash_: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null): Promise<string>

/**
 * Run a script asynchronously by its path
 * @param path - Script path in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @returns Job ID of the created job
 */
async runScriptByPathAsync(path: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>

/**
 * Run a script asynchronously by its hash
 * @param hash_ - Script hash in Windmill
 * @param args - Arguments to pass to the script
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @returns Job ID of the created job
 */
async runScriptByHashAsync(hash_: string, args: Record<string, any> | null = null, scheduledInSeconds: number | null = null): Promise<string>

/**
 * Run a flow asynchronously by its path
 * @param path - Flow path in Windmill
 * @param args - Arguments to pass to the flow
 * @param scheduledInSeconds - Schedule execution for a future time (in seconds)
 * @param doNotTrackInParent - If false, tracks state in parent job (only use when fully awaiting the job)
 * @returns Job ID of the created job
 */
async runFlowAsync(path: string | null, args: Record<string, any> | null, scheduledInSeconds: number | null = null, // can only be set to false if this the job will be fully await and not concurrent with any other job // as otherwise the child flow and its own child will store their state in the parent job which will // lead to incorrectness and failures doNotTrackInParent: boolean = true): Promise<string>

/**
 * Resolve a resource value in case the default value was picked because the input payload was undefined
 * @param obj resource value or path of the resource under the format `$res:path`
 * @returns resource value
 */
async resolveDefaultResource(obj: any): Promise<any>

/**
 * Get the state file path from environment variables
 * @returns State path string
 */
getStatePath(): string

/**
 * Set a resource value by path
 * @param path path of the resource to set, default to state path
 * @param value new value of the resource to set
 * @param initializeToTypeIfNotExist if the resource does not exist, initialize it with this type
 */
async setResource(value: any, path?: string, initializeToTypeIfNotExist?: string): Promise<void>

/**
 * Set the state
 * @param state state to set
 * @deprecated use setState instead
 */
async setInternalState(state: any): Promise<void>

/**
 * Set the state
 * @param state state to set
 */
async setState(state: any): Promise<void>

/**
 * Set the progress
 * Progress cannot go back and limited to 0% to 99% range
 * @param percent Progress to set in %
 * @param jobId? Job to set progress for
 */
async setProgress(percent: number, jobId?: any): Promise<void>

/**
 * Get the progress
 * @param jobId? Job to get progress from
 * @returns Optional clamped between 0 and 100 progress value
 */
async getProgress(jobId?: any): Promise<number | null>

/**
 * Set a flow user state
 * @param key key of the state
 * @param value value of the state
 */
async setFlowUserState(key: string, value: any, errorIfNotPossible?: boolean): Promise<void>

/**
 * Get a flow user state
 * @param path path of the variable
 */
async getFlowUserState(key: string, errorIfNotPossible?: boolean): Promise<any>

/**
 * Get the internal state
 * @deprecated use getState instead
 */
async getInternalState(): Promise<any>

/**
 * Get the state shared across executions
 */
async getState(): Promise<any>

/**
 * Get a variable by path
 * @param path path of the variable
 * @returns variable value
 */
async getVariable(path: string): Promise<string>

/**
 * Set a variable by path, create if not exist
 * @param path path of the variable
 * @param value value of the variable
 * @param isSecretIfNotExist if the variable does not exist, create it as secret or not (default: false)
 * @param descriptionIfNotExist if the variable does not exist, create it with this description (default: "")
 */
async setVariable(path: string, value: string, isSecretIfNotExist?: boolean, descriptionIfNotExist?: string): Promise<void>

/**
 * Build a PostgreSQL connection URL from a database resource
 * @param path - Path to the database resource
 * @returns PostgreSQL connection URL string
 */
async databaseUrlFromResource(path: string): Promise<string>

/**
 * Get S3 client settings from a resource or workspace default
 * @param s3_resource_path - Path to S3 resource (uses workspace default if undefined)
 * @returns S3 client configuration settings
 */
async denoS3LightClientSettings(s3_resource_path: string | undefined): Promise<DenoS3LightClientSettings>

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
async loadS3File(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Uint8Array | undefined>

/**
 * Load the content of a file stored in S3 as a stream. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * let fileContentBlob = await wmill.loadS3FileStream(inputFile)
 * // if the content is plain text, the blob can be read directly:
 * console.log(await fileContentBlob.text());
 * ```
 */
async loadS3FileStream(s3object: S3Object, s3ResourcePath: string | undefined = undefined): Promise<Blob | undefined>

/**
 * Persist a file to the S3 bucket. If the s3ResourcePath is undefined, it will default to the workspace S3 resource.
 * 
 * ```typescript
 * const s3object = await writeS3File(s3Object, "Hello Windmill!")
 * const fileContentAsUtf8Str = (await s3object.toArray()).toString('utf-8')
 * console.log(fileContentAsUtf8Str)
 * ```
 */
async writeS3File(s3object: S3Object | undefined, fileContent: string | Blob, s3ResourcePath: string | undefined = undefined, contentType: string | undefined = undefined, contentDisposition: string | undefined = undefined): Promise<S3Object>

/**
 * Sign S3 objects to be used by anonymous users in public apps
 * @param s3objects s3 objects to sign
 * @returns signed s3 objects
 */
async signS3Objects(s3objects: S3Object[]): Promise<S3Object[]>

/**
 * Sign S3 object to be used by anonymous users in public apps
 * @param s3object s3 object to sign
 * @returns signed s3 object
 */
async signS3Object(s3object: S3Object): Promise<S3Object>

/**
 * Generate a presigned public URL for an array of S3 objects.
 * If an S3 object is not signed yet, it will be signed first.
 * @param s3Objects s3 objects to sign
 * @returns list of signed public URLs
 */
async getPresignedS3PublicUrls(s3Objects: S3Object[], { baseUrl }: { baseUrl?: string } = {}): Promise<string[]>

/**
 * Generate a presigned public URL for an S3 object. If the S3 object is not signed yet, it will be signed first.
 * @param s3Object s3 object to sign
 * @returns signed public URL
 */
async getPresignedS3PublicUrl(s3Objects: S3Object, { baseUrl }: { baseUrl?: string } = {}): Promise<string>

/**
 * Get URLs needed for resuming a flow after this step
 * @param approver approver name
 * @returns approval page UI URL, resume and cancel API URLs for resuming the flow
 */
async getResumeUrls(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>

/**
 * @deprecated use getResumeUrls instead
 */
getResumeEndpoints(approver?: string): Promise<{
  approvalPage: string;
  resume: string;
  cancel: string;
}>

/**
 * Get an OIDC jwt token for auth to external services (e.g: Vault, AWS) (ee only)
 * @param audience audience of the token
 * @param expiresIn Optional number of seconds until the token expires
 * @returns jwt token
 */
async getIdToken(audience: string, expiresIn?: number): Promise<string>

/**
 * Convert a base64-encoded string to Uint8Array
 * @param data - Base64-encoded string
 * @returns Decoded Uint8Array
 */
base64ToUint8Array(data: string): Uint8Array

/**
 * Convert a Uint8Array to base64-encoded string
 * @param arrayBuffer - Uint8Array to encode
 * @returns Base64-encoded string
 */
uint8ArrayToBase64(arrayBuffer: Uint8Array): string

/**
 * Get email from workspace username
 * This method is particularly useful for apps that require the email address of the viewer.
 * Indeed, in the viewer context, WM_USERNAME is set to the username of the viewer but WM_EMAIL is set to the email of the creator of the app.
 * @param username
 * @returns email address
 */
async usernameToEmail(username: string): Promise<string>

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
 * });
 * ```
 * 
 * **Note:** This function requires execution within a Windmill flow or flow preview.
 */
async requestInteractiveSlackApproval({ slackResourcePath, channelId, message, approver, defaultArgsJson, dynamicEnumsJson, }: SlackApprovalOptions): Promise<void>

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
async requestInteractiveTeamsApproval({ teamName, channelName, message, approver, defaultArgsJson, dynamicEnumsJson, }: TeamsApprovalOptions): Promise<void>

/**
 * Parse an S3 object from URI string or record format
 * @param s3Object - S3 object as URI string (s3://storage/key) or record
 * @returns S3 object record with storage and s3 key
 */
parseS3Object(s3Object: S3Object): S3ObjectRecord

/**
 * Create a SQL template function for PostgreSQL/datatable queries
 * @param name - Database/datatable name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.datatable()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}::int
 * `.fetch()
 */
datatable(name: string = "main"): SqlTemplateFunction

/**
 * Create a SQL template function for DuckDB/ducklake queries
 * @param name - DuckDB database name (default: "main")
 * @returns SQL template function for building parameterized queries
 * @example
 * let sql = wmill.ducklake()
 * let name = 'Robin'
 * let age = 21
 * await sql`
 *   SELECT * FROM friends
 *     WHERE name = ${name} AND age = ${age}
 * `.fetch()
 */
ducklake(name: string = "main"): SqlTemplateFunction

async polarsConnectionSettings(s3_resource_path: string | undefined): Promise<any>

async duckdbConnectionSettings(s3_resource_path: string | undefined): Promise<any>
