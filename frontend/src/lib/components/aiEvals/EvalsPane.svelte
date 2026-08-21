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
	import { onDestroy, untrack } from 'svelte'
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
	import { caseLabel } from './evalCaseUtils'
	import type { EvalsLocation } from './evalRuns'
	import { experimentName, subjectLabel } from './evalRuns'
	import { formatDelta, formatScore, scorerLabel } from './evalScorers'

	let {
		agentPath,
		opWorkspace = undefined,
		editedConfig = undefined,
		location = $bindable()
	}: {
		/** The agent under test. A dataset and its runs belong to an agent, so an agent that has
		 * never been saved has nothing to hang them on. */
		agentPath: string
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
		/** Opened from an agent being edited: the edits, as the step holds them, are what a run is
		 * offered on. Everywhere else the agent is what is deployed. */
		editedConfig?: () => AgentDraft
		/** The level the pane is on and the way out of it, reported up so the surface holding it
		 * can put both in its header. Undefined at the root, which that surface already names.
		 * The pane navigates; where that shows belongs to whoever owns the frame. */
		location?: EvalsLocation
	} = $props()

	let ws = $derived(opWorkspace ?? $workspaceStore)
	let datasets = $state<EvalDataset[]>([])
	let dataset = $state<EvalDataset | undefined>(undefined)
	let selectedDataset = $state<string | undefined>(undefined)
	let experiments = $state<EvalExperiment[]>([])
	// The drawer opens on the dataset's runs and a run is opened from there: what the table shows
	// only means something once you have said which run you are reading. The run being read is
	// `experimentId`; this is only whether one is.
	let viewingRun = $state(false)
	let experimentId = $state<string | undefined>(undefined)
	let baselineId = $state<string | undefined>(undefined)
	let rows = $state<ExperimentRow[]>([])
	// The dataset's cases as they are now, keyed by id. A row carries the case *as the experiment
	// ran it*, which is what results must show and exactly what must not be written back: editing
	// from a row would save a stale input and drop the fields a row does not carry.
	let storedCases = $state<Record<string, EvalCase>>({})
	let means = $state<ScorerMean[]>([])
	/** The version the agent is on now, against which a row's own version is stale or current. */
	let currentVersion = $state<number | undefined>(undefined)
	/** What the agent hashes to as deployed: a run of edits carrying it ran what was then saved. */
	let deployedHash = $state<string | undefined>(undefined)
	let running = $state(false)
	let scorers = $derived(dataset?.scorers ?? [])
	let selectedCaseId = $state<string | undefined>(undefined)

	// The dataset is edited on top of the runs rather than in among them: what a run measured and
	// what the next one will are two different questions, and the table answers the first.
	let datasetDrawer: EvalDatasetDrawer | undefined = $state()
	// What to run is asked rather than assumed: which state of the agent, and against which
	// dataset. Both cost a provider bill, and neither follows from where you were standing.
	let runDialogOpen = $state(false)
	// The dataset drawer was reached from the dialog, which had to give up the screen to it. Naming
	// a dataset there is a detour on the way to a run, so the way back is taken for you.
	let resumeRunDialog = $state(false)

	let experiment = $derived(experiments.find((e) => e.id === experimentId))

	// One agent, one history: runs of what is deployed and runs of the edits waiting on top of it
	// are both this agent's, and each says which it was. Keeping them in separate lists would only
	// hide the comparison that is the point of running the draft at all.
	async function listSubjectExperiments(): Promise<EvalExperiment[]> {
		if (!ws) return []
		return await AiEvalsService.listAllExperiments({ workspace: ws, subjectPath: agentPath })
	}

	/** Whether the two lists the pane opens on have been read. Every empty state here is a
	 *  statement — this agent has never been run, this workspace has no dataset — and neither is
	 *  something to say while the answer is still on its way. */
	let runsLoaded = $state(false)
	let datasetsLoaded = $state(false)
	let loaded = $derived(!ws || (runsLoaded && datasetsLoaded))

	/** The agent's whole history, which is the screen the pane opens on. */
	async function loadRuns() {
		try {
			experiments = await listSubjectExperiments()
		} finally {
			runsLoaded = true
		}
	}

	// Which dataset this subject was last worked in. Opening on someone else's dataset would read
	// as this agent's history when it is not, but opening on the one you were in yesterday is
	// exactly where you left off.
	let lastDatasetKey = $derived(`evals:dataset:${ws}:${agentPath}`)
	// Only ever written, never cleared: the pane mounts with nothing selected, and clearing on that
	// would erase the memory a moment before the load that reads it.
	function rememberDataset(path: string | undefined) {
		if (!path) return
		try {
			localStorage.setItem(lastDatasetKey, path)
		} catch {
			// Storage is a convenience here: a browser refusing it costs the memory, not the pane.
		}
	}

	async function loadDatasets() {
		if (!ws) return
		try {
			datasets = await AiEvalsService.listEvalDatasets({ workspace: ws })
		} finally {
			datasetsLoaded = true
		}
		if (selectedDataset) return
		let remembered: string | null = null
		try {
			remembered = localStorage.getItem(lastDatasetKey)
		} catch {
			// See above.
		}
		// Only if it is still there: a dataset that was deleted or is no longer readable must not
		// leave the run dialog offering one that has gone. Brought into context rather than merely
		// selected, so that what is selected is always something the pane has actually read.
		if (remembered && datasets.some((d) => d.path === remembered)) {
			await useDataset(remembered)
		}
	}

	/**
	 * Bring a dataset into context: its metadata, its scorers and its cases.
	 *
	 * Called explicitly — opening a run, running one, editing one — rather than from an effect on
	 * the selection. The load resets which run is being read, so an effect would race every caller
	 * that sets the dataset and then opens a run of it.
	 */
	async function useDataset(path: string) {
		selectedDataset = path
		rememberDataset(path)
		await loadDataset(path)
	}

	// Switching datasets leaves the previous request in flight; only the newest may write, or a
	// slow response for the dataset you just left replaces the one you are looking at.
	let loadGeneration = 0

	async function loadDataset(path: string | undefined) {
		const generation = ++loadGeneration
		if (!ws || !path) {
			dataset = undefined
			storedCases = {}
			return
		}
		// Deliberately not the pane's `loading`: opening a dataset to edit it happens over the runs
		// list, and blanking that list to a skeleton makes the click read as navigation.
		try {
			const [row, cases] = await Promise.all([
				AiEvalsService.getEvalDataset({ workspace: ws, path }),
				AiEvalsService.listEvalCases({ workspace: ws, path, perPage: CASE_PAGE_SIZE })
			])
			if (generation !== loadGeneration) return
			dataset = row
			storedCases = Object.fromEntries(cases.cases.map((c) => [c.id, c]))
		} catch (e) {
			if (generation === loadGeneration) {
				sendUserToast(`Failed to load ${path}: ${e}`, true)
			}
		}
	}

	const CASE_PAGE_SIZE = 1000

	async function reloadCases() {
		if (!ws || !selectedDataset) return
		const cases = await AiEvalsService.listEvalCases({
			workspace: ws,
			path: selectedDataset,
			perPage: CASE_PAGE_SIZE
		})
		storedCases = Object.fromEntries(cases.cases.map((c) => [c.id, c]))
	}

	/**
	 * What is on screen, and which request may write it.
	 *
	 * The run picker, the baseline picker and the 2s poller all call this, so responses overlap:
	 * the generation drops a superseded one, and the key — which run, against which baseline —
	 * empties the table when it changes, since one run's cells under another run's name is the one
	 * thing a table of comparisons must never show.
	 */
	let resultsGeneration = 0
	let renderedResults: string | undefined = undefined
	let reportedResultsFailure: string | undefined = undefined

	async function loadResults() {
		const generation = ++resultsGeneration
		if (!ws || !selectedDataset || !experimentId) {
			rows = []
			means = []
			renderedResults = undefined
			return
		}
		// Joined on a separator neither a path nor an id can contain, spelled as an escape:
		// a raw NUL in the source makes this file binary to grep.
		const key = [selectedDataset, experimentId, baselineId ?? ''].join('\u0000')
		if (key !== renderedResults) {
			rows = []
			means = []
			renderedResults = undefined
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
			renderedResults = key
			reportedResultsFailure = undefined
		} catch (e) {
			if (generation !== resultsGeneration) return
			// Said once per run: the poller comes back every 2s, and a run that cannot be read is a
			// run that cannot be read again.
			if (reportedResultsFailure !== key) {
				reportedResultsFailure = key
				sendUserToast(`Failed to read the run: ${e}`, true)
			}
		}
	}

	/**
	 * What the table renders: every case of the dataset, in dataset order, carrying its result in
	 * the selected experiment when there is one. A case list that only appeared once something had
	 * been run would leave a dataset looking empty in exactly the state where you want to press
	 * Run. Cases the experiment ran but the dataset no longer has keep their row at the end: the
	 * run happened, and deleting the case does not unmake it.
	 */
	let displayRows: ExperimentRow[] = $derived.by(() => {
		const byCase = new Map(rows.map((row) => [row.case_id, row]))
		const ordered: ExperimentRow[] = Object.values(storedCases).map(
			(stored) =>
				byCase.get(stored.id) ?? {
					case_id: stored.id,
					name: stored.name,
					input: stored.input ?? {},
					expected: stored.expected,
					job_id: '',
					status: 'not_run' as ExperimentRow['status'],
					scores: []
				}
		)
		return [...ordered, ...rows.filter((row) => !storedCases[row.case_id])]
	})

	/**
	 * The version the agent is on right now, so a run's label can say it is of an earlier one.
	 * Read on its own small endpoint rather than from the results, which harvest scores and read
	 * every job; asked when the pane opens and when the tab comes back, which is when a save made
	 * elsewhere is most likely waiting.
	 */
	async function readSubjectState() {
		if (!ws || !agentPath || document.hidden) return
		try {
			const state = await AiEvalsService.evalSubjectState({ workspace: ws, path: agentPath })
			currentVersion = state.version
		} catch {
			// The agent was deleted or is no longer readable: the table keeps what it last knew
			// rather than claiming everything went stale.
		}
	}
	$effect(() => {
		agentPath
		untrack(() => readSubjectState())
	})
	// Coming back to the tab is the moment an edit made elsewhere is most likely waiting, and the
	// same is true of a run: the poller only arms for a run this pane already knows about, so one
	// started from another tab would otherwise never appear on a list left open.
	$effect(() => {
		const onFocus = () =>
			untrack(() => {
				readSubjectState()
				refresh()
			})
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

	/**
	 * Read what the run has produced so far. Nothing here drives it: a run answers and scores
	 * itself on workers, so closing this pane costs the numbers nothing, and reopening it shows
	 * where the run got to.
	 */
	// One pass at a time. The poller fires every 2s whether or not the last read came back, and
	// every read supersedes the one before it — so a read slower than the interval would be
	// discarded by the next one forever, and the table would never advance.
	let refreshing = false
	async function refresh() {
		if (!ws || refreshing) return
		refreshing = true
		try {
			// On the list, the run in flight is a row whose scores are still arriving; in a run, it
			// is the table. Reading both would read every cell of a run nobody is looking at.
			if (viewingRun) {
				await loadResults()
			} else {
				experiments = await listSubjectExperiments()
			}
		} finally {
			refreshing = false
		}
	}

	/**
	 * Opening a run is what the table is of, and a run of a dataset the pane is not in brings that
	 * dataset with it: the table is the run's cells against that dataset's cases and columns.
	 *
	 * The run before it in the list is the obvious thing to compare against, offered rather than
	 * imposed — the picker beside it can clear it. Reading the cells is left to the effect watching
	 * which run is selected, so the list, the picker and a fresh run all open one the same way.
	 */
	async function openRun(id: string) {
		// Against the dataset that is loaded, not the one that is selected: the two are the same
		// once a dataset has been brought into context, and skipping on the selection alone would
		// leave a run open over a dataset whose cases and scorers were never read.
		const target = experiments.find((e) => e.id === id)
		if (target && target.dataset !== dataset?.path) {
			await useDataset(target.dataset)
		}
		const index = experiments.findIndex((e) => e.id === id)
		// Against the run before it *of the same dataset*: the list spans datasets, and a run of
		// another set of cases is not a baseline for this one.
		baselineId = experiments.slice(index + 1).find((e) => e.dataset === target?.dataset)?.id
		experimentId = id
		viewingRun = true
		selectedCaseId = undefined
	}

	/** Runs a dataset against a chosen state of the agent, as the run dialog asked for it. */
	async function runAll(runSubject: EvalSubject, path: string) {
		if (!ws || !path) return
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
			return
		}
		// From here the run exists and is billing: what can still fail is reading it back, and
		// saying "failed to run" to that invites a second, duplicate run.
		try {
			// Running a dataset is also choosing it: what you started is what the pane is now about.
			if (path !== dataset?.path) await useDataset(path)
			await loadRuns()
			// Straight into the run that was just started: it is the one thing on screen that is
			// still changing, and watching it is why you pressed Run.
			await openRun(id)
		} catch (e) {
			sendUserToast(
				`The run started but could not be read back: ${e}. Reload the runs list to see it.`,
				true
			)
		} finally {
			running = false
		}
	}

	/** The dataset a run is of, once it has been created or moved: both are a different list of
	 *  datasets and a different thing selected in it. */
	async function selectSavedDataset(path: string) {
		await loadDatasets()
		await useDataset(path)
	}

	/** The columns changed: the dataset is re-read for them, and the run on screen is re-read
	 *  through them — a pass line that moved re-reads every score already recorded. */
	async function scorersChanged() {
		if (selectedDataset) await loadDataset(selectedDataset)
		await loadResults()
		await loadRuns()
	}

	/** Curating the dataset changes what the table lists, and a case that was edited is a case the
	 *  recorded runs no longer ran: the rows say so once they are read again. The runs list names
	 *  the dataset and counts its cases, so it is re-read too — after a rename it would otherwise
	 *  still name a path that no longer exists. */
	async function casesChanged() {
		await reloadCases()
		await loadResults()
		await loadRuns()
	}

	/** A row of the table is a case as one run executed it, which is what its panel shows. Editing
	 *  it is editing the dataset, one drawer up. */
	function openCase(row: ExperimentRow) {
		selectedCaseId = row.case_id
	}

	let selectedRow = $derived(displayRows.find((row) => row.case_id === selectedCaseId))

	$effect(() => {
		if (!ws) return
		untrack(() => {
			// The runs are the screen; the datasets are what the run dialog offers, and what the
			// remembered one is checked against.
			loadRuns()
			loadDatasets()
		})
	})
	$effect(() => {
		// Picking a run is asking to see it, and picking a baseline is what turns every column into
		// a comparison. Both are a different table.
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
			loadResults()
		})
	})

	/**
	 * What a run is called in the picker: the run it is, what ran it, and how many cases it holds.
	 * What ran is named rather than marked, because a sigil beside a version is a legend nobody has.
	 */
	function experimentTitle(e: EvalExperiment): string {
		return `${experimentName(e)} · ${subjectLabelOf(e)} · ${e.case_count}`
	}

	let subjectLabelOf = $derived((e: EvalExperiment) =>
		subjectLabel(e, deployedHash, currentVersion)
	)

	/** Where the pane is, reported to whatever frames it. Named as the run picker names a run,
	 *  without the case count: a header says where you are, not everything the row said. */
	$effect(() => {
		const run = viewingRun ? experiments.find((e) => e.id === experimentId) : undefined
		location = run
			? {
					label: `${experimentName(run)} · ${subjectLabelOf(run)}`,
					// The panel showed a case of this run, so it closes with the run rather than hanging
					// over the list that replaces it.
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

	/**
	 * The one number a column reports: how many cases pass, when it has a line to pass; how they
	 * average when it does not. The other is in the header's tooltip.
	 *
	 * One line rather than both, because the header is as tall as the number of lines in it and the
	 * first score landing must not move the table under the reader.
	 */
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

	function statusOf(status: string) {
		return STATUS[status as keyof typeof STATUS] ?? STATUS.running
	}

	/** The per-assertion results a script scorer reports, when it reports any. */
	function checksOf(cell: {
		checks?: unknown
	}): { name: string; passed: boolean; detail?: string }[] {
		return Array.isArray(cell.checks) ? (cell.checks as any[]) : []
	}
</script>

<div class="flex flex-col h-full min-h-0">
	<!-- What you are looking at on the first row, what changes it on the second: two kinds of
	     control that read badly interleaved, and the row of pickers no longer wraps.

	     Nothing on the list: which run, and which dataset it was of, is what the rows say. The
	     pickers belong to a run being read, so they arrive with one. -->
	<div class="flex flex-wrap items-end gap-2 py-2">
		{#if viewingRun}
			<!-- Which run, and what it is read against, side by side: they are one question asked
			     twice, and a comparison you cannot see the control for is one nobody knows they can
			     make. The way back is in the dialog's header, which does not move with this row. -->
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
			<!-- The run is one flow, so it has a job: what it is doing, what it cost and what it
			     logged are all there rather than reconstructed here. -->
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
			<!-- Beside starting a run rather than only inside the dialog that starts one: a dataset
			     is a set someone curates between runs, and reaching it through the run they are not
			     making yet is a detour. Secondary, because a run is what this screen is a list of.
			     Absent until there is a dataset: the empty state below is then the one move. -->
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
				     first row would be. Named for what it opens rather than for what that then does:
				     it asks which state of the agent and which dataset, and both cost a provider
				     bill, so a button that reads as spending one on the way past would be lying
				     about the click. -->
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
					{#if loaded && datasets.length === 0}
						<!-- Nothing to run and nothing to have run: the first dataset is the only move. -->
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
						<!-- What this agent has already been measured at, across every dataset it has been
						     measured on: a run is worth reading against the ones before it, and that is a
						     list before it is a table. It is also what the pane shows while it reads: the
						     table is the screen, and the states around it are answers it does not have yet. -->
						<EvalRunsList
							{experiments}
							{datasets}
							{loaded}
							onOpen={(e) => openRun(e.id)}
							onEditDataset={async (path) => {
								await useDataset(path)
								datasetDrawer?.openDrawer('edit')
							}}
							onNew={() => (runDialogOpen = true)}
						/>
					{:else}
						<!-- Square against the panel: a rounded corner there reads as the table ending, when
						     what is beside it is the row it opened. -->
						<DataTable size="sm" tableFixed rounded={!selectedRow}>
							<!-- A score column is as wide as a score and its column name need; the question and the
							     answer share what is left. Sized rather than divided equally, because the text will
							     take any width it is given and leave the numbers squeezed against each other. -->
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
											<!-- Two rows, always: the name, and the number under the name it is a number of.
											     The second row keeps its height while there is nothing in it, so the table
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
											<!-- The answer, with what became of the job that produced it in front of it.
											     One column rather than two: a status beside an empty cell says the same
											     thing twice, and a status beside an answer says nothing the answer does
											     not. The spin is on the icon, not on the cell around it. -->
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
													<!-- The number is the verdict; why it was given is the part worth reading, so
													     it is one hover away rather than in a browser tooltip that arrives after a
													     second and wraps at 80 columns. -->
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
													<!-- A verdict, so it is said rather than left as the dash of a cell
													     nothing reached. The column's mean is of the cases it measured. -->
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
							<!-- The case itself, in full: it is the question this panel is about, and the
							     table beside it is where it is abbreviated. -->
							<span class="text-xs font-semibold text-emphasis break-words">
								{openRow.input?.user_message ?? caseLabel(openRow)}
							</span>
							<div class="grow"></div>
							{#if openRow.job_id}
								<!-- Over the panel rather than over the answer: the job is this case as this run
								     executed it, the agent and the scorers that read it, which is everything the
								     panel is showing and not the answer alone. -->
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
								<!-- What each column made of this case, and why. A scorer is a step inside the
								     case's own job, which the panel's header opens; what is worth having here is
								     the number with the reasoning beside it. -->
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
								<!-- The answer as the run recorded it, which is what the agent returned and not
								     what the job as a whole did. -->
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
											<!-- Rendered: an agent writes prose, and its own headings and lists are how it
											     meant the answer to be read. -->
											<div class="text-xs text-secondary break-words">
												<GfmMarkdown md={openRow.output} noPadding />
											</div>
										{:else if openRow.status === 'running'}
											<!-- A wait, not an answer: the word alone sat exactly where the answer goes
											     and read as one. -->
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
		resumeRunDialog = true
		await useDataset(path)
		datasetDrawer?.openDrawer('edit')
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
