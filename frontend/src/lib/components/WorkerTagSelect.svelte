<script lang="ts">
	import { workerTags } from '$lib/stores'
	import { WorkerService } from '$lib/gen'

	export let tag: string | undefined
	export let noLabel: boolean = false
	export let nullTag: string | undefined = undefined

	loadWorkerGroups()

	async function loadWorkerGroups() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTags()
		}
	}
</script>

<div class="flex gap-1 items-center">
	{#if !noLabel}
		<div class="text-tertiary text-2xs">tag</div>
	{/if}
	<select
		class="min-w-32"
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
			<option value={undefined} disabled selected>{nullTag ? `default: ${nullTag}` : ''}</option>
		{/if}
		{#if tag && tag != '' && !($workerTags ?? []).includes(tag)}
			<option value={tag} selected>{tag}</option>
		{/if}
		{#each $workerTags ?? [] as tag (tag)}
			<option value={tag}>{tag}</option>
		{/each}
	</select>
</div>
