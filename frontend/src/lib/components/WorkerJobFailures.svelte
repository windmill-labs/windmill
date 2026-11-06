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

	interface Props {
		stats: WorkerJobStats | undefined
		/** Label of the last-100 window, which sums per-worker windows for a group. */
		last100Label?: string
		/** All three windows side by side; otherwise only the 24h rate, the rest in a tooltip. */
		inline?: boolean
	}

	let { stats, last100Label = 'last 100 jobs', inline = false }: Props = $props()

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

{#if !stats}
	<span class="text-hint">--</span>
{:else if !inline}
	<!-- One figure per table row; the other windows are in the tooltip. -->
	<MeltTooltip>
		<span
			class="text-xs whitespace-nowrap {stats.last_24h.failed > 0
				? 'text-red-600 dark:text-red-400'
				: 'text-secondary'}"
		>
			{pct(stats.last_24h)}
		</span>
		{#snippet text()}
			{#each windows as w (w.short)}
				<div>{w.label}: {pct(w.counts)} ({w.counts.failed} failed of {w.counts.executed})</div>
			{/each}
		{/snippet}
	</MeltTooltip>
{:else}
	<div class="flex flex-row gap-3 text-xs">
		{#each windows as w (w.short)}
			<MeltTooltip>
				<div class="flex flex-row gap-1 items-baseline whitespace-nowrap">
					<span class="text-hint text-2xs w-6">{w.short}</span>
					<span class={w.counts.failed > 0 ? 'text-red-600 dark:text-red-400' : 'text-secondary'}>
						{pct(w.counts)}
					</span>
				</div>
				{#snippet text()}
					{w.label}: {w.counts.failed} failed of {w.counts.executed} jobs
				{/snippet}
			</MeltTooltip>
		{/each}
	</div>
{/if}
