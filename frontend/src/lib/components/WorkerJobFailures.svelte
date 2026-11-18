<script lang="ts" module>
	import type { WorkerJobCounts, WorkerPing } from '$lib/gen'

	export type WorkerJobStats = NonNullable<WorkerPing['job_stats']>

	/** Sums the counts of several workers, e.g. a worker group. */
	export function sumJobStats(stats: (WorkerJobStats | undefined)[]): WorkerJobStats | undefined {
		const present = stats.filter((s): s is WorkerJobStats => s != undefined)
		if (present.length == 0) return undefined
		const add = (a: WorkerJobCounts, b: WorkerJobCounts) => ({
			executed: a.executed + b.executed,
			failed: a.failed + b.failed
		})
		const zero = { executed: 0, failed: 0 }
		return present.reduce(
			(acc, s) => ({
				total: add(acc.total, s.total),
				last_24h: add(acc.last_24h, s.last_24h),
				last_100: add(acc.last_100, s.last_100)
			}),
			{ total: zero, last_24h: zero, last_100: zero }
		)
	}
</script>

<script lang="ts">
	import MeltTooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		stats: WorkerJobStats | undefined
		/** Label of the last-100 window, which sums per-worker windows for a group. */
		last100Label?: string
		/** Bar beside `children` rather than under it. */
		row?: boolean
		children?: Snippet
	}

	let { stats, last100Label = 'last 100 jobs', row = false, children }: Props = $props()

	function failedPct(c: WorkerJobCounts): number {
		return c.executed == 0 ? 0 : (c.failed / c.executed) * 100
	}

	function pct(c: WorkerJobCounts): string {
		if (c.executed == 0) return '--'
		const p = (c.failed / c.executed) * 100
		if (p > 0 && p < 0.1) return '<0.1%'
		return (p >= 10 || p == 0 ? Math.round(p) : p.toFixed(1)) + '%'
	}

	const windows = $derived(
		stats
			? [
					{ label: '24h', short: '24h', counts: stats.last_24h },
					{ label: last100Label, short: '100', counts: stats.last_100 },
					{ label: 'ever', short: 'ever', counts: stats.total }
				]
			: []
	)
</script>

<!-- A grey track with the 24h failed share in red next to `children`; the other windows are in
     the tooltip. Healthy workers show an all-grey track. -->
{#if !stats}
	{@render children?.()}
{:else}
	{@const failedShare = failedPct(stats.last_24h)}
	<MeltTooltip>
		<div class="flex w-fit {row ? 'flex-row items-center gap-2' : 'flex-col gap-1'}">
			{@render children?.()}
			<div class="h-1.5 w-12 rounded-full bg-surface-secondary border overflow-hidden">
				{#if failedShare > 0}
					<div
						class="h-full bg-red-500 dark:bg-red-400"
						style="width: {Math.max(failedShare, 8)}%"
					></div>
				{/if}
			</div>
		</div>
		{#snippet text()}
			<div class="font-semibold">Failed jobs</div>
			{#each windows as w (w.short)}
				<div>{w.label}: {pct(w.counts)} ({w.counts.failed} failed of {w.counts.executed})</div>
			{/each}
			<div class="text-hint">Flows are not counted; canceled jobs are not failures.</div>
		{/snippet}
	</MeltTooltip>
{/if}
