<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { FlowService, ScriptService, type Flow } from '$lib/gen'
	import { clearPreviewResults, hubScripts, workspaceStore } from '$lib/stores'
	import { sendUserToast, setQueryWithoutLoad } from '$lib/utils'
	import { faFileExport, faFileImport } from '@fortawesome/free-solid-svg-icons'
	import { onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import FlowEditor from './FlowEditor.svelte'
	import { flowStore, type FlowMode } from './flows/flowStore'
	import { flowToMode } from './flows/utils'

	import ScriptSchema from './ScriptSchema.svelte'

	export let initialPath: string = ''
	let pathError = ''

	let mode: FlowMode

	$: step = Number($page.url.searchParams.get('step')) || 1

	async function loadSearchData() {
		const scripts = await ScriptService.listHubScripts()
		$hubScripts = scripts.map((x) => ({
			path: `hub/${x.id}/${x.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
			summary: `${x.summary} (${x.app})`,
			approved: x.approved,
			is_trigger: x.is_trigger
		}))
	}

	async function saveFlow(): Promise<void> {
		const newFlow = flowToMode($flowStore, mode)

		if (initialPath === '') {
			await FlowService.createFlow({
				workspace: $workspaceStore!,
				requestBody: {
					path: newFlow.path,
					summary: newFlow.summary,
					description: newFlow.description ?? '',
					value: newFlow.value,
					schema: newFlow.schema
				}
			})
		} else {
			await FlowService.updateFlow({
				workspace: $workspaceStore!,
				path: newFlow.path,
				requestBody: {
					path: newFlow.path,
					summary: newFlow.summary,
					description: newFlow.description ?? '',
					value: newFlow.value,
					schema: newFlow.schema
				}
			})
		}
		sendUserToast(`Success! flow saved at ${$flowStore.path}`)
		goto(`/flows/get/${$flowStore.path}`)
	}

	async function changeStep(step: number) {
		goto(`?step=${step}`)
	}

	flowStore.subscribe((flow: Flow) => {
		setQueryWithoutLoad($page.url, 'state', btoa(JSON.stringify(flowToMode(flow, mode))))
	})

	onMount(() => {
		loadSearchData()
		clearPreviewResults()
	})
</script>

<div class="flex flex-col h-screen max-w-screen-lg xl:-ml-20 xl:pl-4 w-full -mt-4 pt-4 md:mx-10 ">
	<!-- Nav between steps-->
	<div class="flex flex-col w-full">
		<div class="justify-between flex flex-row drop-shadow-sm w-full">
			<div class="wizard-nav flex flex-row w-full">
				<button
					disabled={pathError != ''}
					class="{step === 1
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(1)
					}}>Step 1: Flow</button
				>
				<button
					disabled={pathError != ''}
					class="{step === 2
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(2)
					}}>Step 2: UI customisation</button
				>
			</div>
			<div class="flex flex-row-reverse ml-2">
				{#if step == 1}
					<button
						disabled={pathError != ''}
						class="default-button px-6 max-h-8"
						on:click={() => {
							changeStep(step + 1)
						}}
					>
						Next
					</button>
					<button
						disabled={pathError != ''}
						class="default-button-secondary px-6 max-h-8 mr-2"
						on:click={saveFlow}
					>
						Save
					</button>
				{:else}
					<button class="default-button px-6 self-end" on:click={saveFlow}>Save</button>
				{/if}
			</div>
		</div>
		<div class="flex flex-row-reverse">
			<span class="my-1 text-sm text-gray-500 italic">
				{#if initialPath && initialPath != $flowStore.path} {initialPath} &rightarrow; {/if}
				{$flowStore.path}
			</span>
		</div>
	</div>

	<!-- metadata -->

	{#if step === 1}
		<FlowEditor bind:mode bind:pathError bind:initialPath />
	{:else if step === 2}
		<ScriptSchema
			synchronizedHeader={false}
			bind:summary={$flowStore.summary}
			bind:description={$flowStore.description}
			bind:schema={$flowStore.schema}
		/>
	{/if}
</div>

<style>
	/* .wizard-nav {
		@apply w-1/2 sm:w-1/4;
	} */

	.wizard-nav button {
		max-height: 30px;
	}
</style>
