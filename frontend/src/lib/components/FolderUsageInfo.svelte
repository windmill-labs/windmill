<script lang="ts">
	import { FolderService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

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
		<td class="text-center">
			{usage[key] ?? ''}
		</td>
	{/each}
{:else}
	<div class="flex flex-col text-xs text-gray-600">
		{#each Object.entries(usage) as [k, v]}
			<div>
				{k}: {v}
			</div>
		{/each}
	</div>
{/if}
