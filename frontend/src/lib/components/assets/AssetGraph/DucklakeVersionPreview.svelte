<script lang="ts">
	// Read-only grid preview of a ducklake table AT a chosen DuckLake snapshot
	// ("time-travel"). Mirrors DucklakeResultPreview but, instead of scoping to a
	// partition, pins every read to a catalog version via `AT (VERSION => n)`
	// (threaded server-side through the SELECT/COUNT markers). This is the
	// "query this version" scratchpad — the rendered SQL the user would write by
	// hand is shown above the grid so the affordance is self-documenting.
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
		// DuckLake snapshot to pin reads to. Undefined renders nothing (the
		// parent prompts the user to pick a version from the history first).
		version?: number
		class?: string
	}
	let { assetUri, version, class: className = '' }: Props = $props()

	let input = $derived<DbInput | undefined>(parseDbInputFromAssetSyntax(assetUri) ?? undefined)
	let table = $derived(
		input && 'specificTable' in input ? (input.specificTable as string | undefined) : undefined
	)
	let schema = $derived(
		input && 'specificSchema' in input ? (input.specificSchema as string | undefined) : undefined
	)
	let tableKey = $derived(schema && table ? `${schema}.${table}` : table)

	// The hand-written equivalent of what this preview runs — surfaced so the
	// user learns the syntax they can paste into their own consumer SQL.
	let exampleSql = $derived(
		version != undefined && tableKey ? `FROM ${tableKey} AT (VERSION => ${version})` : undefined
	)

	let colDefs = resource(
		// Column metadata is the same across snapshots — keyed on `input` only so
		// switching versions doesn't re-dispatch a metadata preview job (the grid
		// itself re-reads via `dbTableOps` + the `{#key version}` block).
		() => [input] as const,
		async ([_input]) => {
			if (!_input || !$workspaceStore) return undefined
			try {
				return await loadAllTablesMetaData($workspaceStore, _input)
			} catch {
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
		return found
	})

	let dbTableOps = $derived.by(() => {
		if (!(input && tableColDefs && tableKey && $workspaceStore && version != undefined))
			return undefined
		const ops = dbTableOpsWithPreviewScripts({
			input,
			tableKey,
			colDefs: tableColDefs,
			workspace: $workspaceStore,
			version
		})
		// Historical reads are immutable: drop every mutation handler so DBTable
		// renders without edit/delete/insert affordances.
		const readOnly = { ...ops }
		delete readOnly.onUpdate
		delete readOnly.onDelete
		delete readOnly.onInsert
		return readOnly
	})
</script>

<div class={twMerge('flex flex-col min-h-0 relative', className)}>
	{#if version == undefined}
		<div class="flex items-center gap-2 p-3 text-2xs text-tertiary">
			Pick a version from the History tab to query the table as of that snapshot.
		</div>
	{:else}
		{#if exampleSql}
			<div class="pb-2 text-2xs font-mono text-tertiary truncate" title={exampleSql}>
				{exampleSql}
			</div>
		{/if}
		{#if colDefs.loading && !colDefs.current}
			<div class="flex items-center justify-center p-4 text-tertiary">
				<Loader2 class="animate-spin" size={18} />
			</div>
		{:else if !tableColDefs}
			<div class="flex items-center gap-2 p-3 text-2xs text-tertiary">
				<AlertTriangle size={14} class="text-amber-500" />
				Couldn't load this table at version {version}.
			</div>
		{:else if dbTableOps}
			{#key version}
				<div class="grow min-h-0">
					<DBTable {dbTableOps} />
				</div>
			{/key}
		{/if}
	{/if}
</div>
