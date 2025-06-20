<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'

	let { tag = $bindable(), nullTag = $bindable() }: {
		tag: string | undefined
		nullTag?: string | undefined
	} = $props()

	const { flowStore, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	const dispatch = createEventDispatcher()
	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags({ workspace: $workspaceStore })
		}
	}
</script>

{#if $workerTags}
	{#if $workerTags?.length > 0}
		<div class="w-40">
			{#if flowStore.val.tag == undefined}
				<WorkerTagSelect {nullTag} bind:tag on:change={(e) => dispatch('change', e.detail)} />
			{:else}
				<button
					title="Worker Group is defined at the flow level"
					class="w-full text-left items-center font-normal p-1 py-2 border text-xs rounded"
					onclick={() => ($selectedId = 'settings-worker-group')}
				>
					Flow's WG: {flowStore.val.tag}
				</button>
			{/if}
		</div>
	{/if}
{/if}
