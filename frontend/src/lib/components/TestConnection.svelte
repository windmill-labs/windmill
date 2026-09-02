<script lang="ts">
	import { type CompletedJob, JobService, type Preview, UserService } from '$lib/gen'

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
	}

	let {
		workspaceOverride = undefined,
		resourceType,
		args = {},
		buttonTextOverride = undefined
	}: Props = $props()

	const scripts: {
		[key: string]: {
			code: string
			lang: string
			argName: string
			// Shown as an info tooltip next to the button, e.g. to clarify where the test executes
			tooltip?: string
			// The object-storage probe runs on the API server, whose route never treats a job token
			// ($WM_TOKEN) as a super admin. A script naming this argument receives a short-lived
			// `settings:write` token minted for the caller instead, so a super admin keeps the
			// unrestricted path (private endpoints, ambient server credentials).
			apiTokenArg?: string
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
			code: `
import * as wmill from "windmill-client"

type S3 = object

export async function main(s3: S3, api_token: string) {
	return fetch(process.env["BASE_URL"] + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + api_token,
		},
		body: JSON.stringify({
			type: "S3",
			region: s3.region,
			bucket: s3.bucket,
			endpoint: s3.endPoint,
			port: s3.port,
			allow_http: !s3.useSSL,
			access_key: s3.accessKey,
			secret_key: s3.secretKey,
			path_style: s3.pathStyle,
		}),
	}).then(async (res) => {
		if (!res.ok) {
			throw new Error(await res.text())
		}
		return res.text()
	})
}
`,
			lang: 'bun',
			argName: 's3',
			apiTokenArg: 'api_token',
			tooltip:
				'The storage operations of this test run on the Windmill server (the API process) with your permissions, not on the worker. Non-super-admins can only test public endpoints with an explicit access key and secret key; super admins can also test private endpoints and rely on the ambient AWS credentials of the server (environment variables, instance role). Scripts using this resource directly through an S3 SDK resolve credentials on the worker instead, so results may differ.'
		},
		azure_blob: {
			code: `
import * as wmill from "windmill-client"

type S3 = object

export async function main(s3: S3, api_token: string) {
	return fetch(process.env["BASE_URL"] + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + api_token,
		},
		body: JSON.stringify({
			type: "Azure",
			...s3
		}),
	}).then(async (res) => {
		if (!res.ok) {
			throw new Error(await res.text())
		}
		return res.text()
	})
}
`,
			lang: 'bun',
			argName: 's3',
			apiTokenArg: 'api_token',
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
			code: `

const process = require('process');

export async function main(bucket: any, api_token: string) {
	const req = await fetch(process.env.BASE_URL + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + api_token,
		},
		body: JSON.stringify(bucket),
	});
	if (!req.ok) {
		throw new Error(await req.text());
	}
	return await req.text();
}
`,
			lang: 'bun',
			argName: 'bucket',
			apiTokenArg: 'api_token',
			tooltip:
				"The storage operations of this test run on the Windmill server (the API process) with your permissions. Non-super-admins can only test public endpoints with explicit credentials; super admins can also test private endpoints and rely on the server's ambient credentials for the configured provider (environment variables, instance role)."
		}
	}

	let loading = $state(false)

	// The token is revoked as soon as the job settles; the expiry only covers a browser that
	// goes away mid-test. It stays visible in the job's stored args, hence the short life.
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

	async function testConnection() {
		if (!resourceType) return
		loading = true

		const resourceScript = scripts[resourceType]
		const workspace = workspaceOverride ?? $workspaceStore!

		let apiToken: string | undefined = undefined
		let job: string
		try {
			if (resourceScript.apiTokenArg) {
				apiToken = await mintApiToken()
			}
			job = await JobService.runScriptPreview({
				workspace,
				requestBody: {
					path: `testConnection: ${resourceType}`,
					language: resourceScript.lang as Preview['language'],
					content: resourceScript.code,
					args: {
						[resourceScript.argName]: args,
						...(resourceScript.apiTokenArg && apiToken
							? { [resourceScript.apiTokenArg]: apiToken }
							: {})
					}
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
