<script lang="ts">
	// What a dbt invocation did, per node. The payload is already structured (the
	// worker settles it from `run_results.json`); this renders it as the run
	// summary a dbt user reads, instead of leaving them to scroll the JSON or
	// count PASS/WARN lines in the log.
	import { CheckCircle2, XCircle, AlertTriangle, MinusCircle, FlaskConical } from 'lucide-svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import {
		splitUniqueId as split,
		splitRelation,
		statusRank as rank,
		type DbtOutcome,
		type DbtRun
	} from '$lib/components/dbt/parseDbtRun'

	let { run }: { run: DbtRun } = $props()

	// Failures first, then warnings, then skips: the run's outcome is decided by
	// a handful of nodes and a long green list buries them.
	let nodes = $derived([...(run.nodes ?? [])].sort((a, b) => rank(a.status, a.outcome) - rank(b.status, b.outcome)))
	let totals = $derived(run.totals ?? {})
	let hasTests = $derived(nodes.some((n) => split(n.unique_id).kind === 'test'))

	function statusClass(status: string, outcome?: DbtOutcome): string {
		switch (rank(status, outcome)) {
			case 0:
				return 'text-red-600 dark:text-red-400'
			case 1:
				return 'text-yellow-600 dark:text-yellow-400'
			case 2:
				return 'text-secondary'
			default:
				return 'text-green-600 dark:text-green-400'
		}
	}

	function fmtTime(s: number | undefined): string {
		if (s === undefined) return ''
		return s < 1 ? `${Math.round(s * 1000)}ms` : `${s.toFixed(2)}s`
	}

	// dbt qualifies every part. The database is dropped only when every row shares
	// it, which is the common case and the noisiest to repeat — a model that
	// overrode `database` would otherwise render identically to a same-named
	// relation in another catalog.
	let oneDatabase = $derived(
		new Set(
			nodes
				.map((n) => n.relation_name)
				.filter((r): r is string => !!r)
				.map((r) => splitRelation(r).slice(0, -2).join('.'))
		).size <= 1
	)
	function fmtRelation(relation: string | undefined): string | undefined {
		if (!relation) return undefined
		const parts = splitRelation(relation)
		return (oneDatabase ? parts.slice(-2) : parts).join('.')
	}
</script>

<div class="w-full flex flex-col gap-2 mb-2">
	<div class="flex items-center flex-wrap gap-x-3 gap-y-1 text-xs">
		{#each [{ k: 'success', label: 'passed', cls: 'text-green-600 dark:text-green-400' }, { k: 'warn', label: 'warned', cls: 'text-yellow-600 dark:text-yellow-400' }, { k: 'error', label: 'failed', cls: 'text-red-600 dark:text-red-400' }, { k: 'skipped', label: 'skipped', cls: 'text-secondary' }] as t (t.k)}
			{@const n = (totals as Record<string, number | undefined>)[t.k] ?? 0}
			{#if n > 0}
				<span class={t.cls}><span class="font-semibold">{n}</span> {t.label}</span>
			{/if}
		{/each}
		<span class="text-secondary">of {totals.total ?? nodes.length} nodes</span>
		<span class="ml-auto text-secondary font-mono text-2xs">
			{run.command ?? 'build'} · {run.engine ?? ''}
			{run.engine_version ?? ''}
		</span>
	</div>

	{#if nodes.length > 0}
		<div class="border rounded overflow-hidden">
			<table class="w-full text-xs">
				<thead class="bg-surface-secondary text-secondary">
					<tr>
						<th class="text-left font-normal px-2 py-1">Node</th>
						<th class="text-left font-normal px-2 py-1 w-24">Kind</th>
						<th class="text-left font-normal px-2 py-1">Relation</th>
						<th class="text-right font-normal px-2 py-1 w-20">Rows</th>
						<th class="text-right font-normal px-2 py-1 w-20">Time</th>
					</tr>
				</thead>
				<tbody>
					{#each nodes as node (node.unique_id)}
						{@const s = split(node.unique_id)}
						{@const r = rank(node.status, node.outcome)}
						<tr class="border-t {r < 2 ? 'bg-surface-secondary/40' : ''}">
							<td class="px-2 py-1">
								<div class="flex items-center gap-1.5 min-w-0">
									<span class={statusClass(node.status, node.outcome)}>
										{#if r === 0}
											<XCircle size={13} />
										{:else if r === 1}
											<AlertTriangle size={13} />
										{:else if r === 2}
											<MinusCircle size={13} />
										{:else}
											<CheckCircle2 size={13} />
										{/if}
									</span>
									<span class="font-mono truncate">{s.name}</span>
									{#if node.message && r < 2}
										<Tooltip>{node.message}</Tooltip>
									{/if}
								</div>
								{#if node.message && r < 2}
									<div class="pl-5 text-2xs {statusClass(node.status, node.outcome)} truncate">
										{node.message}
									</div>
								{/if}
							</td>
							<td class="px-2 py-1 text-secondary">
								<span class="inline-flex items-center gap-1">
									{#if s.kind === 'test'}
										<FlaskConical size={11} />
									{/if}
									{s.kind}
								</span>
							</td>
							<td class="px-2 py-1 font-mono text-secondary truncate">
								{fmtRelation(node.relation_name) ?? ''}
							</td>
							<td class="px-2 py-1 text-right tabular-nums text-secondary">
								{node.rows_affected ?? ''}
							</td>
							<td class="px-2 py-1 text-right tabular-nums text-secondary">
								{fmtTime(node.execution_time)}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
		{#if hasTests}
			<div class="text-2xs text-secondary">
				A test's severity decides the outcome: dbt's own <span class="font-mono">warn</span> surfaces
				without failing the job.
			</div>
		{/if}
	{/if}
</div>
