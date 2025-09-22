<script lang="ts">
	import type { SyncResponse } from '$lib/git-sync'

	let { previewResult } = $props<{
		previewResult: SyncResponse | undefined
	}>()

	let added = $derived(previewResult?.changes?.filter(c => c.type === 'added').map(c => c.path) || [])
	let deleted = $derived(previewResult?.changes?.filter(c => c.type === 'deleted').map(c => c.path) || [])
	let edited = $derived(previewResult?.changes?.filter(c => c.type === 'edited').map(c => c.path) || [])
</script>

<div class="border rounded p-2 text-xs max-h-40 overflow-y-auto bg-surface-secondary">
	<div class="font-semibold text-[11px] mb-1 text-tertiary">Preview of changes:</div>

	{#if !added.length && !deleted.length && !edited.length}
		<div class="mt-2 text-tertiary">No changes found! The workspace is up to date.</div>
	{:else}
		{#if added.length}
			<div class="mt-2">
				<div class="text-green-600">Added:</div>
				<ul class="list-disc list-inside">
					{#each added as file}
						<li>
							{file}{!file.includes('.') ? ' (dir)' : ''}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if deleted.length}
			<div class="mt-2">
				<div class="text-red-600">Deleted:</div>
				<ul class="list-disc list-inside">
					{#each deleted as file}
						<li>
							{file}{!file.includes('.') ? ' (dir)' : ''}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if edited.length}
			<div class="mt-2">
				<div class="text-yellow-600">Edited:</div>
				<ul class="list-disc list-inside">
					{#each edited as file}
						<li>
							{file}{!file.includes('.') ? ' (dir)' : ''}
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	{/if}
</div>
