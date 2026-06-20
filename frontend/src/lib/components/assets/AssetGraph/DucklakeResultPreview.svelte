<script lang="ts">
	// Live row preview for a ducklake table, used inside the materialize result
	// display (DisplayResult). Loads the table's column metadata then renders the
	// shared read-only DBTable grid (paged). Mirrors DataTablePreview but for the
	// `ducklake` input type — a ducklake catalog is always attachable, so there
	// is no "configure connection" branch.
	import DBTable from '$lib/components/DBTable.svelte'
	import { resource } from 'runed'
	import { workspaceStore } from '$lib/stores'
	import { loadAllTablesMetaData } from '$lib/components/apps/components/display/dbtable/metadata'
	import { dbTableOpsWithPreviewScripts } from '$lib/components/dbOps'
	import type { DbInput } from '$lib/components/dbTypes'
	import { parseDbInputFromAssetSyntax } from '$lib/utils'
	import { AlertTriangle, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		// Full asset URI, e.g. `ducklake://main/orders_daily`.
		assetUri: string
		// Bump to force a re-fetch (parallel to DataTablePreview's refreshKey).
		refreshKey?: any
		class?: string
	}
	let { assetUri, refreshKey, class: className = '' }: Props = $props()

	let input = $derived<DbInput | undefined>(parseDbInputFromAssetSyntax(assetUri) ?? undefined)
	let table = $derived(
		input && 'specificTable' in input ? (input.specificTable as string | undefined) : undefined
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

	// loadAllTablesMetaData keys columns by `schema.table` or bare `table`. The
	// ducklake default schema is `main`; fall back to the bare key and finally to
	// any schema-qualified key matching the table name.
	let tableColDefs = $derived.by(() => {
		const defs = colDefs.current
		if (!table || !defs) return undefined
		const direct = defs[`main.${table}`] ?? defs[table]
		if (direct) return direct
		const key = Object.keys(defs).find((k) => k === table || k.endsWith(`.${table}`))
		return key ? defs[key] : undefined
	})

	let dbTableOps = $derived.by(() => {
		if (!(input && tableColDefs && table && $workspaceStore)) return undefined
		const ops = dbTableOpsWithPreviewScripts({
			input,
			tableKey: table,
			colDefs: tableColDefs,
			workspace: $workspaceStore
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
		{#key refreshKey}
			<DBTable {dbTableOps} />
		{/key}
	{/if}
</div>
