<script lang="ts">
	import { base } from '$lib/base'
	import { Button, Skeleton } from '$lib/components/common'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { AiEvalsService, type EvalExperiment, type ExperimentRow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { ExternalLink, RefreshCw } from 'lucide-svelte'

	let {
		dataset,
		workspace = undefined,
		refreshToken = 0
	}: {
		dataset: string | undefined
		workspace?: string
		/** Bumped by the parent when it starts an experiment, to pick the new one up. */
		refreshToken?: number
	} = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let experiments = $state<EvalExperiment[]>([])
	let selectedId = $state<string | undefined>(undefined)
	let rows = $state<ExperimentRow[]>([])
	let scorerLabels = $state<string[]>([])
	let loading = $state(false)

	let listGeneration = 0
	async function loadExperiments(path: string | undefined, _token: number) {
		const generation = ++listGeneration
		if (!ws || !path) {
			experiments = []
			selectedId = undefined
			return
		}
		const found = await AiEvalsService.listExperiments({ workspace: ws, path }).catch(() => [])
		if (generation !== listGeneration) return
		experiments = found
		// Newest first from the API, so an unset or vanished selection lands on the latest run.
		if (!selectedId || !found.some((e) => e.id === selectedId)) {
			selectedId = found[0]?.id
		}
	}

	let resultsGeneration = 0
	async function loadResults(path: string | undefined, id: string | undefined) {
		const generation = ++resultsGeneration
		if (!ws || !path || !id) {
			rows = []
			scorerLabels = []
			loading = false
			return
		}
		loading = true
		try {
			const res = await AiEvalsService.experimentResults({
				workspace: ws,
				requestBody: { dataset: path, id }
			})
			if (generation !== resultsGeneration) return
			rows = res.rows ?? []
			scorerLabels = res.scorer_labels ?? []
		} finally {
			if (generation === resultsGeneration) loading = false
		}
	}

	$effect(() => {
		loadExperiments(dataset, refreshToken)
	})
	$effect(() => {
		loadResults(dataset, selectedId)
	})

	let selected = $derived(experiments.find((e) => e.id === selectedId))

	// The summary a comparison would diff. Scorers that produced no number are left out rather
	// than counted as zero, which would read as a regression instead of a missing score.
	let means = $derived(
		scorerLabels.map((_, index) => {
			const values = rows
				.map((r) => r.scores?.[index])
				.filter((v): v is number => typeof v === 'number')
			if (values.length === 0) return undefined
			return values.reduce((a, b) => a + b, 0) / values.length
		})
	)
	let stillRunning = $derived(rows.filter((r) => r.status === 'running').length)
</script>

<div class="flex flex-col h-full min-h-0 gap-2">
	<div class="flex items-center gap-2">
		<div class="grow min-w-0">
			<Select
				items={experiments.map((e) => ({
					label: `${displayDate(e.created_at)} · ${e.subject.path}${
						e.subject.version != undefined ? ` v${e.subject.version}` : ''
					}`,
					value: e.id
				}))}
				bind:value={selectedId}
				placeholder="No experiment yet"
				class="text-xs"
			/>
		</div>
		<Button
			variant="default"
			size="xs2"
			startIcon={{ icon: RefreshCw }}
			iconOnly
			title="Refresh results"
			onclick={() => loadResults(dataset, selectedId)}
		/>
	</div>

	{#if selected}
		<div class="flex items-center gap-3 text-2xs text-tertiary">
			<span>{rows.length} cases</span>
			{#if stillRunning > 0}
				<span>{stillRunning} running</span>
			{/if}
			{#each scorerLabels as label, index (label)}
				<span>
					{label}:
					<span class="text-primary font-medium">
						{means[index] != undefined ? means[index].toFixed(2) : '—'}
					</span>
				</span>
			{/each}
		</div>
	{/if}

	<div class="flex-1 min-h-0 overflow-auto">
		{#if loading && rows.length === 0}
			<Skeleton layout={[[2], [2], [2]]} />
		{:else if !selected}
			<div class="text-xs text-tertiary p-2">
				No experiment yet. Run the dataset to score every case in one go.
			</div>
		{:else}
			<DataTable size="xs" noBorder shouldHidePagination>
				<Head>
					<tr>
						<Cell head first>Case</Cell>
						<Cell head>Output</Cell>
						{#each scorerLabels as label (label)}
							<Cell head>{label}</Cell>
						{/each}
						<Cell head last></Cell>
					</tr>
				</Head>
				<tbody>
					{#each rows as row (row.case_id)}
						<tr class="border-b last:border-b-0">
							<Cell first>
								<div class="flex items-center gap-1 min-w-0">
									<span
										class={row.status === 'success'
											? 'text-green-600'
											: row.status === 'failure'
												? 'text-red-600'
												: 'text-tertiary'}>●</span
									>
									<span class="truncate">
										{row.name || row.input?.user_message || 'Untitled case'}
									</span>
								</div>
							</Cell>
							<Cell>
								<span class="truncate block max-w-72" title={row.output}>
									{row.output ?? ''}
								</span>
							</Cell>
							{#each scorerLabels as label, index (label)}
								<Cell numeric>
									{row.scores?.[index] != undefined ? row.scores[index]?.toFixed(2) : '—'}
								</Cell>
							{/each}
							<Cell last>
								<a
									href={`${base}/run/${row.job_id}?workspace=${ws}`}
									target="_blank"
									title="Open this case's run"
								>
									<ExternalLink size={12} />
								</a>
							</Cell>
						</tr>
					{/each}
				</tbody>
			</DataTable>
		{/if}
	</div>
</div>
