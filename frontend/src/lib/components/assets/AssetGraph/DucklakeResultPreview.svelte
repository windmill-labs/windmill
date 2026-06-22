<script lang="ts">
	// Live row preview for a ducklake table, used inside the materialize result
	// display (DisplayResult). Loads the table's column metadata then renders the
	// shared read-only DBTable grid (paged). Mirrors DataTablePreview but for the
	// `ducklake` input type — a ducklake catalog is always attachable, so there
	// is no "configure connection" branch.
	//
	// For a partitioned target the preview can scope to the just-written slice
	// ("This partition") or show the full table ("Whole table") via a toggle.
	import DBTable from '$lib/components/DBTable.svelte'
	import { resource } from 'runed'
	import { workspaceStore } from '$lib/stores'
	import { loadAllTablesMetaData } from '$lib/components/apps/components/display/dbtable/metadata'
	import { dbTableOpsWithPreviewScripts } from '$lib/components/dbOps'
	import type { DbInput } from '$lib/components/dbTypes'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import { AlertTriangle, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	interface Props {
		// Full asset URI, e.g. `ducklake://main/orders_daily`.
		assetUri: string
		// When set the target is partitioned and the preview can scope to this slice.
		partition?: string
		// Bump to force a re-fetch (parallel to DataTablePreview's refreshKey).
		refreshKey?: any
		class?: string
	}
	let { assetUri, partition, refreshKey, class: className = '' }: Props = $props()

	// Default to the just-written slice; `scoped` is a no-op without a partition
	// (the toggle is only shown when there is one).
	let scope = $state<'partition' | 'whole'>('partition')
	let scoped = $derived(!!partition && scope === 'partition')

	let input = $derived<DbInput | undefined>(parseDbInputFromAssetSyntax(assetUri) ?? undefined)
	let table = $derived(
		input && 'specificTable' in input ? (input.specificTable as string | undefined) : undefined
	)
	// Explicit schema from `ducklake://<lake>/<schema>.<table>` (default `main`).
	// The preview SELECT is `FROM <tableKey>` unquoted, so a `schema.table` key
	// resolves to the right schema — without this the read hits the default one.
	let schema = $derived(
		input && 'specificSchema' in input ? (input.specificSchema as string | undefined) : undefined
	)
	let tableKey = $derived(schema && table ? `${schema}.${table}` : table)

	// Scope rows to the partition slice via the preview SELECT's whereClause.
	// The value is single-quote-escaped (it's raw-injected server-side).
	let whereClause = $derived(
		scoped && partition ? `_wm_partition = '${partition.replaceAll("'", "''")}'` : undefined
	)

	let colDefs = resource(
		() => [input, refreshKey] as const,
		async ([_input]) => {
			if (!_input || !$workspaceStore) return undefined
			try {
				return await loadAllTablesMetaData($workspaceStore, _input)
			} catch {
				// A load failure reads the same as "table missing" from the preview's
				// POV; the underlying error is surfaced by loadAllTablesMetaData.
				return undefined
			}
		}
	)

	let tableColDefs = $derived.by(() => {
		const defs = colDefs.current
		if (!table || !defs) return undefined
		const direct = defs[`${schema ?? 'main'}.${table}`] ?? defs[table]
		const found =
			direct ??
			(() => {
				const key = Object.keys(defs).find((k) => k === table || k.endsWith(`.${table}`))
				return key ? defs[key] : undefined
			})()
		if (!found) return undefined
		// Hide the internal partition column when scoped to one partition (every
		// row has the same value); keep it in whole-table view so the rows from
		// different partitions are distinguishable.
		return scoped ? found.filter((c: { field?: string }) => c.field !== '_wm_partition') : found
	})

	let dbTableOps = $derived.by(() => {
		if (!(input && tableColDefs && tableKey && $workspaceStore)) return undefined
		const ops = dbTableOpsWithPreviewScripts({
			input,
			tableKey,
			colDefs: tableColDefs,
			workspace: $workspaceStore,
			whereClause
		})
		// Read-only preview: drop the mutation handlers so DBTable hides its
		// edit / delete / insert affordances — this is a result view, not a
		// table editor (and the table is overwritten on the next materialize).
		const readOnly = { ...ops }
		delete readOnly.onUpdate
		delete readOnly.onDelete
		delete readOnly.onInsert
		return readOnly
	})
</script>

<div class={twMerge('flex flex-col min-h-0 relative', className)}>
	{#if partition}
		<div class="pb-2">
			<ToggleButtonGroup selected={scope} on:selected={(e) => (scope = e.detail)}>
				{#snippet children({ item })}
					<ToggleButton size="sm" value="partition" label="This partition" {item} />
					<ToggleButton size="sm" value="whole" label="Whole table" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>
	{/if}
	{#if colDefs.loading && !colDefs.current}
		<div class="flex items-center justify-center p-4 text-tertiary">
			<Loader2 class="animate-spin" size={18} />
		</div>
	{:else if !tableColDefs}
		<div class="flex items-center gap-2 p-3 text-2xs text-tertiary">
			<AlertTriangle size={14} class="text-amber-500" />
			Couldn't load a preview of this table.
		</div>
	{:else if dbTableOps}
		<!-- Re-mount when the scope toggles so DBTable re-fetches with the new
		     whereClause / column set. -->
		{#key [refreshKey, scope]}
			<div class="grow min-h-0">
				<DBTable {dbTableOps} />
			</div>
		{/key}
	{/if}
</div>
