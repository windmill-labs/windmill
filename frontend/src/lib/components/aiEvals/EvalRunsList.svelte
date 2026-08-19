<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { Bot, Code2, Loader2 } from 'lucide-svelte'
	import type { EvalDataset, EvalExperiment, ExperimentScore } from '$lib/gen'
	import { formatScore } from './evalScorers'
	import { experimentName, subjectLabel } from './evalRuns'

	let {
		experiments,
		datasets,
		onOpen,
		onEditDataset
	}: {
		/** Every run of this agent, newest first, whichever dataset each is of. */
		experiments: EvalExperiment[]
		/** The workspace's datasets, for naming the one a run is of by what it is for. */
		datasets: EvalDataset[]
		onOpen: (experiment: EvalExperiment) => void
		onEditDataset: (dataset: string) => void
	} = $props()

	function datasetSummary(path: string): string | undefined {
		return datasets.find((d) => d.path === path)?.summary || undefined
	}

	/** A run that only scored an earlier one says so, or it reads as the agent having answered
	 *  again. Named by the run it measured, which is a number the list also shows. */
	function scoredFrom(experiment: EvalExperiment): string | undefined {
		if (!experiment.scored_from) return undefined
		const parent = experiments.find((e) => e.id === experiment.scored_from)
		return parent ? `scored from run ${parent.run_number}` : 'scored from an earlier run'
	}

	/** The one number a column reports: a pass rate where it has a line to pass, the mean where it
	 *  does not. The same headline the column shows over the table of that run. */
	function headline(score: ExperimentScore): string | undefined {
		if (score.pass_rate != undefined) return `${Math.round(score.pass_rate * 100)}%`
		return score.mean == undefined ? undefined : formatScore(score.mean)
	}
</script>

<DataTable size="sm" tableFixed>
	<colgroup>
		<col style="width: 20%" />
		<col style="width: 5rem" />
		<col />
		<col style="width: 22%" />
		<col style="width: 7rem" />
	</colgroup>
	<Head>
		<tr>
			<Cell head first>Run</Cell>
			<Cell head numeric>Cases</Cell>
			<Cell head>Scores</Cell>
			<Cell head>Dataset</Cell>
			<Cell head last numeric>When</Cell>
		</tr>
	</Head>
	<tbody class="divide-y">
		{#each experiments as experiment (experiment.id)}
			{@const scored = scoredFrom(experiment)}
			<Row on:click={() => onOpen(experiment)}>
				<Cell first>
					<!-- What ran beside which run it was: two runs are only comparable because each says
					     what it executed. -->
					<div class="flex flex-col min-w-0">
						<div class="flex items-center gap-1.5 min-w-0">
							<span class="truncate text-emphasis font-medium">{experimentName(experiment)}</span>
							<Badge color="gray" class="shrink-0">{subjectLabel(experiment)}</Badge>
						</div>
						<span class="text-2xs text-tertiary truncate">
							{#if scored}{scored}{:else}{experiment.created_by}{/if}
						</span>
					</div>
				</Cell>
				<Cell numeric>
					<span class="tabular-nums text-secondary">{experiment.case_count}</span>
				</Cell>
				<Cell>
					<!-- One badge per column of the dataset that ran, named: a run measured by three
					     scorers answered three different questions, and one number would be their average
					     rather than an answer to any of them. -->
					<div class="flex flex-wrap gap-1 min-w-0">
						{#each experiment.scores ?? [] as score (score.scorer_id)}
							{@const value = headline(score)}
							<Badge color="gray" class="max-w-full">
								<span class="flex items-baseline gap-1 min-w-0">
									{#if score.kind === 'agent'}
										<Bot size={11} class="text-tertiary shrink-0 self-center" />
									{:else}
										<Code2 size={11} class="text-tertiary shrink-0 self-center" />
									{/if}
									<span class="truncate">{score.name}</span>
									{#if value != undefined}
										<span class="tabular-nums font-semibold text-emphasis">{value}</span>
									{:else}
										<span class="text-tertiary">—</span>
									{/if}
								</span>
							</Badge>
						{/each}
						{#if (experiment.scores ?? []).length === 0}
							{#if experiment.running}
								<span class="text-2xs text-tertiary inline-flex items-center gap-1">
									<Loader2 size={11} class="animate-spin text-blue-500" />
									scoring
								</span>
							{:else}
								<!-- Nothing scored this run, rather than a column still scoring it: a dataset
								     with no scorers, or scorers added after the run happened. -->
								<span class="text-2xs text-tertiary">not scored</span>
							{/if}
						{/if}
					</div>
				</Cell>
				<Cell>
					<!-- The dataset a run is of, and the way into it: what a run measured and what the
					     next one will are the same question asked a day apart. -->
					<button
						type="button"
						class="flex flex-col min-w-0 max-w-full text-left hover:underline"
						title={`Edit ${experiment.dataset}`}
						onclick={(e) => {
							e.stopPropagation()
							onEditDataset(experiment.dataset)
						}}
					>
						<span class="text-xs text-secondary truncate leading-tight">
							{datasetSummary(experiment.dataset) || experiment.dataset}
						</span>
						{#if datasetSummary(experiment.dataset)}
							<span class="text-2xs text-tertiary truncate leading-tight">
								{experiment.dataset}
							</span>
						{/if}
					</button>
				</Cell>
				<Cell last numeric>
					<span class="text-2xs text-tertiary whitespace-nowrap">
						<TimeAgo date={experiment.created_at} agoOnlyIfRecent />
					</span>
				</Cell>
			</Row>
		{/each}
		{#if experiments.length === 0}
			<tr>
				<td colspan="5" class="p-6">
					<div class="flex flex-col items-center justify-center gap-2 text-center">
						<span class="text-sm text-emphasis">No runs yet</span>
						<span class="text-xs text-secondary max-w-md">
							A run answers every case of a dataset and scores the answers. Each one is kept, so the
							next has something to be compared against.
						</span>
					</div>
				</td>
			</tr>
		{/if}
	</tbody>
</DataTable>
