<script lang="ts">
	// Top pane for a selected ducklake asset: tabs over the views of a versioned
	// table. "Partitions" is the materialization status grid; "History" is a
	// master-detail time-travel surface — the snapshot list on the left, a
	// read-only preview of the table at the selected snapshot on the right.
	//
	// The snapshot list is fetched here (not in the list child) so both the list
	// and the preview share one source of truth: `effectiveVersion` derives to
	// the user's pick, or the newest snapshot until they pick one.
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Table2, History } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { resource } from 'runed'
	import { fetchDucklakeSnapshots } from '$lib/components/dbOps'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import PartitionStatusGrid from './PartitionStatusGrid.svelte'
	import DucklakeSnapshotHistory from './DucklakeSnapshotHistory.svelte'
	import DucklakeVersionPreview from './DucklakeVersionPreview.svelte'

	interface Props {
		// The materialized ducklake asset path (`<ducklake>/<table>`).
		path: string
		workspace: string
	}
	let { path, workspace }: Props = $props()

	let tab = $state<'partitions' | 'history'>('partitions')
	// The user's explicit snapshot pick (undefined until they click a row).
	let selectedVersion = $state<number | undefined>(undefined)

	let ducklake = $derived.by(() => {
		const i = parseDbInputFromAssetSyntax(`ducklake://${path}`)
		return i && 'ducklake' in i ? i.ducklake : undefined
	})
	let snapshots = resource(
		[() => workspace, () => ducklake],
		async ([ws, dl]) => {
			if (!ws || !dl) return []
			return await fetchDucklakeSnapshots({ workspace: ws, ducklake: dl })
		}
	)
	// Newest snapshot is the default preview until the user selects one.
	let effectiveVersion = $derived(selectedVersion ?? snapshots.current?.[0]?.snapshot_id)
</script>

<div class="flex flex-col h-full">
	<div class="flex items-center gap-2 px-3 py-2 border-b shrink-0">
		<ToggleButtonGroup selected={tab} on:selected={(e) => (tab = e.detail)}>
			{#snippet children({ item })}
				<ToggleButton size="sm" value="partitions" label="Partitions" icon={Table2} {item} />
				<ToggleButton size="sm" value="history" label="History" icon={History} {item} />
			{/snippet}
		</ToggleButtonGroup>
	</div>

	<div class="flex-1 min-h-0">
		{#if tab === 'partitions'}
			<PartitionStatusGrid {path} {workspace} />
		{:else}
			<Splitpanes class="!h-full">
				<Pane size={42} minSize={25}>
					<DucklakeSnapshotHistory
						items={snapshots.current ?? []}
						loading={snapshots.loading}
						error={snapshots.error?.message}
						onRefresh={() => snapshots.refetch()}
						selectedVersion={effectiveVersion}
						onSelect={(v) => (selectedVersion = v)}
					/>
				</Pane>
				<Pane size={58} minSize={30}>
					<div class="h-full p-3 overflow-auto">
						<DucklakeVersionPreview
							assetUri={`ducklake://${path}`}
							version={effectiveVersion}
							class="h-full"
						/>
					</div>
				</Pane>
			</Splitpanes>
		{/if}
	</div>
</div>
