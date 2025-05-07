<script lang="ts">
	import { type CompletedJob, JobService, type Preview } from '$lib/gen'

	import { Database, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { tryEvery } from '$lib/utils'

	export let workspaceOverride: string | undefined = undefined
	export let resourceType: string | undefined
	export let args: Record<string, any> | any = {}
	export let buttonTextOverride: string | undefined = undefined

	const scripts: {
		[key: string]: {
			code: string
			lang: string
			argName: string
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
		ms_sql_server: {
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
			argName: 's3'
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
			argName: 's3'
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
			argName: 'bucket'
		}
	}

	let loading = false
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

{#if Object.keys(scripts).includes(resourceType || '')}
	<Button
		spacingSize="sm"
		size="xs"
		btnClasses="h-8"
		color="light"
		variant="border"
		on:click={testConnection}
	>
		{#if loading}
			<Loader2 class="animate-spin mr-2 !h-4 !w-4" />
		{:else}
			<Database class="mr-2 !h-4 !w-4" />
		{/if}
		{buttonTextOverride ?? 'Test connection'}
	</Button>
{/if}
