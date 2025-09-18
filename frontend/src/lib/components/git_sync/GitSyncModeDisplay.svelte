<script lang="ts">
	import type { GitSyncRepository } from './GitSyncContext.svelte'

	let {
		mode,
		targetBranch,
		repository
	} = $props<{
		mode?: 'sync' | 'promotion' | null
		targetBranch: string
		repository?: GitSyncRepository | null
	}>()
</script>

<div class="text-base">
	{#if mode === 'promotion'}
		<div><span class="font-bold">Promotion:</span> Creating branches whose promotion target is {targetBranch}</div>
		{#if repository?.group_by_folder}
			<div class="text-sm text-tertiary mt-1">Grouped by folder</div>
		{/if}
	{:else}
		<div><span class="font-bold">Sync:</span> Syncing back to branch {targetBranch}</div>
	{/if}
</div>