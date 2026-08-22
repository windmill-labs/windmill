<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Label from '$lib/components/Label.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import {
		type AgentDraft,
		AiEvalsService,
		type EvalCase,
		type EvalDataset,
		type EvalExperiment,
		type EvalSubject,
		type ExperimentRow,
		type Scorer,
		type ScorerMean
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, onMount, untrack } from 'svelte'
	import {
		Plus,
		X,
		Check,
		Ban,
		FastForward,
		Loader2,
		Minus,
		Bot,
		Code2,
		ExternalLink
	} from 'lucide-svelte'
	import EvalDatasetDrawer from './EvalDatasetDrawer.svelte'
	import EvalRunsList from './EvalRunsList.svelte'
	import EvalRunDialog from './EvalRunDialog.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import {
		caseLabel,
		experimentName,
		formatDelta,
		formatScore,
		scorerLabel,
		subjectLabel,
		type EvalsLocation
	} from './evalUtils'

	/** A dataset is capped at this many cases, so one page holds the whole set. */
	const CASE_PAGE_SIZE = 1000

	let {
		agentPath,
		opWorkspace = undefined,
		editedConfig = undefined,
		location = $bindable()
	}: {
		/** The agent under test. A dataset and its runs belong to an agent. */
		agentPath: string
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
		/** Opened from an agent being edited: the edits, as the step holds them, are what a run is
		 * offered on. Everywhere else the agent is what is deployed. */
		editedConfig?: () => AgentDraft
		/** The level the pane is on and the way out of it, reported up so the surface holding it
		 * can put both in its header. Undefined at the root, which that surface already names. */
		location?: EvalsLocation
	} = $props()

	let ws = $derived(opWorkspace ?? $workspaceStore)
	let datasets = $state<EvalDataset[]>([])
	let dataset = $state<EvalDataset | undefined>(undefined)
	let selectedDataset = $state<string | undefined>(undefined)
	let experiments = $state<EvalExperiment[]>([])
	// Whether a run is open. Which run is `experimentId`, which outlives closing it.
	let viewingRun = $state(false)
	let experimentId = $state<string | undefined>(undefined)
	let baselineId = $state<string | undefined>(undefined)
	let rows = $state<ExperimentRow[]>([])
	// The dataset's cases as they are now, keyed by id. A row carries the case *as the experiment
	// ran it*, so a row is never written back: that would save a stale input and drop the fields a
	// row does not carry.
	let storedCases = $state<Record<string, EvalCase>>({})
	let means = $state<ScorerMean[]>([])
	/** The version the agent is on now, against which a row's own version is stale or current. */
	let currentVersion = $state<number | undefined>(undefined)
	/** What the agent hashes to as deployed: a run of edits carrying it ran what was then saved. */
	let deployedHash = $state<string | undefined>(undefined)
	let running = $state(false)
	let scorers = $derived(dataset?.scorers ?? [])
	let selectedCaseId = $state<string | undefined>(undefined)

	let datasetDrawer: EvalDatasetDrawer | undefined = $state()
	let runDialogOpen = $state(false)
	let resumeRunDialog = $state(false)

	let experiment = $derived(experiments.find((e) => e.id === experimentId))

	async function listSubjectExperiments(): Promise<EvalExperiment[]> {
		if (!ws) return []
		return await AiEvalsService.listAllExperiments({ workspace: ws, subjectPath: agentPath })
	}

	/** Whether the two lists have been read. Every empty state here is a statement about the agent
	 *  or the workspace, so none of them is said while the answer is still on its way. */
	let runsLoaded = $state(false)
	let datasetsLoaded = $state(false)
	// A rejected load would otherwise render as "No dataset yet": a real emptiness and a failed read
	// must not look the same. Tracked per loader, so a successful reload of one keeps the other's.
	let runsLoadError = $state(false)
	let datasetsLoadError = $state(false)
	let loadError = $derived(runsLoadError || datasetsLoadError)
	let loaded = $derived(!ws || (runsLoaded && datasetsLoaded))

	async function loadRuns() {
		runsLoadError = false
		try {
			experiments = await listSubjectExperiments()
		} catch (e) {
			runsLoadError = true
			sendUserToast(`Failed to load the runs: ${e}`, true)
		} finally {
			runsLoaded = true
		}
	}

	let lastDatasetKey = $derived(`evals:dataset:${ws}:${agentPath}`)
	// Only ever written, never cleared: the pane mounts with nothing selected, and clearing on that
	// would erase the memory a moment before the load that reads it.
	function rememberDataset(path: string | undefined) {
		if (!path) return
		try {
			localStorage.setItem(lastDatasetKey, path)
		} catch {
			// Storage is a convenience: a browser refusing it costs the memory, not the pane.
		}
	}

	function rememberedDataset(): string | undefined {
		try {
			return localStorage.getItem(lastDatasetKey) ?? undefined
		} catch {
			return undefined
		}
	}

	async function loadDatasets() {
		if (!ws) return
		datasetsLoadError = false
		try {
			datasets = await AiEvalsService.listEvalDatasets({ workspace: ws })
		} catch (e) {
			datasetsLoadError = true
			sendUserToast(`Failed to load the datasets: ${e}`, true)
			return
		} finally {
			datasetsLoaded = true
		}
		if (selectedDataset) return
		const remembered = rememberedDataset()
		// Only if it is still there, and brought into context rather than merely selected: what is
		// selected is always a dataset the pane has actually read.
		if (remembered && datasets.some((d) => d.path === remembered)) {
			await useDataset(remembered)
		}
	}

	/**
	 * Bring a dataset into context: its metadata, its scorers and its cases. Called explicitly
	 * rather than from an effect on the selection: the load resets which run is being read, so an
	 * effect would race every caller that sets the dataset and then opens a run of it.
	 */
	async function useDataset(path: string): Promise<boolean> {
		selectedDataset = path
		rememberDataset(path)
		return await loadDataset(path)
	}

	// Switching datasets leaves the previous request in flight; only the newest may write, or a
	// slow response for the dataset you just left replaces the one you are looking at.
	let loadGeneration = 0

	async function loadDataset(path: string | undefined): Promise<boolean> {
		const generation = ++loadGeneration
		if (!ws || !path) {
			dataset = undefined
			storedCases = {}
			return false
		}
		try {
			const [row, cases] = await Promise.all([
				AiEvalsService.getEvalDataset({ workspace: ws, path }),
				AiEvalsService.listEvalCases({ workspace: ws, path, perPage: CASE_PAGE_SIZE })
			])
			if (generation !== loadGeneration) return false
			dataset = row
			storedCases = Object.fromEntries(cases.cases.map((c) => [c.id, c]))
			return true
		} catch (e) {
			if (generation === loadGeneration) {
				// A failed load leaves the previous dataset in hand: cleared, or the edit drawer opens
				// on another dataset's cases and writes them back under this path.
				dataset = undefined
				storedCases = {}
				sendUserToast(`Failed to load ${path}: ${e}`, true)
			}
			return false
		}
	}

	async function reloadCases() {
		if (!ws || !selectedDataset) return
		const cases = await AiEvalsService.listEvalCases({
			workspace: ws,
			path: selectedDataset,
			perPage: CASE_PAGE_SIZE
		})
		storedCases = Object.fromEntries(cases.cases.map((c) => [c.id, c]))
	}

	// The run picker, the baseline picker and the 2s poller all call this, so responses overlap:
	// only the newest may write, or a superseded read replaces the table under the reader.
	let resultsGeneration = 0
	// Said once per failing streak: the poller comes back every 2s, and a run that cannot be read
	// is a run that cannot be read again.
	let resultsFailureReported = false

	async function loadResults() {
		const generation = ++resultsGeneration
		if (!ws || !selectedDataset || !experimentId) {
			rows = []
			means = []
			return
		}
		try {
			const results = await AiEvalsService.experimentResults({
				workspace: ws,
				path: selectedDataset,
				id: experimentId,
				baseline: baselineId
			})
			if (generation !== resultsGeneration) return
			rows = results.rows
			means = results.means
			currentVersion = results.subject_current_version
			deployedHash = results.subject_deployed_hash
			if (dataset) dataset = { ...dataset, scorers: results.scorers }
			resultsFailureReported = false
		} catch (e) {
			if (generation !== resultsGeneration) return
			if (!resultsFailureReported) {
				resultsFailureReported = true
				sendUserToast(`Failed to read the run: ${e}`, true)
			}
		}
	}

	/**
	 * Every case of the dataset, in dataset order, carrying its result in the selected experiment
	 * when there is one. Cases the experiment ran but the dataset no longer has keep their row at
	 * the end: the run happened, and deleting the case does not unmake it.
	 */
	let displayRows: ExperimentRow[] = $derived.by(() => {
		const byCase = new Map(rows.map((row) => [row.case_id, row]))
		const ordered: ExperimentRow[] = Object.values(storedCases).map(
			(stored) =>
				byCase.get(stored.id) ?? {
					case_id: stored.id,
					input: stored.input ?? {},
					expected: stored.expected,
					job_id: '',
					status: 'not_run' as ExperimentRow['status'],
					scores: []
				}
		)
		return [...ordered, ...rows.filter((row) => !storedCases[row.case_id])]
	})

	/** The version the agent is on right now, so a run's label can say it is of an earlier one. Its
	 *  own endpoint rather than the results, which harvest scores and read every job. */
	async function readSubjectState() {
		if (!ws || !agentPath || document.hidden) return
		try {
			const state = await AiEvalsService.evalSubjectState({ workspace: ws, path: agentPath })
			currentVersion = state.version
		} catch {
			// Deleted or no longer readable: the table keeps what it last knew.
		}
	}
	onMount(() => {
		readSubjectState()
	})
	// Coming back to the tab is when an edit or a run made elsewhere is waiting: the poller only
	// arms for a run this pane already knows about, so one started in another tab would otherwise
	// never appear on a list left open.
	$effect(() => {
		const onFocus = () => {
			if (document.hidden) return
			untrack(() => {
				readSubjectState()
				refresh()
			})
		}
		window.addEventListener('focus', onFocus)
		document.addEventListener('visibilitychange', onFocus)
		return () => {
			window.removeEventListener('focus', onFocus)
			document.removeEventListener('visibilitychange', onFocus)
		}
	})

	/** A case still running, or a score not in yet. On the list, a run whose flow is still going. */
	let pollNeeded = $derived(
		viewingRun
			? rows.some((row) => row.status === 'running' || row.scores.some((score) => score.pending))
			: experiments.some((e) => e.running)
	)
	let poller: ReturnType<typeof setInterval> | undefined = undefined
	$effect(() => {
		if (pollNeeded && !poller) {
			poller = setInterval(() => {
				untrack(() => refresh())
			}, 2000)
		} else if (!pollNeeded && poller) {
			clearInterval(poller)
			poller = undefined
		}
	})
	onDestroy(() => poller && clearInterval(poller))

	// One pass at a time: the poller fires every 2s whether or not the last read came back, and
	// every read supersedes the one before it, so a read slower than the interval would be
	// discarded by the next one forever and the table would never advance.
	let refreshing = false
	async function refresh() {
		if (!ws || refreshing) return
		refreshing = true
		try {
			// Reading both would read every cell of a run nobody is looking at.
			if (viewingRun) {
				await loadResults()
			} else {
				experiments = await listSubjectExperiments()
			}
		} finally {
			refreshing = false
		}
	}

	/** Opens a run, bringing its dataset with it and offering the run before it as the baseline.
	 *  Reading the cells is left to the effect on the selection, so every way in opens one alike. */
	async function openRun(id: string) {
		// Against the dataset that is loaded, not the one that is selected: skipping on the selection
		// alone would leave a run open over a dataset whose cases and scorers were never read.
		const target = experiments.find((e) => e.id === id)
		if (target && target.dataset !== dataset?.path) {
			await useDataset(target.dataset)
		}
		const index = experiments.findIndex((e) => e.id === id)
		// The run before it *of the same dataset*: the list spans datasets, and a run of another set
		// of cases is not a baseline for this one.
		baselineId = experiments.slice(index + 1).find((e) => e.dataset === target?.dataset)?.id
		experimentId = id
		viewingRun = true
		selectedCaseId = undefined
	}

	async function runAll(runSubject: EvalSubject, path: string): Promise<boolean> {
		if (!ws || !path) return false
		running = true
		let id: string
		try {
			id = await AiEvalsService.runExperiment({
				workspace: ws,
				requestBody: { dataset: path, subject: runSubject }
			})
		} catch (e) {
			running = false
			sendUserToast(`Failed to run the dataset: ${e}`, true)
			return false
		}
		// From here the run exists and is billing: what can still fail is reading it back, and
		// saying "failed to run" to that invites a second, duplicate run.
		try {
			if (path !== dataset?.path) await useDataset(path)
			await loadRuns()
			await openRun(id)
		} catch (e) {
			sendUserToast(
				`The run started but could not be read back: ${e}. Reload the runs list to see it.`,
				true
			)
		} finally {
			running = false
		}
		return true
	}

	async function selectSavedDataset(path: string) {
		await loadDatasets()
		await useDataset(path)
	}

	/** The dataset is gone and every run of it with it: back to the list, on no dataset. */
	async function datasetDeleted(path: string) {
		if (selectedDataset === path) {
			viewingRun = false
			selectedCaseId = undefined
			experimentId = undefined
			baselineId = undefined
			selectedDataset = undefined
			rememberDataset(undefined)
			await loadDataset(undefined)
		}
		await loadDatasets()
		await loadRuns()
	}

	/** A pass line that moved re-reads every score already recorded, so the run on screen is
	 *  re-read with the dataset. */
	async function scorersChanged() {
		if (selectedDataset) await loadDataset(selectedDataset)
		await loadResults()
		await loadRuns()
	}

	/** The runs list names the dataset and counts its cases, so it is re-read too: after a rename it
	 *  would otherwise still name a path that no longer exists. */
	async function casesChanged() {
		await reloadCases()
		await loadResults()
		await loadRuns()
	}

	function openCase(row: ExperimentRow) {
		selectedCaseId = row.case_id
	}

	let selectedRow = $derived(displayRows.find((row) => row.case_id === selectedCaseId))

	$effect(() => {
		if (!ws) return
		untrack(() => {
			loadRuns()
			loadDatasets()
		})
	})
	$effect(() => {
		experimentId
		baselineId
		untrack(() => {
			// Opening the run that was being compared against: a run is not a comparison with itself,
			// and the picker offers every run but this one, so it would be left holding an id it has
			// no entry for.
			if (baselineId != undefined && baselineId === experimentId) {
				baselineId = undefined
				return
			}
			// Emptied before the read: one run's cells under another run's name is the one thing a
			// table of comparisons must never show.
			rows = []
			means = []
			resultsFailureReported = false
			loadResults()
		})
	})

	function subjectLabelOf(e: EvalExperiment): string {
		return subjectLabel(e, deployedHash, currentVersion)
	}

	function experimentTitle(e: EvalExperiment): string {
		return `${experimentName(e)} · ${subjectLabelOf(e)} · ${e.case_count}`
	}

	/** Where the pane is, reported to whatever frames it. */
	$effect(() => {
		const run = viewingRun ? experiment : undefined
		location = run
			? {
					label: `${experimentName(run)} · ${subjectLabelOf(run)}`,
					back: () => {
						viewingRun = false
						selectedCaseId = undefined
					}
				}
			: undefined
	})

	// The pickers inside a run offer that dataset's runs only: they select by id without bringing a
	// dataset with them, and a run of another set of cases is not a comparison for this one.
	let experimentItems = $derived(
		experiments
			.filter((e) => e.dataset === selectedDataset)
			.map((e) => ({ label: experimentTitle(e), value: e.id }))
	)

	const STATUS = {
		success: { icon: Check, class: 'text-green-500', label: 'Success' },
		failure: { icon: X, class: 'text-red-500', label: 'Failed' },
		canceled: { icon: Ban, class: 'text-yellow-500', label: 'Canceled' },
		skipped: { icon: FastForward, class: 'text-tertiary', label: 'Skipped' },
		running: { icon: Loader2, class: 'text-blue-500 animate-spin', label: 'Running' },
		not_run: { icon: Minus, class: 'text-tertiary', label: 'Not run in this run' },
		// The case ran, but its job was cleaned up before anything read what it produced.
		unavailable: { icon: Minus, class: 'text-tertiary', label: 'Not recorded' }
	} as const

	/** The one number a column reports: how many cases pass, when it has a line to pass; how they
	 *  average when it does not. The other is in the header's tooltip. */
	function columnHeadline(
		scorer: Scorer,
		mean: ScorerMean | undefined
	): { value: string; delta?: string; direction: number } | undefined {
		if (scorer.pass_if != undefined) {
			if (mean?.pass_rate == undefined) return undefined
			const delta =
				mean.baseline_pass_rate == undefined
					? undefined
					: Math.round((mean.pass_rate - mean.baseline_pass_rate) * 100)
			return {
				value: `${Math.round(mean.pass_rate * 100)}% pass`,
				delta: delta == undefined ? undefined : `${delta > 0 ? '+' : ''}${delta}`,
				direction: delta ?? 0
			}
		}
		if (mean?.mean == undefined) return undefined
		const delta = mean.baseline_mean == undefined ? undefined : mean.mean - mean.baseline_mean
		return {
			value: formatScore(mean.mean),
			delta: delta == undefined ? undefined : formatDelta(delta),
			direction: delta ?? 0
		}
	}

	/** Everything about a column that does not fit over its numbers. */
	function columnTitle(scorer: Scorer, mean: ScorerMean | undefined): string {
		const parts = [scorer.path]
		if (mean?.mean != undefined) {
			parts.push(`mean ${formatScore(mean.mean)} of ${mean.scored} scored`)
		}
		if (scorer.pass_if != undefined && mean?.pass_rate != undefined) {
			parts.push(
				`${Math.round(mean.pass_rate * mean.scored)} of ${mean.scored} at or above ${scorer.pass_if}`
			)
		}
		if (mean?.definition_changed) {
			parts.push('the scorer itself changed between these two runs')
		}
		return parts.join(' · ')
	}

	// A status this build does not know is not a run in flight: falling back to `running` would spin
	// a cell the poller has no reason to poll.
	function statusOf(status: string) {
		return STATUS[status as keyof typeof STATUS] ?? STATUS.unavailable
	}

	/** The per-assertion results a script scorer reports, when it reports any. */
	function checksOf(cell: {
		checks?: unknown
	}): { name: string; passed: boolean; detail?: string }[] {
		return Array.isArray(cell.checks) ? (cell.checks as any[]) : []
	}
</script>

<div class="flex flex-col h-full min-h-0">
	<div class="flex flex-wrap items-end gap-2 py-2">
		{#if viewingRun}
			<Label label="Run" class="w-52 shrink">
				<Select items={experimentItems} bind:value={experimentId} class="text-xs" />
			</Label>
			<Label label="Compare to" class="w-48 shrink">
				<Select
					items={experimentItems.filter((i) => i.value !== experimentId)}
					bind:value={baselineId}
					placeholder="No comparison"
					clearable
					disabled={experiments.length < 2}
					class="text-xs"
				/>
			</Label>
		{/if}
		<div class="grow"></div>
		{#if viewingRun && experiment?.run_job_id}
			<a
				class="text-xs text-accent hover:underline inline-flex items-center gap-1 shrink-0 pb-2"
				href={`${base}/run/${experiment.run_job_id}?workspace=${ws}`}
				target="_blank"
			>
				Open the job
				<ExternalLink size={12} />
			</a>
		{/if}
		{#if !viewingRun && loaded && datasets.length > 0}
			<Button
				unifiedSize="md"
				variant="default"
				startIcon={{ icon: Plus }}
				onclick={() => datasetDrawer?.openDrawer('new')}
			>
				New dataset
			</Button>
			{#if experiments.length > 0}
				<!-- Only once there is a list: with no runs the table offers this itself, where the
				     first row would be. -->
				<Button
					unifiedSize="md"
					variant="accent"
					startIcon={{ icon: Plus }}
					loading={running}
					disabled={running || !agentPath}
					onclick={() => (runDialogOpen = true)}
				>
					New evaluation
				</Button>
			{/if}
		{/if}
	</div>

	<div class="grow min-h-0">
		<Splitpanes class="h-full">
			<Pane size={selectedRow ? 60 : 100} minSize={35}>
				<div class="h-full overflow-auto">
					{#if loaded && loadError}
						<div class="h-full flex flex-col items-center justify-center gap-2 p-6 text-center">
							<span class="text-sm text-emphasis">Could not load evals</span>
							<span class="text-xs text-secondary max-w-md">
								The datasets or runs could not be read. Check your access to this agent and reload.
							</span>
						</div>
					{:else if loaded && datasets.length === 0}
						<div class="h-full flex flex-col items-center justify-center gap-3 p-6 text-center">
							<span class="text-sm text-emphasis">No dataset yet</span>
							<span class="text-xs text-secondary max-w-md">
								A dataset is the set of cases this agent is measured on. Runs are of a dataset, so
								it is the first thing to make.
							</span>
							<Button
								unifiedSize="md"
								variant="accent"
								startIcon={{ icon: Plus }}
								onclick={() => datasetDrawer?.openDrawer('new')}
							>
								New dataset
							</Button>
						</div>
					{:else if !viewingRun || !loaded}
						<EvalRunsList
							{experiments}
							{datasets}
							{loaded}
							{deployedHash}
							{currentVersion}
							onOpen={(e) => openRun(e.id)}
							onEditDataset={async (path) => {
								if (await useDataset(path)) datasetDrawer?.openDrawer('edit')
							}}
							onNew={() => (runDialogOpen = true)}
						/>
					{:else}
						<DataTable size="sm" tableFixed rounded={!selectedRow}>
							<colgroup>
								<col style="width: 24%" />
								<col style="width: 32%" />
								{#each scorers as scorer (scorer.id)}
									<col style="width: 9rem" />
								{/each}
							</colgroup>
							<Head>
								<tr>
									<Cell head first>Case</Cell>
									<Cell head last={scorers.length === 0}>Answer</Cell>
									{#each scorers as scorer, index (scorer.id)}
										{@const mean = means.find((m) => m.scorer_id === scorer.id)}
										{@const headline = columnHeadline(scorer, mean)}
										<Cell head numeric last={index === scorers.length - 1}>
											<!-- The second row keeps its height while there is nothing in it, so the table
											     does not move when the first score lands. -->
											<div class="flex flex-col items-end min-w-0 w-full overflow-hidden">
												<span
													class="flex items-center gap-1 min-w-0 max-w-full"
													title={columnTitle(scorer, mean)}
												>
													{#if scorer.kind === 'agent'}
														<Bot size={13} class="text-tertiary shrink-0" />
													{:else}
														<Code2 size={13} class="text-tertiary shrink-0" />
													{/if}
													<span class="truncate min-w-0">{scorerLabel(scorer)}</span>
												</span>
												<span class="h-4 flex items-baseline gap-1.5 font-normal">
													{#if headline}
														<span class="tabular-nums text-emphasis font-semibold">
															{headline.value}
														</span>
														{#if headline.delta && headline.direction !== 0}
															<span
																class={`text-2xs tabular-nums ${headline.direction > 0 ? 'text-green-500' : headline.direction < 0 ? 'text-red-500' : 'text-tertiary'}`}
															>
																{headline.delta}
															</span>
														{/if}
													{/if}
												</span>
											</div>
										</Cell>
									{/each}
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each displayRows as row (row.case_id)}
									{@const status = statusOf(row.status)}
									<Row
										hoverable
										selected={row.case_id === selectedCaseId}
										on:click={() => openCase(row)}
									>
										<Cell first>
											<span class="truncate block text-emphasis">{caseLabel(row)}</span>
										</Cell>
										<Cell last={scorers.length === 0}>
											<span
												class="flex items-center gap-1.5 min-w-0"
												title={row.subject_version
													? `${status.label} · v${row.subject_version}`
													: status.label}
											>
												<status.icon size={14} class={`shrink-0 ${status.class}`} />
												{#if row.output != undefined}
													<span class="truncate text-secondary">{row.output}</span>
												{:else if status === STATUS.not_run}
													<span class="text-2xs text-tertiary">not run</span>
												{:else}
													<span class="text-2xs text-tertiary">{status.label.toLowerCase()}</span>
												{/if}
											</span>
										</Cell>
										{#each scorers as scorer, index (scorer.id)}
											{@const cell = row.scores.find((s) => s.scorer_id === scorer.id)}
											<Cell numeric last={index === scorers.length - 1}>
												{#if cell?.pending}
													<span class="inline-flex justify-end" title="Scoring">
														<Loader2 size={13} class="animate-spin text-blue-500" />
													</span>
												{:else if cell?.score != undefined}
													<Popover placement="left">
														{#snippet text()}
															<div class="flex flex-col gap-2 max-w-80 text-left">
																{#if cell.reason}
																	<span class="text-xs">{cell.reason}</span>
																{/if}
																{#each checksOf(cell) as check (check.name)}
																	<span class="text-2xs flex items-baseline gap-1.5">
																		<span class={check.passed ? 'text-green-500' : 'text-red-500'}>
																			{check.passed ? '✓' : '✗'}
																		</span>
																		<span>{check.name}</span>
																		{#if check.detail}
																			<span class="text-tertiary">{check.detail}</span>
																		{/if}
																	</span>
																{/each}
															</div>
														{/snippet}
														<span class="inline-flex items-baseline gap-1.5 justify-end">
															{#if cell.passed != undefined}
																<span
																	class={cell.passed ? 'text-green-500' : 'text-red-500'}
																	title={`${cell.passed ? 'Passed' : 'Failed'}: the threshold is ${scorer.pass_if}`}
																>
																	{cell.passed ? '✓' : '✗'}
																</span>
															{/if}
															<span class="tabular-nums font-medium text-emphasis">
																{formatScore(cell.score)}
															</span>
															{#if cell.baseline != undefined && cell.score !== cell.baseline}
																{@const delta = cell.score - cell.baseline}
																<span
																	class={`text-2xs tabular-nums ${delta > 0 ? 'text-green-500' : 'text-red-500'}`}
																>
																	{formatDelta(delta)}
																</span>
															{/if}
														</span>
													</Popover>
												{:else if cell?.not_applicable}
													<Popover placement="left" disablePopup={!cell.reason}>
														{#snippet text()}
															<span class="text-xs max-w-80 text-left">{cell.reason}</span>
														{/snippet}
														<span class="text-2xs text-tertiary" title="Not measured on this case">
															n/a
														</span>
													</Popover>
												{:else if cell?.error}
													<Popover placement="left">
														{#snippet text()}
															<span class="text-xs max-w-80 text-left">{cell.error}</span>
														{/snippet}
														<span class="text-2xs text-red-500">failed</span>
													</Popover>
												{:else}
													<span class="text-2xs text-tertiary">—</span>
												{/if}
											</Cell>
										{/each}
									</Row>
								{/each}
							</tbody>
						</DataTable>
					{/if}
				</div>
			</Pane>
			{#if selectedRow}
				{@const openRow = selectedRow}
				<Pane size={40} minSize={25}>
					<div class="h-full overflow-auto flex flex-col">
						<div class="flex items-start gap-2 px-3 py-2 border-b">
							<span class="text-xs font-semibold text-emphasis break-words">
								{openRow.input?.user_message ?? caseLabel(openRow)}
							</span>
							<div class="grow"></div>
							{#if openRow.job_id}
								<a
									class="text-2xs text-accent hover:underline inline-flex items-center gap-1 shrink-0 mt-0.5"
									href={`${base}/run/${openRow.job_id}?workspace=${ws}`}
									target="_blank"
								>
									Open the case job
									<ExternalLink size={12} />
								</a>
							{/if}
							<Button
								unifiedSize="sm"
								variant="subtle"
								startIcon={{ icon: X }}
								iconOnly
								title="Close"
								onclick={() => (selectedCaseId = undefined)}
							/>
						</div>
						<div class="p-3 flex flex-col gap-4">
							{#if openRow.expected != undefined && openRow.expected !== ''}
								<Label label="Expected">
									<span class="text-xs text-secondary whitespace-pre-wrap break-words">
										{typeof openRow.expected === 'string'
											? openRow.expected
											: JSON.stringify(openRow.expected, null, 2)}
									</span>
								</Label>
							{/if}
							{#if scorers.length > 0 && openRow.scores.length > 0}
								<Label label="Scores">
									<div class="flex flex-col divide-y border rounded-md">
										{#each scorers as scorer (scorer.id)}
											{@const cell = openRow.scores.find((s) => s.scorer_id === scorer.id)}
											<div class="flex flex-col gap-1 px-2 py-1.5">
												<div class="flex items-center gap-2 min-w-0">
													{#if scorer.kind === 'agent'}
														<Bot size={13} class="text-tertiary shrink-0" />
													{:else}
														<Code2 size={13} class="text-tertiary shrink-0" />
													{/if}
													<span class="text-xs text-emphasis truncate min-w-0">
														{scorerLabel(scorer)}
													</span>
													<div class="grow"></div>
													{#if cell?.pending || (!cell && openRow.status === 'running')}
														<Loader2 size={12} class="animate-spin text-blue-500 shrink-0" />
													{:else if cell?.score != undefined}
														<span
															class="text-xs tabular-nums font-semibold shrink-0 {cell.passed ===
															true
																? 'text-green-600'
																: cell.passed === false
																	? 'text-red-600'
																	: 'text-emphasis'}"
														>
															{formatScore(cell.score)}
														</span>
													{:else if cell?.not_applicable}
														<span class="text-2xs text-tertiary shrink-0">n/a</span>
													{:else}
														<span class="text-2xs text-tertiary shrink-0">no score</span>
													{/if}
												</div>
												{#if cell?.error}
													<span class="text-2xs text-red-500 break-words">{cell.error}</span>
												{:else if cell?.reason}
													<span class="text-2xs text-tertiary break-words">{cell.reason}</span>
												{/if}
											</div>
										{/each}
									</div>
								</Label>
							{/if}
							{#if experiment && (openRow.job_id || openRow.output != undefined)}
								<div class="rounded-md border border-light overflow-hidden">
									<div
										class="flex items-center gap-2 px-2 py-1 border-b border-light bg-surface-secondary"
									>
										<span class="text-2xs font-semibold text-secondary truncate">
											Case result
										</span>
									</div>
									<div class="p-2">
										{#if openRow.output != undefined}
											<div class="text-xs text-secondary break-words">
												<GfmMarkdown md={openRow.output} noPadding />
											</div>
										{:else if openRow.status === 'running'}
											<span class="text-xs text-tertiary inline-flex items-center gap-1.5">
												<Loader2 size={12} class="animate-spin text-blue-500" />
												Running
											</span>
										{:else}
											<span class="text-xs text-tertiary">{statusOf(openRow.status).label}</span>
										{/if}
									</div>
								</div>
							{/if}
						</div>
					</div>
				</Pane>
			{/if}
		</Splitpanes>
	</div>
</div>

<EvalRunDialog
	bind:open={runDialogOpen}
	workspace={ws}
	{agentPath}
	{datasets}
	defaultDataset={selectedDataset}
	{editedConfig}
	{running}
	onRun={runAll}
	onEditDataset={async (path) => {
		if (await useDataset(path)) {
			resumeRunDialog = true
			datasetDrawer?.openDrawer('edit')
		}
	}}
	onNewDataset={() => {
		resumeRunDialog = true
		datasetDrawer?.openDrawer('new')
	}}
/>

<EvalDatasetDrawer
	bind:this={datasetDrawer}
	workspace={ws}
	{agentPath}
	datasetPath={selectedDataset}
	{dataset}
	{datasets}
	cases={Object.values(storedCases)}
	onCreated={selectSavedDataset}
	onRenamed={selectSavedDataset}
	onDeleted={datasetDeleted}
	onCasesChanged={casesChanged}
	onScorersChanged={scorersChanged}
	onClosed={() => {
		if (!resumeRunDialog) return
		resumeRunDialog = false
		// On the dataset the drawer was just in: the dialog opens on the pane's own, which
		// creating or editing one has already moved to it.
		runDialogOpen = true
	}}
/>
