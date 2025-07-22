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
	}

	let {
		tag = $bindable(),
		popupPlacement = 'bottom-end',
		disabled = false,
		placeholder
	}: Props = $props()

	loadWorkerTags()
	async function loadWorkerTags(force = false) {
		if (!$workerTags || force) {
			$workerTags = await WorkerService.getCustomTags({ workspace: $workspaceStore })
		}
	}
</script>

<div class="flex gap-2 items-center">
	<div class="max-w-sm grow">
		{#if $workerTags}
			{#if $workerTags?.length ?? 0 > 0}
				<WorkerTagSelect {placeholder} noLabel bind:tag {disabled} />
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
		variant="border"
		color="light"
		on:click={() => {
			$workerTags = undefined
			loadWorkerTags(true)
		}}
		startIcon={{ icon: RotateCw }}
		{disabled}
	/>
	<AssignableTags placement={popupPlacement} color="nord" {disabled} />
</div>
