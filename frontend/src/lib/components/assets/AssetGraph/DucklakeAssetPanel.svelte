<script lang="ts">
	// Top pane for a selected ducklake asset: tabs over the three views of a
	// versioned table — materialized partitions (status grid), snapshot history
	// (time-travel surface), and a read-only "query at version" scratchpad. The
	// History tab feeds the Query tab a chosen snapshot via shared state.
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Table2, History, Play } from 'lucide-svelte'
	import PartitionStatusGrid from './PartitionStatusGrid.svelte'
	import DucklakeSnapshotHistory from './DucklakeSnapshotHistory.svelte'
	import DucklakeVersionPreview from './DucklakeVersionPreview.svelte'

	interface Props {
		// The materialized ducklake asset path (`<ducklake>/<table>`).
		path: string
		workspace: string
	}
	let { path, workspace }: Props = $props()

	let tab = $state<'partitions' | 'history' | 'query'>('partitions')
	// Snapshot the Query tab reads at — set when the user clicks "Query" on a
	// history row.
	let selectedVersion = $state<number | undefined>(undefined)

	function onQueryVersion(version: number) {
		selectedVersion = version
		tab = 'query'
	}
</script>

<div class="flex flex-col h-full">
	<div class="flex items-center gap-2 px-3 py-2 border-b shrink-0">
		<ToggleButtonGroup selected={tab} on:selected={(e) => (tab = e.detail)}>
			{#snippet children({ item })}
				<ToggleButton size="sm" value="partitions" label="Partitions" icon={Table2} {item} />
				<ToggleButton size="sm" value="history" label="History" icon={History} {item} />
				<ToggleButton size="sm" value="query" label="Query" icon={Play} {item} />
			{/snippet}
		</ToggleButtonGroup>
	</div>

	<div class="flex-1 min-h-0">
		{#if tab === 'partitions'}
			<PartitionStatusGrid {path} {workspace} />
		{:else if tab === 'history'}
			<DucklakeSnapshotHistory {path} {workspace} {onQueryVersion} />
		{:else}
			<div class="h-full p-3 overflow-auto">
				<DucklakeVersionPreview
					assetUri={`ducklake://${path}`}
					version={selectedVersion}
					class="h-full"
				/>
			</div>
		{/if}
	</div>
</div>
