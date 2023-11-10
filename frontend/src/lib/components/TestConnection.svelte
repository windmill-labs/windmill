<script lang="ts">
	import { CompletedJob, JobService, Preview } from '$lib/gen'

	import { Database, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { tryEvery } from '$lib/utils'

	export let resource_type: string | undefined
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
		if (!resource_type) return
		loading = true

		const resourceScript = scripts[resource_type]

		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
			requestBody: {
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
				sendUserToast('Connection did not resolve after 5s', true)
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: job,
						requestBody: {
							reason: 'Connection did not resolve after 5s'
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

{#if Object.keys(scripts).includes(resource_type || '')}
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
