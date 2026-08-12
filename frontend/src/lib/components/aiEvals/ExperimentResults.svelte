<script lang="ts">
	import { base } from '$lib/base'
	import { Button, Skeleton } from '$lib/components/common'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { AiEvalsService, type EvalExperiment, type ExperimentRow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate } from '$lib/utils'
	import { Ban, Check, ExternalLink, FastForward, Play, RefreshCw, X } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	// Status carries an icon as well as a colour, since colour alone says nothing to a
	// colour-blind reader. Same icon vocabulary as the runs table (`JobStatusIcon`).
	const STATUS = {
		success: { icon: Check, color: 'text-green-500' },
		failure: { icon: X, color: 'text-red-500' },
		canceled: { icon: Ban, color: 'text-tertiary' },
		skipped: { icon: FastForward, color: 'text-tertiary' },
		running: { icon: Play, color: 'text-yellow-500' }
	} as const

	let {
		dataset,
		workspace = undefined,
		refreshToken = 0,
		selectExperimentId = undefined
	}: {
		dataset: string | undefined
		workspace?: string
		/** Bumped by the parent when it starts an experiment, to pick the new one up. */
		refreshToken?: number
		/** The experiment the parent just started, which becomes the selected one. */
		selectExperimentId?: string
	} = $props()

	let ws = $derived(workspace ?? $workspaceStore)

	let experiments = $state<EvalExperiment[]>([])
	let selectedId = $state<string | undefined>(undefined)
	let rows = $state<ExperimentRow[]>([])
	let scorerLabels = $state<string[]>([])
	let loading = $state(false)
	let shownExperimentId: string | undefined = undefined
	// The comparison is the point of the table: one experiment's numbers say little, the delta
	// against the run before the change says whether the change helped.
	let baselineId = $state<string | undefined>(undefined)
	let baselineRows = $state<ExperimentRow[]>([])
	let baselineScorers = $state<{ kind: string; path: string }[]>([])
	let scorers = $state<{ kind: string; path: string }[]>([])
	let onlyRegressions = $state(false)

	let listGeneration = 0
	let listedDataset: string | undefined = undefined
	async function loadExperiments(path: string | undefined, _token: number) {
		const generation = ++listGeneration
		// Dropped synchronously, before the await: an id from the previous dataset would otherwise
		// be requested under the new one and toast its 404.
		if (path !== listedDataset) {
			listedDataset = path
			selectedId = undefined
			baselineId = undefined
			experiments = []
		}
		if (!ws || !path) {
			experiments = []
			selectedId = undefined
			return
		}
		const found = await AiEvalsService.listExperiments({ workspace: ws, path }).catch(() => [])
		if (generation !== listGeneration) return
		experiments = found
		// A just-started experiment wins; otherwise, newest first from the API means an unset or
		// vanished selection lands on the latest run.
		if (selectExperimentId && found.some((e) => e.id === selectExperimentId)) {
			selectedId = selectExperimentId
		} else if (!selectedId || !found.some((e) => e.id === selectedId)) {
			selectedId = found[0]?.id
		}
	}

	let resultsGeneration = 0
	async function loadResults(path: string | undefined, id: string | undefined) {
		const generation = ++resultsGeneration
		// Cleared up front when the selection changes, so the header never names one experiment
		// over another's numbers and a failed request cannot leave them there. A refresh of the
		// same experiment keeps its rows rather than flashing a skeleton.
		if (id !== shownExperimentId) {
			rows = []
			scorerLabels = []
			scorers = []
		}
		shownExperimentId = id
		if (!ws || !path || !id) {
			loading = false
			return
		}
		loading = true
		try {
			const res = await AiEvalsService.experimentResults({ workspace: ws, path, id })
			if (generation !== resultsGeneration) return
			rows = res.rows ?? []
			scorerLabels = res.scorer_labels ?? []
			scorers = (res.experiment?.scorers ?? []).map((sc) => ({ kind: sc.kind, path: sc.path }))
		} catch (err) {
			if (generation === resultsGeneration) {
				sendUserToast((err as any)?.body ?? String(err), true)
			}
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
	// A baseline is only meaningful among the loaded experiments: it must drop both when it becomes
	// the selected run and when the dataset changes underneath it, or comparison mode stays on with
	// nothing to compare and every mean reads as missing.
	$effect(() => {
		if (!baselineId) return
		if (baselineId === selectedId || !experiments.some((e) => e.id === baselineId)) {
			baselineId = undefined
		}
	})

	let baselineGeneration = 0
	$effect(() => {
		const path = dataset
		const id = baselineId
		const generation = ++baselineGeneration
		baselineRows = []
		baselineScorers = []
		if (!ws || !path || !id) return
		AiEvalsService.experimentResults({ workspace: ws, path, id })
			.then((res) => {
				if (generation !== baselineGeneration) return
				baselineRows = res.rows ?? []
				baselineScorers = (res.experiment?.scorers ?? []).map((sc) => ({
					kind: sc.kind,
					path: sc.path
				}))
			})
			.catch(() => {
				if (generation === baselineGeneration) {
					baselineRows = []
					baselineScorers = []
				}
			})
	})

	let baselineByCase = $derived(Object.fromEntries(baselineRows.map((r) => [r.case_id, r])))

	// The two experiments' scorer lists can differ, so a delta is only meaningful between the same
	// scorer. Matched on kind and path rather than label: labels default to a path's last segment,
	// so `f/a/quality` and `f/b/quality` both read "quality" and would compare against each other.
	function baselineIndex(index: number): number {
		const mine = scorers[index]
		if (!mine) return -1
		return baselineScorers.findIndex((b) => b.kind === mine.kind && b.path === mine.path)
	}

	function delta(row: ExperimentRow, index: number): number | undefined {
		const other = baselineIndex(index)
		if (other < 0) return undefined
		const now = row.scores?.[index]
		const before = baselineByCase[row.case_id]?.scores?.[other]
		if (typeof now !== 'number' || typeof before !== 'number') return undefined
		return now - before
	}

	// A row is a regression if any scorer went down against the baseline.
	function isRegression(row: ExperimentRow): boolean {
		return scorerLabels.some((_, index) => (delta(row, index) ?? 0) < 0)
	}

	let visibleRows = $derived(onlyRegressions && baselineId ? rows.filter(isRegression) : rows)

	let selected = $derived(experiments.find((e) => e.id === selectedId))

	// Scorers that produced no number are left out rather than counted as zero, which would read
	// as a regression instead of a missing score. While comparing, the mean is taken over the same
	// cases as its delta — otherwise the two numbers beside each other describe different sets.
	let means = $derived(
		scorerLabels.map((_, index) => {
			// Paired rows only while there is something to compare this scorer against; a scorer the
			// baseline never ran still shows its own mean rather than blanking a column of numbers.
			const paired = baselineId ? pairedRows(index) : []
			const values = (paired.length > 0 ? paired : rows)
				.map((r) => r.scores?.[index])
				.filter((v): v is number => typeof v === 'number')
			if (values.length === 0) return undefined
			return values.reduce((a, b) => a + b, 0) / values.length
		})
	)

	/** Rows this scorer produced a number for in both runs. */
	function pairedRows(index: number): ExperimentRow[] {
		const other = baselineIndex(index)
		if (other < 0) return []
		return rows.filter(
			(r) =>
				typeof r.scores?.[index] === 'number' &&
				typeof baselineByCase[r.case_id]?.scores?.[other] === 'number'
		)
	}
	let stillRunning = $derived(rows.filter((r) => r.status === 'running').length)
	let regressionCount = $derived(baselineId ? rows.filter(isRegression).length : 0)
	let meanDeltas = $derived(
		scorerLabels.map((_, index) => {
			const other = baselineIndex(index)
			if (other < 0) return undefined
			// Averaged over the cases both runs scored. Comparing each run's own average would
			// report a regression from a case the baseline never ran, with no regressed row to
			// point at.
			const paired = pairedRows(index)
			if (paired.length === 0) return undefined
			const before =
				paired.reduce((a, r) => a + (baselineByCase[r.case_id]!.scores![other] as number), 0) /
				paired.length
			return means[index]! - before
		})
	)
</script>

<div class="flex flex-col h-full min-h-0 gap-2">
	<div class="flex items-center gap-2">
		<div class="grow min-w-0">
			<Select
				items={experiments.map((e) => ({
					label: `${displayDate(e.created_at)} · ${e.subject.path}${
						e.subject.version != undefined ? ` v${e.subject.version}` : ''
					} · ${e.case_count} ${e.case_count === 1 ? 'case' : 'cases'}`,
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

	{#if selected && experiments.length > 1}
		<div class="flex items-center gap-2">
			<span class="text-2xs text-tertiary whitespace-nowrap">vs</span>
			<div class="grow min-w-0">
				<Select
					items={experiments
						.filter((e) => e.id !== selectedId)
						.map((e) => ({
							label: `${displayDate(e.created_at)} · ${e.subject.path}${
								e.subject.version != undefined ? ` v${e.subject.version}` : ''
							}`,
							value: e.id
						}))}
					bind:value={baselineId}
					placeholder="Compare with"
					clearable
					class="text-xs"
				/>
			</div>
		</div>
	{/if}

	{#if selected}
		<div class="flex items-center gap-3 text-2xs text-tertiary">
			<span>{rows.length} cases</span>
			{#if stillRunning > 0}
				<span>{stillRunning} running</span>
			{/if}
			{#each scorerLabels as label, index (index)}
				<span>
					{label}:
					<span class="text-primary font-medium">
						{means[index] != undefined ? means[index].toFixed(2) : '—'}
					</span>
					{#if baselineId && meanDeltas[index] != undefined}
						<span class={meanDeltas[index]! < 0 ? 'text-red-500' : 'text-green-500'}>
							{meanDeltas[index]! >= 0 ? '+' : ''}{meanDeltas[index]!.toFixed(2)}
						</span>
					{/if}
				</span>
			{/each}
			{#if baselineId}
				<Toggle
					bind:checked={onlyRegressions}
					size="2xs"
					options={{ right: `${regressionCount} regressed` }}
				/>
			{/if}
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
						{#each scorerLabels as label, index (index)}
							<Cell head>{label}</Cell>
						{/each}
						<Cell head last></Cell>
					</tr>
				</Head>
				<tbody>
					{#each visibleRows as row (row.case_id)}
						<tr class="border-b last:border-b-0">
							<Cell first>
								{@const status = STATUS[row.status] ?? STATUS.running}
								{@const StatusIcon = status.icon}
								<div class="flex items-center gap-1 min-w-0">
									<span
										class={`shrink-0 ${status.color}`}
										role="img"
										title={row.status}
										aria-label={row.status}
									>
										<StatusIcon size={12} />
									</span>
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
							{#each scorerLabels as _label, index (index)}
								{@const d = delta(row, index)}
								<Cell numeric>
									{row.scores?.[index] != undefined ? row.scores[index]?.toFixed(2) : '—'}
									{#if d != undefined && d !== 0}
										<span class={d < 0 ? 'text-red-500' : 'text-green-500'}>
											{d > 0 ? '+' : ''}{d.toFixed(2)}
										</span>
									{/if}
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
