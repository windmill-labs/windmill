<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { workerTags } from '$lib/stores'
	import { WorkerService } from '$lib/gen'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'

	export let tag: string | undefined
	export let nullTag: string | undefined = undefined

	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}
</script>

{#if $workerTags}
	{#if $workerTags?.length > 0}
		<div class="w-40">
			{#if $flowStore.tag == undefined}
				<WorkerTagSelect {nullTag} bind:tag />
			{:else}
				<button
					title="Worker Group is defined at the flow level"
					class="w-full text-left items-center font-normal p-1 py-2 border text-xs rounded"
					on:click={() => ($selectedId = 'settings-worker-group')}
				>
					Flow's WG: {$flowStore.tag}
				</button>
			{/if}
		</div>
	{/if}
{/if}
