<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ExternalLink, RotateCw, Loader2 } from 'lucide-svelte'
	import { workerTags, workspaceStore } from '$lib/stores'
	import AssignableTags from './AssignableTags.svelte'
	import { WorkerService } from '$lib/gen'
	import WorkerTagSelect from './WorkerTagSelect.svelte'

	interface Props {
		tag: string | undefined
		popupPlacement?: 'bottom-end' | 'top-end'
		disabled?: boolean
		placeholder?: string
		// Workspace to read tags from; defaults to $workspaceStore. A fork-scoped
		// session passes its effective workspace so the picker matches the deploy target.
		workspaceId?: string
	}

	let {
		tag = $bindable(),
		popupPlacement = 'bottom-end',
		disabled = false,
		placeholder,
		workspaceId = undefined
	}: Props = $props()

	// See WorkerTagSelect: the shared `workerTags` cache is navigation-scoped, so a
	// different target workspace reads/writes a local list to avoid clobbering it.
	let effectiveWorkspace = $derived(workspaceId ?? $workspaceStore)
	let usesLocal = $derived(workspaceId != undefined && workspaceId !== $workspaceStore)
	let localWorkerTags = $state<string[] | undefined>(undefined)
	let currentTags = $derived(usesLocal ? localWorkerTags : $workerTags)

	loadWorkerTags()
	async function loadWorkerTags(force = false) {
		if (usesLocal) {
			if (!localWorkerTags || force) {
				localWorkerTags = await WorkerService.getCustomTagsForWorkspace({
					workspace: effectiveWorkspace!
				})
			}
		} else if (!$workerTags || force) {
			$workerTags = await WorkerService.getCustomTagsForWorkspace({
				workspace: effectiveWorkspace!
			})
		}
	}
</script>

<div class="flex gap-2 items-center">
	<div class="max-w-sm grow">
		{#if currentTags}
			{#if currentTags?.length ?? 0 > 0}
				<WorkerTagSelect
					{placeholder}
					noLabel
					bind:tag
					{disabled}
					workspaceId={usesLocal ? effectiveWorkspace : undefined}
				/>
			{:else}
				<div class="text-sm text-secondary flex flex-row gap-2">
					No custom worker group tag defined on this instance in "Workers {'->'} Custom tags"
					<a
						href="https://www.windmill.dev/docs/core_concepts/worker_groups"
						target="_blank"
						class="hover:underline"
					>
						<div class="flex flex-row gap-2 items-center">
							See documentation
							<ExternalLink size="12" />
						</div>
					</a>
				</div>
			{/if}
		{:else}
			<Loader2 class="animate-spin" />
		{/if}
	</div>

	<Button
		variant="default"
		on:click={() => {
			loadWorkerTags(true)
		}}
		startIcon={{ icon: RotateCw }}
		{disabled}
	/>
	<AssignableTags placement={popupPlacement} {disabled} />
</div>
