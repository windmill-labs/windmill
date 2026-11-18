<script lang="ts">
	// Time-travel snapshot list for a ducklake table (presentational): the
	// catalog's DuckLake snapshots, newest first. Selecting a row drives the
	// adjacent preview pane via `onSelect`; the selected row is highlighted. The
	// list data + selection default are owned by the parent; the preview pane
	// carries the copy-able `FROM lake.<table> AT (VERSION => n)` clause.
	import { Button } from '$lib/components/common'
	import { Loader2, RefreshCw } from 'lucide-svelte'
	import type { DucklakeSnapshot } from '$lib/components/dbOps'

	interface Props {
		items: DucklakeSnapshot[]
		loading: boolean
		error?: string
		onRefresh?: () => void
		// Snapshot currently shown in the preview pane (highlighted in the list).
		selectedVersion?: number
		// Select a snapshot to preview.
		onSelect?: (version: number) => void
	}
	let { items, loading, error, onRefresh, selectedVersion, onSelect }: Props = $props()

	// DuckLake serializes `snapshot_time` as microseconds-since-epoch (a numeric
	// string), not an ISO date — `new Date(µs)` would be Invalid Date. Detect the
	// µs magnitude and convert to ms; fall back to direct parsing otherwise.
	function fmtSnapshotTime(t: string | number | undefined): string {
		if (t == undefined || t === '') return '—'
		const n = typeof t === 'number' ? t : Number(t)
		const d = Number.isFinite(n) && n > 1e14 ? new Date(n / 1000) : new Date(t)
		return isNaN(d.getTime()) ? String(t) : d.toLocaleString()
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
			onclick={() => onRefresh?.()}
			title="Refresh"
		/>
	</div>

	<div class="flex-1 min-h-0 overflow-auto p-3">
		{#if loading && !items.length}
			<div class="flex items-center gap-2 text-tertiary text-xs">
				<Loader2 size={14} class="animate-spin" /> Loading snapshots…
			</div>
		{:else if error}
			<p class="text-xs text-red-600">Failed to load: {error}</p>
		{:else if !items.length}
			<p class="text-xs text-secondary">
				No snapshots yet. DuckLake records one on every <span class="font-mono"
					>// materialize</span
				> write; each becomes a version you can time-travel to.
			</p>
		{:else}
			<div class="flex flex-col gap-1.5">
				{#each items as s (s.snapshot_id)}
					{@const selected = s.snapshot_id === selectedVersion}
					<button
						type="button"
						class="flex items-center justify-between gap-2 rounded border px-2 py-1.5 text-left {selected
							? 'border-blue-400 bg-blue-50 dark:border-blue-500 dark:bg-blue-950/30'
							: 'hover:bg-surface-hover'}"
						onclick={() => onSelect?.(s.snapshot_id)}
						title="Preview the table at this version"
					>
						<span class="text-xs font-mono">v{s.snapshot_id}</span>
						<span class="text-3xs text-tertiary shrink-0">{fmtSnapshotTime(s.snapshot_time)}</span>
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>
