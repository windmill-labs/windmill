<script lang="ts">
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { workerTags, workspaceStore } from '$lib/stores'
	import { WorkerService } from '$lib/gen'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'

	let {
		tag = $bindable(),
		nullTag,
		placeholder,
		noLabel,
		isPreprocessor
	}: {
		tag: string | undefined
		nullTag?: string | undefined
		placeholder?: string
		noLabel?: boolean
		isPreprocessor: boolean
	} = $props()

	const { flowStore, selectionManager, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')

	// A fork-scoped session deploys to opWorkspace, not $workspaceStore. Keep a local
	// tag list in that case so the gate reflects the fork without clobbering the
	// shared, navigation-scoped `workerTags` cache.
	let effectiveWorkspace = $derived(opWorkspace?.() ?? $workspaceStore)
	let usesLocal = $derived(
		effectiveWorkspace != undefined && effectiveWorkspace !== $workspaceStore
	)
	let localWorkerTags = $state<string[] | undefined>(undefined)
	let currentTags = $derived(usesLocal ? localWorkerTags : $workerTags)

	const dispatch = createEventDispatcher()
	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (usesLocal) {
			if (!localWorkerTags) {
				localWorkerTags = await WorkerService.getCustomTagsForWorkspace({
					workspace: effectiveWorkspace!
				})
			}
		} else if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTagsForWorkspace({
				workspace: effectiveWorkspace!
			})
		}
	}
</script>

{#if currentTags}
	{#if currentTags?.length > 0}
		<div class="w-40">
			{#if flowStore.val.tag == undefined || isPreprocessor || flowStore.val.value?.preserve_step_tags}
				<WorkerTagSelect
					{noLabel}
					{placeholder}
					{nullTag}
					bind:tag
					workspaceId={usesLocal ? effectiveWorkspace : undefined}
					on:change={(e) => dispatch('change', e.detail)}
				/>
			{:else}
				<button
					title="Worker group is defined at the flow level"
					class="w-full text-left items-center font-normal p-1 py-2 border text-xs rounded"
					onclick={() => selectionManager.selectId('settings-worker-group')}
				>
					Flow's WG: {flowStore.val.tag}
				</button>
			{/if}
		</div>
	{/if}
{/if}
