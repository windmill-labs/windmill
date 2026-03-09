<script lang="ts">
	import { run } from 'svelte/legacy';

	import { FolderService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Cell from './table/Cell.svelte'

	interface Props {
		name: string;
		tabular?: boolean;
		order?: any;
	}

	let { name, tabular = false, order = ['scripts', 'flows', 'apps', 'schedules', 'variables', 'resources'] }: Props = $props();


	let usage: Record<string, number> = $state({})

	async function loadUsage() {
		usage = await FolderService.getFolderUsage({ workspace: $workspaceStore!, name })
	}
	run(() => {
		$workspaceStore && loadUsage()
	});
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
