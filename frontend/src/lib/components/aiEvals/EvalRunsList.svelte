<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { Button } from '$lib/components/common'
	import { Bot, Code2, Loader2, Plus } from 'lucide-svelte'
	import type { EvalDataset, EvalExperiment, ExperimentScore } from '$lib/gen'
	import { datasetSummary, experimentName, formatScore, subjectLabel } from './evalUtils'

	let {
		experiments,
		datasets,
		loaded,
		deployedHash = undefined,
		currentVersion = undefined,
		onOpen,
		onEditDataset,
		onNew
	}: {
		/** Every run of this agent, newest first, whichever dataset each is of. */
		experiments: EvalExperiment[]
		/** Whether the list has been read: an empty table is a statement about the agent. */
		loaded: boolean
		/** The workspace's datasets, for naming the one a run is of by what it is for. */
		datasets: EvalDataset[]
		/** What the agent hashes to as deployed, and the version it is on: they resolve a run of
		 *  edits that were later saved, so a run is labelled here as the run picker labels it. */
		deployedHash?: string
		currentVersion?: number
		onOpen: (experiment: EvalExperiment) => void
		onEditDataset: (dataset: string) => void
		onNew: () => void
	} = $props()

	/** The one number a column reports: a pass rate where it has a line to pass, the mean where it
	 *  does not. */
	function headline(score: ExperimentScore): string | undefined {
		if (score.pass_rate != undefined) return `${Math.round(score.pass_rate * 100)}%`
		return score.mean == undefined ? undefined : formatScore(score.mean)
	}
</script>

<DataTable size="sm" tableFixed>
	<colgroup>
		<col style="width: 20%" />
		<col style="width: 22%" />
		<col style="width: 5rem" />
		<col />
		<col style="width: 7rem" />
	</colgroup>
	<Head>
		<tr>
			<Cell head first>Run</Cell>
			<Cell head>Dataset</Cell>
			<Cell head numeric>Cases</Cell>
			<Cell head>Scores</Cell>
			<Cell head last numeric>When</Cell>
		</tr>
	</Head>
	<tbody class="divide-y">
		{#each experiments as experiment (experiment.id)}
			<Row hoverable on:click={() => onOpen(experiment)}>
				<Cell first>
					<div class="flex flex-col min-w-0">
						<div class="flex items-center gap-1.5 min-w-0">
							<span class="truncate text-emphasis font-medium">{experimentName(experiment)}</span>
							<Badge color="gray" class="shrink-0">
								{subjectLabel(experiment, deployedHash, currentVersion)}
							</Badge>
						</div>
						<span class="text-2xs text-tertiary truncate">{experiment.created_by}</span>
					</div>
				</Cell>
				<Cell>
					{@const summary = datasetSummary(datasets, experiment.dataset)}
					<Button
						variant="subtle"
						unifiedSize="sm"
						title={`Edit ${experiment.dataset}`}
						wrapperClasses="min-w-0 max-w-full"
						btnClasses="!h-auto py-1 !px-0 !font-normal flex-col items-start text-left hover:underline hover:!bg-transparent min-w-0 max-w-full"
						onClick={(e) => {
							e?.stopPropagation()
							onEditDataset(experiment.dataset)
						}}
					>
						<span class="text-xs text-secondary truncate leading-tight">
							{summary || experiment.dataset}
						</span>
						{#if summary}
							<span class="text-2xs text-tertiary truncate leading-tight">
								{experiment.dataset}
							</span>
						{/if}
					</Button>
				</Cell>
				<Cell numeric>
					<span class="tabular-nums text-secondary">{experiment.case_count}</span>
				</Cell>
				<Cell>
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
									{:else if score.failed > 0}
										<span class="text-red-500">failed</span>
									{:else if experiment.running}
										<Loader2 size={11} class="animate-spin text-blue-500 self-center" />
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
								<span class="text-2xs text-tertiary">not scored</span>
							{/if}
						{/if}
					</div>
				</Cell>
				<Cell last numeric>
					<span class="text-2xs text-tertiary whitespace-nowrap">
						<TimeAgo date={experiment.created_at} agoOnlyIfRecent />
					</span>
				</Cell>
			</Row>
		{/each}
		{#if experiments.length === 0 && !loaded}
			<tr>
				<td colspan="5" class="p-3">
					<Skeleton layout={[[2], 0.5, [2], 0.5, [2]]} />
				</td>
			</tr>
		{:else if experiments.length === 0}
			<tr>
				<td colspan="5" class="p-6">
					<div class="flex flex-col items-center justify-center gap-3 text-center">
						<span class="text-sm text-emphasis">No runs yet</span>
						<span class="text-xs text-secondary max-w-md">
							A run answers every case of a dataset and scores the answers. Each one is kept, so the
							next has something to be compared against.
						</span>
						<Button unifiedSize="md" variant="accent" startIcon={{ icon: Plus }} onclick={onNew}>
							New evaluation
						</Button>
					</div>
				</td>
			</tr>
		{/if}
	</tbody>
</DataTable>
