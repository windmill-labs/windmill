<script lang="ts">
	import {
		type CompletedJob,
		JobService,
		type Preview,
		ResourceService,
		SettingService,
		UserService,
		VariableService
	} from '$lib/gen'

	import { Database, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { tryEvery } from '$lib/utils'

	interface Props {
		workspaceOverride?: string | undefined
		resourceType: string | undefined
		args?: Record<string, any> | any
		buttonTextOverride?: string | undefined
		// Object-storage types only: probe from a preview job (proves a worker reaches the API)
		// instead of the browser. The job gets a short-lived token minted for the caller, since a
		// job token is never a super admin, and that token is readable in the job's stored args
		// until revoked: only use it where the workspace's job readers may hold the caller's rights.
		viaWorker?: boolean
	}

	let {
		workspaceOverride = undefined,
		resourceType,
		args = {},
		buttonTextOverride = undefined,
		viaWorker = false
	}: Props = $props()

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

	const scripts: {
		[key: string]: {
			code: string
			lang: string
			argName: string
			// Shown as an info tooltip next to the button, e.g. to clarify where the test executes
			tooltip?: string
			additionalCheck?: (testResult: CompletedJob) => CompletedJob
		}
	} = {
		postgresql: {
			code: `SELECT 1`,
			lang: 'postgresql',
			argName: 'database'
		},
		mysql: {
			code: `SELECT 1`,
			lang: 'mysql',
			argName: 'database'
		},
		bigquery: {
			code: `select 1`,
			lang: 'bigquery',
			argName: 'database'
		},
		snowflake: {
			code: `select 1`,
			lang: 'snowflake',
			argName: 'database'
		},
		snowflake_oauth: {
			code: `select 1`,
			lang: 'snowflake',
			argName: 'database'
		},
		mssql: {
			code: `SELECT 1`,
			lang: 'mssql',
			argName: 'database'
		},
		oracledb: {
			code: `SELECT 1 FROM DUAL`,
			lang: 'oracledb',
			argName: 'database'
		},
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
			additionalCheck: (testResult: CompletedJob) => {
				if (
					testResult.success &&
					(typeof testResult.result !== 'object' || !('__typename' in (testResult.result ?? {})))
				) {
					return {
						...testResult,
						result: {
							error: {
								message: 'Invalid GraphQL API response'
							}
						},
						success: false
					}
				} else {
					return testResult
				}
			}
		},
		s3_bucket: {
			code: OBJECT_STORAGE_TEST_SCRIPT,
			lang: 'bun',
			argName: 'bucket',
			tooltip:
				"The storage operations of this test run on the Windmill server (the API process) with your permissions. Non-super-admins can only test public endpoints with explicit credentials; super admins can also test private endpoints and rely on the server's ambient credentials for the configured provider (environment variables, instance role)."
		}
	}

	let loading = $state(false)

	// The token is revoked as soon as the job settles; the expiry only covers a browser that
	// goes away mid-test. It is readable in the job's stored args until then, hence the short life.
	const API_TOKEN_TTL_MS = 60_000
	// Tokens are addressed by their first 10 characters (TOKEN_PREFIX_LEN on the backend).
	const API_TOKEN_PREFIX_LEN = 10

	async function mintApiToken(): Promise<string> {
		return await UserService.createToken({
			requestBody: {
				label: `test connection: ${resourceType}`,
				expiration: new Date(Date.now() + API_TOKEN_TTL_MS).toISOString(),
				scopes: ['settings:write']
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

	async function testObjectStorageFromBrowser(body: Record<string, any>, workspace: string) {
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
			sendUserToast('Connection successful', false)
		} catch (err: any) {
			sendUserToast('Connection error: ' + (err?.body ?? err?.message ?? err), true)
		} finally {
			clearTimeout(timer)
			loading = false
		}
	}

	async function testConnection() {
		if (!resourceType) return
		loading = true

		const resourceScript = scripts[resourceType]
		const workspace = workspaceOverride ?? $workspaceStore!
		const objectStorageArgs: Record<string, any> | undefined =
			resourceType in objectStorageBody ? objectStorageBody[resourceType](args) : undefined

		if (objectStorageArgs && !viaWorker) {
			await testObjectStorageFromBrowser(objectStorageArgs, workspace)
			return
		}

		let apiToken: string | undefined = undefined
		let job: string
		try {
			if (objectStorageArgs) {
				apiToken = await mintApiToken()
			}
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
			loading = false
			await revokeApiToken(apiToken)
			sendUserToast('Connection error: ' + (err?.body ?? err?.message ?? err), true)
			return
		}

		tryEvery({
			tryCode: async () => {
				let testResult = await JobService.getCompletedJob({
					workspace,
					id: job
				})
				if (resourceScript.additionalCheck) {
					testResult = resourceScript.additionalCheck(testResult)
				}
				loading = false
				revokeApiToken(apiToken)
				sendUserToast(
					testResult.success
						? 'Connection successful'
						: 'Connection error: ' + testResult.result?.['error']?.['message'],
					!testResult.success
				)
			},
			timeoutCode: async () => {
				loading = false
				revokeApiToken(apiToken)
				sendUserToast(
					'Connection did not resolve after 5s or job did not start. Do you have native workers or a worker group listening to the proper tag available?',
					true
				)
				try {
					await JobService.cancelQueuedJob({
						workspace,
						id: job,
						requestBody: {
							reason:
								'Connection did not resolve after 5s. Do you have native workers or a worker group listening to the proper tag available?'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 5000
		})
	}
</script>

{#if resourceType && Object.keys(scripts).includes(resourceType)}
	<div class="flex items-center gap-1">
		<Button spacingSize="sm" size="xs" unifiedSize="md" variant="default" on:click={testConnection}>
			{#if loading}
				<Loader2 class="animate-spin mr-2 !h-4 !w-4" />
			{:else}
				<Database class="mr-2 !h-4 !w-4" />
			{/if}
			{buttonTextOverride ?? 'Test connection'}
		</Button>
		{#if scripts[resourceType].tooltip}
			<Tooltip>
				{#snippet text()}{scripts[resourceType].tooltip}{/snippet}
			</Tooltip>
		{/if}
	</div>
{/if}
