<script lang="ts">
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { workerTags } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	export let tag: string | undefined

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
				<select
					placeholder="Tag"
					bind:value={tag}
					on:change={(e) => {
						if (tag == '') {
							tag = undefined
						}
					}}
				>
					{#if tag}
						<option value="">reset to default</option>
					{:else}
						<option value={undefined} disabled selected>Tag</option>
					{/if}
					{#each $workerTags ?? [] as tag (tag)}
						<option value={tag}>{tag}</option>
					{/each}
				</select>
			{:else}
				<button
					title="Worker Group is defined at the flow level"
					class="w-full text-left items-center font-normal p-1 border text-xs rounded"
					on:click={() => ($selectedId = 'settings-worker-group')}
				>
					Flow's WG: {$flowStore.tag}
				</button>
			{/if}
		</div>
	{/if}
{/if}
