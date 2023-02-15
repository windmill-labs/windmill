<script lang="ts">
	import { JobService, Preview, ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString, sendUserToast } from '$lib/utils'
	import { Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'

	export let resource_type: string
	export let args: Record<string, any> | any = {}
	export let password: string
	export let isValid = true

	let schema = emptySchema()
	let notFound = false

	async function loadSchema() {
		rawCode = '{}'
		viewJsonSchema = false
		try {
			const rt = await ResourceService.getResourceType({
				workspace: $workspaceStore!,
				path: resource_type
			})
			schema = rt.schema
			notFound = false
		} catch (e) {
			notFound = true
		}
	}
	$: {
		$workspaceStore && loadSchema()
	}
	$: notFound && rawCode && parseJson()

	function parseJson() {
		try {
			args = JSON.parse(rawCode)
			error = ''
			isValid = true
		} catch (e) {
			isValid = false
			error = e.message
		}
	}
	let error = ''
	let rawCode = ''
	let viewJsonSchema = false

	$: rawCode && parseJson()

	function switchTab(asJson: boolean) {
		viewJsonSchema = asJson
		if (asJson) {
			rawCode = JSON.stringify(args, null, 2)
		} else {
			parseJson()
		}
	}

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
	const client = new Client("postgres://" + args.user + ":" + args.password + "@" + args.host + ":" + args.port + "/" + args.dbname + "?sslmode=" + args.sslmode)
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

{#if !notFound}
	<div class="w-full flex gap-4 flex-row-reverse">
		<Toggle
			on:change={(e) => switchTab(e.detail)}
			options={{
				right: 'As JSON'
			}}
		/>
		{#if resource_type == 'postgresql'}
			<Button size="sm" on:click={testConnection}
				>{#if loading}<Loader2 class="animate-spin mr-2" />{/if} Test connection</Button
			>
		{/if}
	</div>
{:else}
	<p class="italic text-gray-500 text-xs mb-4"
		>No corresponding resource type found in your workspace for {resource_type}. Define the value in
		JSON directly</p
	>
{/if}
{#if notFound || viewJsonSchema}
	{#if !emptyString(error)}<span class="text-red-400 text-xs mb-1 flex flex-row-reverse"
			>{error}</span
		>{:else}<div class="py-2" />{/if}
	<SimpleEditor autoHeight lang="json" bind:code={rawCode} fixedOverflowWidgets={false} />
{:else}
	<SchemaForm noDelete {password} isValid {schema} bind:args />
{/if}
