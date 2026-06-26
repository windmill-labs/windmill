<script lang="ts">
	// Top pane for a selected ducklake asset: tabs over the views of a versioned
	// table. "Partitions" is the materialization status grid; "History" is a
	// master-detail time-travel surface — the snapshot list on the left, a
	// read-only preview of the table at the selected snapshot on the right.
	//
	// The snapshot list is fetched here (not in the list child) so both the list
	// and the preview share one source of truth: `effectiveVersion` derives to
	// the user's pick, or the newest snapshot until they pick one. The list is
	// scoped to the table (catalog-wide snapshots predating the table's creation
	// can't be read), so a selectable version always exists in the table.
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { Table2, History, Columns3 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { resource } from 'runed'
	import { fetchDucklakeSnapshots } from '$lib/components/dbOps'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import PartitionStatusGrid from './PartitionStatusGrid.svelte'
	import DucklakeSnapshotHistory from './DucklakeSnapshotHistory.svelte'
	import DucklakeVersionPreview from './DucklakeVersionPreview.svelte'
	import SchemaHistoryPanel from './SchemaHistoryPanel.svelte'

	interface Props {
		// The materialized ducklake asset path (`<ducklake>/<table>`).
		path: string
		workspace: string
		// Whether this asset's schema can evolve (whole-table `replace` producer);
		// drives the Schema tab between version history and a single fixed schema.
		schemaCanEvolve?: boolean
	}
	let { path, workspace, schemaCanEvolve = true }: Props = $props()

	let tab = $state<'partitions' | 'history' | 'schema'>('partitions')
	// The user's explicit snapshot pick (undefined until they click a row).
	let selectedVersion = $state<number | undefined>(undefined)

	let parsed = $derived(parseDbInputFromAssetSyntax(`ducklake://${path}`))
	let ducklake = $derived(parsed && 'ducklake' in parsed ? parsed.ducklake : undefined)
	// Schema-qualified table name (schema defaults to `main`) used to scope the
	// snapshot list to versions where the table exists.
	let qualifiedTable = $derived.by(() => {
		if (!parsed || !('specificTable' in parsed) || !parsed.specificTable) return undefined
		const schema = 'specificSchema' in parsed ? parsed.specificSchema : undefined
		return `${schema ?? 'main'}.${parsed.specificTable}`
	})

	let snapshots = resource(
		[() => workspace, () => ducklake, () => qualifiedTable],
		async ([ws, dl, table]) => {
			if (!ws || !dl) return []
			return await fetchDucklakeSnapshots({ workspace: ws, ducklake: dl, table })
		}
	)
	// Newest snapshot is the default preview until the user picks one. If the
	// pick is no longer in the list (snapshots refetched, or it was stale from a
	// previously-viewed asset), fall back to newest rather than reading a version
	// that isn't in this table.
	let effectiveVersion = $derived.by(() => {
		const list = snapshots.current
		const picked = list?.some((s) => s.snapshot_id === selectedVersion)
		return picked ? selectedVersion : list?.[0]?.snapshot_id
	})

	// Keep the list pane to a roughly fixed width rather than a fixed fraction,
	// so it stays compact on wide panels instead of sprawling. Falls back to a
	// usable fraction on narrow ones and before the width is measured.
	let paneWidth = $state(0)
	let listSize = $derived(
		paneWidth > 0 ? Math.max(24, Math.min(46, Math.round((260 / paneWidth) * 100))) : 36
	)
</script>

<div class="flex flex-col h-full">
	{#if !qualifiedTable}
		<!-- Catalog-level ducklake node (no table segment, e.g. `ducklake://main`):
		     snapshot history / time-travel are per-table, so only the partition
		     grid applies here. -->
		<PartitionStatusGrid {path} {workspace} />
	{:else}
		<div class="flex items-center gap-2 px-3 py-2 border-b shrink-0">
			<ToggleButtonGroup selected={tab} on:selected={(e) => (tab = e.detail)}>
				{#snippet children({ item })}
					<ToggleButton size="sm" value="partitions" label="Partitions" icon={Table2} {item} />
					<ToggleButton size="sm" value="schema" label="Schema" icon={Columns3} {item} />
					<ToggleButton size="sm" value="history" label="History" icon={History} {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>

		<div class="flex-1 min-h-0">
			{#if tab === 'partitions'}
				<PartitionStatusGrid {path} {workspace} />
			{:else if tab === 'schema'}
				<SchemaHistoryPanel {path} {workspace} canEvolve={schemaCanEvolve} />
			{:else}
				<div class="h-full" bind:clientWidth={paneWidth}>
					<Splitpanes class="!h-full">
						<Pane size={listSize} minSize={20}>
							<DucklakeSnapshotHistory
								items={snapshots.current ?? []}
								loading={snapshots.loading}
								error={snapshots.error?.message}
								onRefresh={() => snapshots.refetch()}
								selectedVersion={effectiveVersion}
								onSelect={(v) => (selectedVersion = v)}
							/>
						</Pane>
						<Pane size={100 - listSize} minSize={30}>
							<div class="h-full p-3 overflow-auto">
								<DucklakeVersionPreview
									assetUri={`ducklake://${path}`}
									version={effectiveVersion}
									class="h-full"
								/>
							</div>
						</Pane>
					</Splitpanes>
				</div>
			{/if}
		</div>
	{/if}
</div>
