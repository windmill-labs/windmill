<script lang="ts">
	import { CompletedJob, JobService, Preview } from '$lib/gen'

	import { Database, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { tryEvery } from '$lib/utils'

	export let resourceType: string | undefined
	export let args: Record<string, any> | any = {}

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
		ms_sql_server: {
			code: `SELECT 1`,
			lang: 'mssql',
			argName: 'database'
		},
		s3: {
			code: `
import { S3Client } from "https://deno.land/x/s3_lite_client@0.6.1/mod.ts";		

type S3 = object

export async function main(s3: S3) {
	const s3client = new S3Client(s3);
	for await (const obj of s3client.listObjects({ prefix: "/" })) {
		console.log(obj);
	}
}
`,
			lang: 'deno',
			argName: 's3'
		},
		graphql: {
			code: '{ __typename }',
			lang: 'graphql',
			argName: 'api',
			additionalCheck: (testResult: CompletedJob) => {
				if (
					testResult.success &&
					(typeof testResult.result !== 'object' || !('__typename' in testResult.result))
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
		}
	}

	let loading = false
	async function testConnection() {
		if (!resourceType) return
		loading = true

		const resourceScript = scripts[resourceType]

		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
			requestBody: {
				path: `testConnection: ${resourceType}`,
				language: resourceScript.lang as Preview.language,
				content: resourceScript.code,
				args: {
					[resourceScript.argName]: args
				}
			}
		})

		tryEvery({
			tryCode: async () => {
				let testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
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
						workspace: $workspaceStore!,
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
		Test connection
	</Button>
{/if}
