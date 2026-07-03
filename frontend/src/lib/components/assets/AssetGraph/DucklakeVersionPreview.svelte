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
	import {
		fetchDucklakeColumnsAtVersion,
		dbTableOpsWithPreviewScripts
	} from '$lib/components/dbOps'
	import type { DbInput } from '$lib/components/dbTypes'
	import { parseDbInputFromAssetSyntax, copyToClipboard } from '$lib/utils'
	import { AlertTriangle, Loader2, ClipboardCopy } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
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
	let ducklake = $derived(input?.type === 'ducklake' ? input.ducklake : undefined)
	let table = $derived(
		input && 'specificTable' in input ? (input.specificTable as string | undefined) : undefined
	)
	let schema = $derived(
		input && 'specificSchema' in input ? (input.specificSchema as string | undefined) : undefined
	)
	let tableKey = $derived(schema && table ? `${schema}.${table}` : table)

	// A paste-able consumer-script form of this read — `lake` matches the alias
	// the duckdb scaffold ATTACHes the ducklake under, so the reference is
	// catalog-qualified — surfaced so the user learns the time-travel syntax.
	let exampleSql = $derived(
		version != undefined && tableKey
			? `FROM lake.${tableKey} AT (VERSION => ${version})`
			: undefined
	)

	// Load the column set *at the pinned version* — a table's schema can differ
	// across snapshots, so enumerating the current columns against an older
	// version would reference columns that didn't exist then and break the read.
	// The result carries the version it was loaded for so the read never uses a
	// stale column set from the previously-viewed snapshot (the resource keeps
	// the prior value while re-fetching).
	let columns = resource(
		() => [ducklake, tableKey, version] as const,
		async ([_ducklake, _tableKey, _version]) => {
			if (!_ducklake || !_tableKey || _version == undefined || !$workspaceStore) return undefined
			const colDefs = await fetchDucklakeColumnsAtVersion({
				workspace: $workspaceStore,
				ducklake: _ducklake,
				tableKey: _tableKey,
				version: _version
			})
			return { version: _version, colDefs }
		}
	)
	// Only ready once the loaded columns are for the version currently shown.
	let ready = $derived(
		columns.current?.version === version && (columns.current?.colDefs.length ?? 0) > 0
	)
	let tableColDefs = $derived(ready ? columns.current!.colDefs : undefined)

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
			Select a snapshot from the list to preview the table as of that version.
		</div>
	{:else}
		{#if exampleSql}
			<div class="flex items-center gap-1 pb-2">
				<span class="text-2xs font-mono text-tertiary truncate" title={exampleSql}>
					{exampleSql}
				</span>
				<Button
					variant="subtle"
					unifiedSize="sm"
					startIcon={{ icon: ClipboardCopy }}
					iconOnly
					onclick={() => copyToClipboard(exampleSql)}
					title="Copy {exampleSql}"
				/>
			</div>
		{/if}
		{#if ready && dbTableOps}
			{#key version}
				<div class="grow min-h-0">
					<DBTable {dbTableOps} />
				</div>
			{/key}
		{:else if columns.error}
			<div class="flex items-center gap-2 p-3 text-2xs text-tertiary">
				<AlertTriangle size={14} class="text-amber-500" />
				Couldn't load this table at version {version}.
			</div>
		{:else}
			<div class="flex items-center justify-center p-4 text-tertiary">
				<Loader2 class="animate-spin" size={18} />
			</div>
		{/if}
	{/if}
</div>
