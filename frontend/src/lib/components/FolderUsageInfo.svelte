<script lang="ts">
	import { FolderService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Cell from './table/Cell.svelte'

	export let name: string
	export let tabular = false
	export let order = ['scripts', 'flows', 'apps', 'schedules', 'variables', 'resources']

	$: $workspaceStore && loadUsage()

	let usage: Record<string, number> = {}

	async function loadUsage() {
		usage = await FolderService.getFolderUsage({ workspace: $workspaceStore!, name })
	}
</script>

{#if tabular}
	{#each order as key}
		<Cell class="w-20">
			{usage[key] ?? ''}
		</Cell>
	{/each}
{:else}
	<div class="flex flex-col text-xs text-secondary">
		{#each Object.entries(usage) as [k, v]}
			<div>
				{k}: {v}
			</div>
		{/each}
	</div>
{/if}
