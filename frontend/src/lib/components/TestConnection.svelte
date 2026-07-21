<script lang="ts">
	import { type CompletedJob, JobService, type Preview } from '$lib/gen'

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

export async function main(s3: S3) {
	return fetch(process.env["BASE_URL"] + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + process.env["WM_TOKEN"],
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
			tooltip:
				'If no access key/secret key is set, the ambient AWS credentials are used. If you only configured AWS credentials on your workers and not the server, this test will fail (but the resource will work from the workers).'
		},
		azure_blob: {
			code: `
import * as wmill from "windmill-client"

type S3 = object

export async function main(s3: S3) {
	return fetch(process.env["BASE_URL"] + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + process.env["WM_TOKEN"],
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
			tooltip: 'The storage operations of this test run on the Windmill server.'
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

export async function main(bucket: any) {
	const req = await fetch(process.env.BASE_URL + '/api/settings/test_object_storage_config', {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: 'Bearer ' + process.env.WM_TOKEN,
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
			tooltip:
				'The storage operations of this test run on the Windmill server; running it as a job additionally verifies that a worker can reach the server API. If no access key/secret key is set, the ambient AWS credentials of the server (environment variables, instance role) are used.'
		}
	}

	let loading = $state(false)
	async function testConnection() {
		if (!resourceType) return
		loading = true

		const resourceScript = scripts[resourceType]

		const job = await JobService.runScriptPreview({
			workspace: workspaceOverride ?? $workspaceStore!,
			requestBody: {
				path: `testConnection: ${resourceType}`,
				language: resourceScript.lang as Preview['language'],
				content: resourceScript.code,
				args: {
					[resourceScript.argName]: args
				}
			}
		})

		tryEvery({
			tryCode: async () => {
				let testResult = await JobService.getCompletedJob({
					workspace: workspaceOverride ?? $workspaceStore!,
					id: job
				})
				if (resourceScript.additionalCheck) {
					testResult = resourceScript.additionalCheck(testResult)
				}
				loading = false
				sendUserToast(
					testResult.success
						? 'Connection successful'
						: 'Connection error: ' + testResult.result?.['error']?.['message'],
					!testResult.success
				)
			},
			timeoutCode: async () => {
				loading = false
				sendUserToast(
					'Connection did not resolve after 5s or job did not start. Do you have native workers or a worker group listening to the proper tag available?',
					true
				)
				try {
					await JobService.cancelQueuedJob({
						workspace: workspaceOverride ?? $workspaceStore!,
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
