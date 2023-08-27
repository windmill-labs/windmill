<script lang="ts">
	import { OauthService, ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, emptyString } from '$lib/utils'
	import SchemaForm from './SchemaForm.svelte'
	import SimpleEditor from './SimpleEditor.svelte'
	import Toggle from './Toggle.svelte'
	import TestConnection from './TestConnection.svelte'
	import SupabaseIcon from './icons/SupabaseIcon.svelte'

	export let resource_type: string
	export let args: Record<string, any> | any = {}
	export let linkedSecret: string | undefined = undefined
	export let isValid = true
	export let linkedSecretCandidates: string[] | undefined = undefined

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
	<div class="w-full flex gap-4 flex-row-reverse items-center">
		<Toggle
			on:change={(e) => switchTab(e.detail)}
			options={{
				right: 'As JSON'
			}}
		/>
		<TestConnection {resource_type} {args} />
		{#if resource_type == 'postgresql' && supabaseWizard}
			<a
				target="_blank"
				href="/api/oauth/connect/supabase_wizard"
				class="border rounded-lg flex flex-row gap-2 items-center text-xs px-3 py-1.5 h-8 bg-[#F1F3F5] hover:bg-[#E6E8EB] dark:bg-[#1C1C1C] dark:hover:bg-black"
			>
				<SupabaseIcon height="16px" width="16px" />
				<div class="text-[#11181C] dark:text-[#EDEDED] font-semibold">Connect Supabase</div>
			</a>
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
	<SchemaForm noDelete {linkedSecretCandidates} bind:linkedSecret isValid {schema} bind:args />
{/if}
