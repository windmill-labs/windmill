<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ExternalLink, RotateCw, Loader2 } from 'lucide-svelte'
	import { workerTags } from '$lib/stores'
	import AssignableTags from './AssignableTags.svelte'
	import { WorkerService } from '$lib/gen'

	export let tag: string | undefined
	export let popupPlacement: 'bottom-end' | 'top-end' = 'bottom-end'

	loadWorkerGroups()
	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}
</script>

<div class="flex gap-2 items-center">
	<div class="max-w-sm grow">
		{#if workerTags}
			{#if $workerTags?.length ?? 0 > 0}
				<select
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
						<option value="" disabled selected>Worker Group Tag</option>
					{/if}
					{#each $workerTags ?? [] as tag (tag)}
						<option value={tag}>{tag}</option>
					{/each}
				</select>
			{:else}
				<div class="text-sm text-secondary flex flex-row gap-2">
					No custom worker group tag defined on this instance in "Workers {'->'} Assignable Tags"
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
			loadWorkerGroups()
		}}
		startIcon={{ icon: RotateCw }}
	/>
	<AssignableTags placement={popupPlacement} />
</div>
