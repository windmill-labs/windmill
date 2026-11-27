import {
	type CompletedJob,
	JobService,
	type Preview,
	ResourceService,
	SettingService,
	UserService,
	VariableService
} from '$lib/gen'
import { aiResourceProvider } from './copilot/aiResourceProvider'

/**
 * Whether a resource value actually reaches what it describes, for every resource type Windmill
 * knows how to check. The resource editor's Test connection button and the AI chat's secret card
 * both call this, so a type gains a check in one place and both get it.
 */

export type ConnectionTestResult = { ok: true } | { ok: false; error: string }

// Object-storage resource types share one probe, the API's own connectivity test, which runs
// on the API server with the caller's privileges. Each type maps its resource to the
// ObjectSettings body that route expects.
const objectStorageBody: { [key: string]: (args: any) => Record<string, any> } = {
	s3: (s3) => ({
		type: 'S3',
		region: s3.region,
		bucket: s3.bucket,
		endpoint: s3.endPoint,
		port: s3.port,
		allow_http: !s3.useSSL,
		access_key: s3.accessKey,
		secret_key: s3.secretKey,
		path_style: s3.pathStyle
	}),
	azure_blob: (s3) => ({ type: 'Azure', ...s3 }),
	s3_bucket: (bucket) => bucket
}

const OBJECT_STORAGE_TEST_SCRIPT = `
export async function main(bucket: any, api_token: string) {
	const res = await fetch(process.env.BASE_URL + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + api_token,
		},
		body: JSON.stringify(bucket),
	})
	if (!res.ok) {
		throw new Error(await res.text())
	}
	return await res.text()
}
`

type ConnectionTestScript = {
	code: string
	lang: string
	argName: string
	/** Where the test executes, when that changes what it proves. */
	tooltip?: string
	additionalCheck?: (testResult: CompletedJob) => CompletedJob
}

const scripts: { [resourceType: string]: ConnectionTestScript } = {
	postgresql: { code: `SELECT 1`, lang: 'postgresql', argName: 'database' },
	mysql: { code: `SELECT 1`, lang: 'mysql', argName: 'database' },
	bigquery: { code: `select 1`, lang: 'bigquery', argName: 'database' },
	snowflake: { code: `select 1`, lang: 'snowflake', argName: 'database' },
	snowflake_oauth: { code: `select 1`, lang: 'snowflake', argName: 'database' },
	mssql: { code: `SELECT 1`, lang: 'mssql', argName: 'database' },
	oracledb: { code: `SELECT 1 FROM DUAL`, lang: 'oracledb', argName: 'database' },
	s3: {
		code: OBJECT_STORAGE_TEST_SCRIPT,
		lang: 'bun',
		argName: 'bucket',
		tooltip:
			'The storage operations of this test run on the Windmill server (the API process) with your permissions, not on the worker. Non-super-admins can only test public endpoints with an explicit access key and secret key; super admins can also test private endpoints and rely on the ambient AWS credentials of the server (environment variables, instance role). Scripts using this resource directly through an S3 SDK resolve credentials on the worker instead, so results may differ.'
	},
	azure_blob: {
		code: OBJECT_STORAGE_TEST_SCRIPT,
		lang: 'bun',
		argName: 'bucket',
		tooltip:
			'The storage operations of this test run on the Windmill server (the API process) with your permissions, not on the worker. Non-super-admins can only test public endpoints with an explicit access key.'
	},
	graphql: {
		code: '{ __typename }',
		lang: 'graphql',
		argName: 'api',
		additionalCheck: (testResult: CompletedJob) =>
			testResult.success &&
			(typeof testResult.result !== 'object' || !('__typename' in (testResult.result ?? {})))
				? {
						...testResult,
						result: { error: { message: 'Invalid GraphQL API response' } },
						success: false
					}
				: testResult
	},
	s3_bucket: {
		code: OBJECT_STORAGE_TEST_SCRIPT,
		lang: 'bun',
		argName: 'bucket',
		tooltip:
			"The storage operations of this test run on the Windmill server (the API process) with your permissions. Non-super-admins can only test public endpoints with explicit credentials; super admins can also test private endpoints and rely on the server's ambient credentials for the configured provider (environment variables, instance role)."
	}
}

export function canTestConnection(resourceType: string | undefined): boolean {
	return !!resourceType && Object.hasOwn(scripts, resourceType)
}

export function connectionTestTooltip(resourceType: string): string | undefined {
	return scripts[resourceType]?.tooltip
}

// The token is revoked as soon as the job settles; the expiry only covers a browser that
// goes away mid-test. It is readable in the job's stored args until then, hence the short life.
const API_TOKEN_TTL_MS = 60_000
// Tokens are addressed by their first 10 characters (TOKEN_PREFIX_LEN on the backend).
const API_TOKEN_PREFIX_LEN = 10

async function mintApiToken(resourceType: string): Promise<string> {
	return await UserService.createToken({
		requestBody: {
			label: `ephemeral-test-connection: ${resourceType}`,
			expiration: new Date(Date.now() + API_TOKEN_TTL_MS).toISOString(),
			scopes: ['settings:write'],
			// The test endpoint lets a superadmin reach private endpoints; the scope bounds the rest.
			super_admin: true
		}
	})
}

async function revokeApiToken(token: string | undefined) {
	if (!token) return
	try {
		await UserService.deleteToken({ tokenPrefix: token.slice(0, API_TOKEN_PREFIX_LEN) })
	} catch (err) {
		console.error(err)
	}
}

// A preview job gets its arguments interpolated on the worker: `$var:`, `$jsonvar:` and
// `$res:` references are replaced with the job's privileges before the script runs. The
// browser path has to do the same with the caller's session, or a secret stored as a linked
// variable (what the "Add resource" drawer saves) is sent verbatim as the credential.
async function resolveReferences(value: any, workspace: string): Promise<any> {
	if (typeof value === 'string') {
		if (value.startsWith('$var:')) {
			return await VariableService.getVariableValue({
				workspace,
				path: value.slice('$var:'.length)
			})
		}
		if (value.startsWith('$jsonvar:')) {
			return JSON.parse(
				await VariableService.getVariableValue({
					workspace,
					path: value.slice('$jsonvar:'.length)
				})
			)
		}
		if (value.startsWith('$res:')) {
			return await ResourceService.getResourceValueInterpolated({
				workspace,
				path: value.slice('$res:'.length)
			})
		}
		return value
	}
	if (Array.isArray(value)) {
		return await Promise.all(value.map((v) => resolveReferences(v, workspace)))
	}
	if (value && typeof value === 'object') {
		const resolved: Record<string, any> = {}
		for (const [key, v] of Object.entries(value)) {
			resolved[key] = await resolveReferences(v, workspace)
		}
		return resolved
	}
	return value
}

// The route bounds the probe only for non-super-admins; a super admin's probe against an
// endpoint that accepts the connection and never answers would otherwise spin here forever.
const BROWSER_TEST_TIMEOUT_MS = 15_000

function errorText(err: any): string {
	return String(err?.body ?? err?.message ?? err)
}

async function testObjectStorageFromBrowser(
	body: Record<string, any>,
	workspace: string
): Promise<ConnectionTestResult> {
	let timer: ReturnType<typeof setTimeout> | undefined = undefined
	try {
		const request = SettingService.testObjectStorageConfig({
			requestBody: await resolveReferences(body, workspace)
		})
		await Promise.race([
			request,
			new Promise<never>((_, reject) => {
				timer = setTimeout(() => {
					request.cancel()
					reject(
						new Error(
							`no answer from the storage endpoint after ${BROWSER_TEST_TIMEOUT_MS / 1000}s`
						)
					)
				}, BROWSER_TEST_TIMEOUT_MS)
			})
		])
		return { ok: true }
	} catch (err: any) {
		return { ok: false, error: errorText(err) }
	} finally {
		clearTimeout(timer)
	}
}

const NO_WORKER =
	'Connection did not resolve after %s or job did not start. Do you have native workers or a worker group listening to the proper tag available?'

/**
 * Runs the resource type's check against `args`, which may be the value itself or a reference
 * (`"$res:<path>"`): a preview job resolves references on the worker, so a saved secret never
 * has to be sent in clear and stored in the job's arguments.
 *
 * `viaWorker` (object-storage types only) probes from a preview job, proving a worker reaches
 * the API, instead of from the browser. The job gets a short-lived token minted for the caller,
 * since a job token is never a super admin, and that token is readable in the job's stored args
 * until revoked: only use it where the workspace's job readers may hold the caller's rights.
 */
export async function testResourceConnection({
	workspace,
	resourceType,
	args,
	viaWorker = false,
	timeoutMs = 5_000
}: {
	workspace: string
	resourceType: string
	args: unknown
	viaWorker?: boolean
	timeoutMs?: number
}): Promise<ConnectionTestResult> {
	const resourceScript = scripts[resourceType]
	if (!resourceScript) return { ok: false, error: `No connection test for ${resourceType}.` }
	const objectStorageArgs =
		resourceType in objectStorageBody ? objectStorageBody[resourceType](args) : undefined
	if (objectStorageArgs && !viaWorker) {
		return await testObjectStorageFromBrowser(objectStorageArgs, workspace)
	}

	let apiToken: string | undefined = undefined
	let job: string
	try {
		if (objectStorageArgs) apiToken = await mintApiToken(resourceType)
		job = await JobService.runScriptPreview({
			workspace,
			requestBody: {
				path: `testConnection: ${resourceType}`,
				language: resourceScript.lang as Preview['language'],
				content: resourceScript.code,
				args: objectStorageArgs
					? { bucket: objectStorageArgs, api_token: apiToken }
					: { [resourceScript.argName]: args }
			}
		})
	} catch (err: any) {
		await revokeApiToken(apiToken)
		return { ok: false, error: errorText(err) }
	}

	try {
		const deadline = Date.now() + timeoutMs
		while (Date.now() < deadline) {
			let testResult: CompletedJob
			try {
				testResult = await JobService.getCompletedJob({ workspace, id: job })
			} catch {
				await new Promise((r) => setTimeout(r, 500))
				continue
			}
			if (resourceScript.additionalCheck) testResult = resourceScript.additionalCheck(testResult)
			return testResult.success
				? { ok: true }
				: { ok: false, error: String(testResult.result?.['error']?.['message'] ?? 'failed') }
		}
		const message = NO_WORKER.replace('%s', `${timeoutMs / 1000}s`)
		try {
			await JobService.cancelQueuedJob({ workspace, id: job, requestBody: { reason: message } })
		} catch (err) {
			console.error(err)
		}
		return { ok: false, error: message }
	} finally {
		await revokeApiToken(apiToken)
	}
}

/** Whether `testUnsavedResource` can test this type before it is saved. */
export function canTestUnsaved(resourceType: string, value?: Record<string, any>): boolean {
	return resourceType in objectStorageBody || !!aiResourceProvider(resourceType, value)
}

/** Tests values the user has typed but not saved. Only where the test writes nothing: a storage
 * test is a request to the API server and an AI key goes to the AI proxy in a header. A database
 * test runs as a job whose arguments stay in its run history, so it waits for the save. */
export async function testUnsavedResource({
	workspace,
	resourceType,
	value
}: {
	workspace: string
	resourceType: string
	value: Record<string, any>
}): Promise<ConnectionTestResult | undefined> {
	if (resourceType in objectStorageBody) {
		return await testResourceConnection({ workspace, resourceType, args: value })
	}
	const aiProvider = aiResourceProvider(resourceType, value)
	if (!aiProvider) return undefined
	return await testAiKey(aiProvider, { workspace, resourceValue: value })
}

async function testAiKey(
	aiProvider: NonNullable<ReturnType<typeof aiResourceProvider>>,
	credential: { workspace: string; resourcePath?: string; resourceValue?: Record<string, any> }
): Promise<ConnectionTestResult> {
	const { testKey } = await import('./copilot/lib')
	const abortController = new AbortController()
	const timer = setTimeout(() => abortController.abort(), 15_000)
	try {
		await testKey({
			...credential,
			model: undefined,
			messages: [{ role: 'user', content: "this is a test, simply reply with 'ok'" }],
			abortController,
			aiProvider
		})
		return { ok: true }
	} catch (err: any) {
		return {
			ok: false,
			error: abortController.signal.aborted
				? 'No answer from the provider within 15s.'
				: errorText(err)
		}
	} finally {
		clearTimeout(timer)
	}
}
