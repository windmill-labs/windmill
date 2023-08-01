<script lang="ts">
	import { JobService, Preview } from '$lib/gen'

	import { Database, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	export let resource_type: string | undefined
	export let args: Record<string, any> | any = {}

	const content = {
		postgresql: `import { Client } from 'https://deno.land/x/postgres@v0.17.0/mod.ts'
export async function main(args: any) {
  const u = new URL("postgres://")
	u.hash = ''
	u.search = '?sslmode=' + args.sslmode
	u.pathname = args.dbname
	u.host = args.host
	u.port = args.port
	u.password = args.password
	u.username = args.user
	const client = new Client(u.toString())
	await client.connect()
	return 'Connection successful'
}`,
		mysql: `import { Client } from "https://deno.land/x/mysql@v2.11.0/mod.ts";
export async function main(args: any) {
  const conn = await new Client().connect({
    hostname: args.host,
    port: args.port,
    username: args.user,
    db: args.database,
    password: args.password,
  });
  await conn.query("SELECT 1");
  return "Connection successful";
}`
	}

	let loading = false
	async function testConnection() {
		if (!resource_type) return
		loading = true

		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
			requestBody: {
				language: 'deno' as Preview.language,
				content: content[resource_type],
				args: {
					args
				}
			}
		})
		await new Promise((r) => setTimeout(r, 5000))
		loading = false
		try {
			const testResult = await JobService.getCompletedJob({
				workspace: $workspaceStore!,
				id: job
			})
			if (testResult) {
				sendUserToast(
					testResult.success ? testResult.result : testResult.result?.['error']?.['message'],
					!testResult.success
				)
			}
		} catch (e) {
			sendUserToast('Connection did not resolve after 5s')
		}
	}
</script>

{#if resource_type == 'postgresql' || resource_type == 'mysql'}
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
