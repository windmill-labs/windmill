<script lang="ts">
	import { JobService, Preview } from '$lib/gen'

	import { Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'

	export let resource_type: string | undefined
	export let args: Record<string, any> | any = {}

	let loading = false
	async function testConnection() {
		loading = true
		const job = await JobService.runScriptPreview({
			workspace: $workspaceStore!,
			requestBody: {
				language: 'deno' as Preview.language,
				content: `
import { Client } from 'https://deno.land/x/postgres/mod.ts'
export async function main(args: any) {
	const u = new URL(`postgres://${args.user}:${args.password}@${args.host}:${args.port ?? 5432}/${args.dbname}?sslmode=${args.sslmode}`)
	u.hash = ''
	u.search = `?sslmode=${args.sslmode}`
	u.pathname = args.dbname
	u.host = args.host
	u.password = args.password
	u.username = args.user
	const client = new Client(u.toString())
	await client.connect()
	return 'Connection successful'
}
				`,
				args: {
					args
				}
			}
		})
		await new Promise((r) => setTimeout(r, 3000))
		loading = false
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
	}
</script>

{#if resource_type == 'postgresql'}
	<Button size="sm" on:click={testConnection}
		>{#if loading}<Loader2 class="animate-spin mr-2" />{/if} Test connection</Button
	>
{/if}
