<script lang="ts">
	// Time-travel history for a ducklake table: lists the catalog's DuckLake
	// snapshots (commit log), newest first, and hands the user the exact
	// `AT (VERSION => n)` syntax to read any of them — copy a clause, copy a full
	// FROM, or open the version in the Query tab. This is the discoverability
	// surface: snapshots are captured automatically on every materialize, and
	// this teaches users they can pin reads against them.
	import { resource } from 'runed'
	import { Button } from '$lib/components/common'
	import { Loader2, RefreshCw, ClipboardCopy, Play } from 'lucide-svelte'
	import { fetchDucklakeSnapshots } from '$lib/components/dbOps'
	import { parseDbInputFromAssetSyntax, copyToClipboard } from '$lib/utils'

	interface Props {
		// The materialized ducklake asset path (`<ducklake>/<table>`).
		path: string
		workspace: string
		// Open the Query tab pinned to the chosen snapshot.
		onQueryVersion?: (version: number) => void
	}
	let { path, workspace, onQueryVersion }: Props = $props()

	let input = $derived(parseDbInputFromAssetSyntax(`ducklake://${path}`))
	let ducklake = $derived(input && 'ducklake' in input ? input.ducklake : undefined)
	let table = $derived(input && 'specificTable' in input ? input.specificTable : undefined)
	let schema = $derived(input && 'specificSchema' in input ? input.specificSchema : undefined)
	let tableKey = $derived(schema && table ? `${schema}.${table}` : table)

	let snapshots = resource([() => workspace, () => ducklake], async ([ws, dl]) => {
		if (!ws || !dl) return []
		return await fetchDucklakeSnapshots({ workspace: ws, ducklake: dl })
	})

	function atClause(version: number): string {
		return `AT (VERSION => ${version})`
	}
	function fromClause(version: number): string {
		// `lake` matches the alias the duckdb scaffold ATTACHes the ducklake under
		// (`ATTACH 'ducklake://…' AS lake`), so the copied clause is catalog-qualified
		// and pastes straight into a consumer script.
		return `FROM lake.${tableKey ?? 'table'} ${atClause(version)}`
	}
</script>

<div class="flex flex-col h-full">
	<div class="flex items-center justify-between gap-2 px-3 py-2 border-b shrink-0">
		<span class="text-xs font-semibold text-secondary">Snapshot history</span>
		<Button
			variant="subtle"
			unifiedSize="sm"
			startIcon={{ icon: RefreshCw }}
			iconOnly
			onclick={() => snapshots.refetch()}
			title="Refresh"
		/>
	</div>

	<div class="flex-1 min-h-0 overflow-auto p-3">
		{#if snapshots.loading}
			<div class="flex items-center gap-2 text-tertiary text-xs">
				<Loader2 size={14} class="animate-spin" /> Loading snapshots…
			</div>
		{:else if snapshots.error}
			<p class="text-xs text-red-600">Failed to load: {snapshots.error.message}</p>
		{:else if !snapshots.current?.length}
			<p class="text-xs text-secondary">
				No snapshots yet. DuckLake records one on every <span class="font-mono">// materialize</span
				> write; each becomes a version you can time-travel to.
			</p>
		{:else}
			<div class="flex flex-col gap-1.5">
				{#each snapshots.current as s (s.snapshot_id)}
					<div
						class="flex items-center justify-between gap-2 rounded border px-2 py-1.5 hover:bg-surface-hover"
					>
						<div class="flex flex-col min-w-0">
							<span class="text-xs font-mono">v{s.snapshot_id}</span>
							<span class="text-3xs text-tertiary">
								{s.snapshot_time ? new Date(s.snapshot_time).toLocaleString() : '—'}
							</span>
						</div>
						<div class="flex items-center gap-1 shrink-0">
							<Button
								variant="subtle"
								unifiedSize="sm"
								startIcon={{ icon: ClipboardCopy }}
								onclick={() => copyToClipboard(atClause(s.snapshot_id))}
								title="Copy AT (VERSION => …) clause"
							>
								AT (VERSION)
							</Button>
							<Button
								variant="subtle"
								unifiedSize="sm"
								startIcon={{ icon: ClipboardCopy }}
								iconOnly
								onclick={() => copyToClipboard(fromClause(s.snapshot_id))}
								title="Copy {fromClause(s.snapshot_id)}"
							/>
							<Button
								variant="default"
								unifiedSize="sm"
								startIcon={{ icon: Play }}
								onclick={() => onQueryVersion?.(s.snapshot_id)}
								title="Preview the table at this version"
							>
								Query
							</Button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
