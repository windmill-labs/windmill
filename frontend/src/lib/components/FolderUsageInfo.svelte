<script lang="ts">
	import { FolderService, GroupService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let name: string

	$: $workspaceStore && loadUsage()

	let usage: Record<string, number> = {}

	async function loadUsage() {
		usage = await FolderService.getFolderUsage({ workspace: $workspaceStore!, name })
	}
</script>

<div class="flex flex-col text-xs text-gray-600">
	{#each Object.entries(usage) as [k, v]}
		<div>
			{k}: {v}
		</div>
	{/each}
</div>
