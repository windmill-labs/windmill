<script lang="ts">
	import { OauthService, ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import { Button } from './common'

	export let resource_type: string
	export let args: Record<string, any> | any = {}
	export let password: string
	export let isValid = true

	let schema = emptySchema()
	let notFound = false

	let supabaseWizard = false

	async function isSupabaseAvailable() {
		supabaseWizard = (await OauthService.listOAuthConnects())['supabase_wizard'] != undefined
	}
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

	$: resource_type == 'postgresql' && isSupabaseAvailable()
</script>

{#if !notFound}
	<div class="w-full flex gap-4 flex-row-reverse">
		<Toggle
			on:change={(e) => switchTab(e.detail)}
			options={{
				right: 'As JSON'
			}}
		/>
		<TestConnection {resource_type} {args} />
		{#if resource_type == 'postgresql' && supabaseWizard}
			<Button variant="border" target="_blank" href="/api/oauth/connect/supabase_wizard">
				Add a Supabase DB
			</Button>
		{/if}
	</div>
{:else}
	<p class="italic text-tertiary text-xs mb-4"
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
