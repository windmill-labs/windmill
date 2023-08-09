<script lang="ts">
	import { JobService, Preview } from '$lib/gen'

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
		}
	} = {
		postgresql: {
			code: `import { Client } from 'https://deno.land/x/postgres@v0.17.0/mod.ts'
export async function main(database: any) {
  const u = new URL("postgres://")
	u.hash = ''
	u.search = '?sslmode=' + database.sslmode
	u.pathname = database.dbname
	u.host = database.host
	u.port = database.port
	u.password = database.password
	u.username = database.user
	const client = new Client(u.toString())
	await client.connect()
}`,
			lang: 'deno',
			argName: 'database'
		},
		mysql: {
			code: `import { Client } from "https://deno.land/x/mysql@v2.11.0/mod.ts";
export async function main(database: any) {
  const conn = await new Client().connect({
    hostname: database.host,
    port: database.port,
    username: database.user,
    db: database.database,
    password: database.password,
  });
  await conn.query("SELECT 1");
}`,
			lang: 'deno',
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
		graphql: {
			code: '{ __typename }',
			lang: 'graphql',
			argName: 'api'
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
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: job
				})
				loading = false
				sendUserToast(
					testResult.success ? 'Connection successful' : testResult.result?.['error']?.['message'],
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
