<script lang="ts">
	import { base } from '$lib/base'
	import { Alert, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Label from '$lib/components/Label.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import {
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
	import { caseLabel, type CaseDraft } from './evalCaseUtils'
	import type { EvalsLocation } from './evalRuns'
	import { experimentName, subjectLabel } from './evalRuns'
	import { formatDelta, formatScore, scorerLabel } from './evalScorers'

	let {
		agentPath,
		opWorkspace = undefined,
		capture = undefined,
		location = $bindable()
	}: {
		/** The agent under test. A dataset and its runs belong to an agent, so an agent that has
		 * never been saved has nothing to hang them on. */
		agentPath: string
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
		/** A case captured from an AI agent run, opened for review before saving. */
		capture?: CaseDraft
		/** The level the pane is on and the way out of it, reported up so the surface holding it
		 * can put both in its header. Undefined at the root, which that surface already names.
		 * The pane navigates; where that shows belongs to whoever owns the frame. */
		location?: EvalsLocation
	} = $props()

	let ws = $derived(opWorkspace ?? $workspaceStore)
	// Running the agent's draft is a subject of its own: the server inlines the draft's value, so
	// the run executes what is being edited rather than what is deployed, and its history stays
	// separate from the deployed agent's.
	let runDraft = $state(false)
	let subject = $derived({
		kind: (runDraft ? 'agent_draft' : 'agent') as 'agent' | 'agent_draft',
		path: agentPath
	})
	// An agent with edits waiting is tested on the edits. Running what they replace would answer a
	// question nobody asked, and the deployed value's numbers are already in the history from before
	// the editing started. Held as state rather than derived so the reload below has one thing to
	// watch.
	$effect(() => {
		const drafted = undeployedChanges
		untrack(() => {
			if (runDraft !== drafted) runDraft = drafted
		})
	})
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
	/** What the agent's draft hashes to now, against which a draft run's own hash is stale. */
	let currentDraftHash = $state<string | undefined>(undefined)
	/** What the agent hashes to as deployed: a draft run carrying it ran what was then saved. */
	let deployedHash = $state<string | undefined>(undefined)
	// A run resolves the agent live, so it executes what is deployed: edits sitting in a draft are
	// not what any of these numbers describe.
	let undeployedChanges = $state(false)
	let running = $state(false)
	let scorers = $derived(dataset?.scorers ?? [])
	// Which columns have scoring in flight. Rescoring a deterministic scorer lands on the same
	// numbers, so without this the only sign it ran is a toast that has already gone.
	let scoringColumns = $derived(
		new Set(rows.flatMap((row) => row.scores.filter((s) => s.pending).map((s) => s.scorer_id)))
	)

	let selectedCaseId = $state<string | undefined>(undefined)

	// The dataset is edited on top of the runs rather than in among them: what a run measured and
	// what the next one will are two different questions, and the table answers the first.
	let datasetDrawer: EvalDatasetDrawer | undefined = $state()
	// What to run is asked rather than assumed: which state of the agent, and against which
	// dataset. Both cost a provider bill, and neither follows from where you were standing.
	let runDialogOpen = $state(false)

	let experiment = $derived(experiments.find((e) => e.id === experimentId))

	// One agent, one history: runs of what is deployed and runs of the edits waiting on top of it
	// are both this agent's, and each says which it was. Keeping them in separate lists would only
	// hide the comparison that is the point of running the draft at all.
	async function listSubjectExperiments(): Promise<EvalExperiment[]> {
		if (!ws) return []
		return await AiEvalsService.listAllExperiments({ workspace: ws, subjectPath: agentPath })
	}

	/** The agent's whole history, which is the screen the pane opens on. */
	async function loadRuns() {
		experiments = await listSubjectExperiments()
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
		datasets = await AiEvalsService.listEvalDatasets({ workspace: ws })
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

	async function loadResults() {
		if (!ws || !selectedDataset || !experimentId) {
			rows = []
			means = []
			return
		}
		const generation = loadGeneration
		const results = await AiEvalsService.experimentResults({
			workspace: ws,
			path: selectedDataset,
			id: experimentId,
			baseline: baselineId
		})
		if (generation !== loadGeneration) return
		rows = results.rows
		means = results.means
		currentVersion = results.subject_current_version
		currentDraftHash = results.subject_current_draft_hash
		deployedHash = results.subject_deployed_hash
		undeployedChanges = results.subject_has_undeployed_changes
		if (dataset) dataset = { ...dataset, scorers: results.scorers }
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
	 * The agent is edited elsewhere — another tab, the drawer, a colleague — and the table has to
	 * notice: a run's numbers stop describing the agent the moment it moves. Watched on its own
	 * small endpoint rather than by re-reading the results, which harvest scores and read every
	 * job, and paused while the tab is hidden.
	 */
	const SUBJECT_WATCH_MS = 5000
	let subjectWatch: ReturnType<typeof setInterval> | undefined = undefined
	async function readSubjectState() {
		if (!ws || !subject.path || document.hidden) return
		try {
			const state = await AiEvalsService.evalSubjectState({ workspace: ws, path: agentPath })
			currentVersion = state.version
			currentDraftHash = state.draft_hash
			undeployedChanges = state.has_undeployed_changes
		} catch {
			// The agent was deleted or is no longer readable: the table keeps what it last knew
			// rather than claiming everything went stale.
		}
	}
	$effect(() => {
		agentPath
		untrack(() => {
			clearInterval(subjectWatch)
			readSubjectState()
			subjectWatch = setInterval(() => untrack(() => readSubjectState()), SUBJECT_WATCH_MS)
		})
	})
	onDestroy(() => clearInterval(subjectWatch))
	// Coming back to the tab is the moment an edit made elsewhere is most likely waiting.
	$effect(() => {
		const onFocus = () => untrack(() => readSubjectState())
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
	async function refresh() {
		if (!ws || !selectedDataset) return
		// On the list, the run in flight is a row whose scores are still arriving; in a run, it is
		// the table. Reading both would read every cell of a run nobody is looking at.
		if (viewingRun) {
			await loadResults()
		} else {
			experiments = await listSubjectExperiments()
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
		try {
			const id = await AiEvalsService.runExperiment({
				workspace: ws,
				requestBody: { dataset: path, subject: runSubject }
			})
			// Running a dataset is also choosing it: what you started is what the pane is now about.
			if (path !== dataset?.path) await useDataset(path)
			await loadRuns()
			// Straight into the run that was just started: it is the one thing on screen that is
			// still changing, and watching it is why you pressed Run.
			await openRun(id)
		} catch (e) {
			sendUserToast(`Failed to run the dataset: ${e}`, true)
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
	 *  recorded runs no longer ran: the rows say so once they are read again. */
	async function casesChanged() {
		await reloadCases()
		await loadResults()
	}

	/** A row of the table is a case as one run executed it, which is what its panel shows. Editing
	 *  it is editing the dataset, one drawer up. */
	function openCase(row: ExperimentRow) {
		selectedCaseId = row.case_id
	}

	let selectedRow = $derived(displayRows.find((row) => row.case_id === selectedCaseId))

	/** A capture is the one case that exists before the dataset has it: it opens in the dataset it
	 *  would join, for review, and saving is what puts it in. Once a dataset is selected — a
	 *  capture reaching an agent that has never had one waits for the first to be created. */
	let captureOpened = false
	$effect(() => {
		const draft = capture
		const path = selectedDataset
		untrack(() => {
			if (!draft || !path || captureOpened) return
			captureOpened = true
			datasetDrawer?.openDrawer('edit', { capture: draft })
		})
	})

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
		untrack(() => loadResults())
	})

	/**
	 * What a run is called in the picker: the run it is, what ran it, and how many cases it holds.
	 * What ran is named rather than marked, because a sigil beside a version is a legend nobody has.
	 */
	function experimentTitle(e: EvalExperiment): string {
		// A run that only scored says so, or it reads as the agent having answered again.
		const parent = e.scored_from ? experiments.find((p) => p.id === e.scored_from) : undefined
		const scored = e.scored_from
			? ` · scored from ${parent ? experimentName(parent).toLowerCase() : 'an earlier run'}`
			: ''
		return `${experimentName(e)} · ${subjectLabelOf(e)} · ${e.case_count}${scored}`
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
		not_run: { icon: Minus, class: 'text-tertiary', label: 'Not run in this run' }
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

	/**
	 * The run's label is `v23 draft`, the agent is still on v23 with edits waiting, and they are
	 * not the same edits.
	 *
	 * That one case, because it is the only one the label cannot express. A run of an older version
	 * is history and says so (`Run 14 · v23` beside an agent on v24), and a run whose edits were
	 * deployed is a run of that version — the results endpoint recognises it and restamps it. Only
	 * two runs both reading `v23 draft` can silently be two different things.
	 *
	 * A property of the run, not of its rows: the subject is resolved once when the run is opened
	 * and every cell is stamped from it, so it is always all of them or none. Saying it once, above
	 * the table, is therefore the whole of saying it.
	 */
	let staleRun = $derived(
		rows.some(
			(row) =>
				// It executed a draft, and one that is not what is deployed now.
				row.subject_draft_hash != undefined &&
				row.subject_draft_hash !== deployedHash &&
				// On the version the agent is still on: a version that moved is named by the label.
				row.subject_version === currentVersion &&
				// And there is a draft now, holding something else.
				currentDraftHash != undefined &&
				row.subject_draft_hash !== currentDraftHash
		)
	)
</script>

<div class="flex flex-col h-full min-h-0">
	<!-- What you are looking at on the first row, what changes it on the second: two kinds of
	     control that read badly interleaved, and the row of pickers no longer wraps.

	     Nothing on the list: which run, and which dataset it was of, is what the rows say. The
	     pickers belong to a run being read, so they arrive with one. -->
	<div class="flex flex-wrap items-end gap-2 py-2 border-b">
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
		{#if !viewingRun}
			<!-- Only on the list, and only this: a run is a record, so there is nothing on it to
			     start. Named for what it opens rather than for what that then does: it asks which
			     state of the agent and which dataset, and both cost a provider bill, so a button
			     that reads as spending one on the way past would be lying about the click. -->
			<Button
				size="xs"
				variant="accent"
				startIcon={{ icon: Plus }}
				loading={running}
				disabled={running || !agentPath}
				onclick={() => (runDialogOpen = true)}
			>
				New evaluation
			</Button>
		{/if}
	</div>

	<!-- About the run on screen, so it goes when the run does: on the list there is no one run for
	     it to be about. No button and no frame of its own — it is a line the table carries, not a
	     panel above it. -->
	{#if viewingRun && staleRun}
		<div class="py-2">
			<Alert
				type="warning"
				size="xs"
				title={`This run executed an earlier state of the draft on v${currentVersion}`}
				collapsible={false}
			/>
		</div>
	{/if}

	<div class="grow min-h-0">
		<Splitpanes class="h-full">
			<Pane size={selectedRow ? 60 : 100} minSize={35}>
				<div class="h-full overflow-auto">
					{#if datasets.length === 0}
						<!-- Nothing to run and nothing to have run: the first dataset is the only move. -->
						<div class="h-full flex flex-col items-center justify-center gap-3 p-6 text-center">
							<span class="text-sm text-emphasis">No dataset yet</span>
							<span class="text-xs text-secondary max-w-md">
								A dataset is the set of cases this agent is measured on. Runs are of a dataset, so
								it is the first thing to make.
							</span>
							<Button
								size="xs"
								variant="accent"
								startIcon={{ icon: Plus }}
								onclick={() => datasetDrawer?.openDrawer('new')}
							>
								New dataset
							</Button>
						</div>
					{:else if !viewingRun}
						<!-- What this agent has already been measured at, across every dataset it has been
						     measured on: a run is worth reading against the ones before it, and that is a
						     list before it is a table. -->
						<EvalRunsList
							{experiments}
							{datasets}
							onOpen={(e) => openRun(e.id)}
							onEditDataset={async (path) => {
								await useDataset(path)
								datasetDrawer?.openDrawer('edit')
							}}
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
													{#if scoringColumns.has(scorer.id ?? '')}
														<Loader2 size={12} class="animate-spin text-blue-500 shrink-0" />
													{/if}
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
									<Row selected={row.case_id === selectedCaseId} on:click={() => openCase(row)}>
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
						<div class="flex items-center gap-2 px-3 py-2 border-b">
							<span class="text-xs font-semibold text-emphasis truncate">
								{caseLabel(openRow)}
							</span>
							<div class="grow"></div>
							<Button
								size="xs2"
								variant="subtle"
								startIcon={{ icon: X }}
								iconOnly
								title="Close"
								onclick={() => (selectedCaseId = undefined)}
							/>
						</div>
						<div class="p-3 flex flex-col gap-4">
							<!-- The case as this run executed it, not as the dataset holds it now: it is what
							     produced the answer below, and editing it is a drawer away. -->
							<Label label="User message">
								<span class="text-xs text-secondary whitespace-pre-wrap break-words">
									{openRow.input?.user_message ?? ''}
								</span>
							</Label>
							{#if openRow.expected != undefined && openRow.expected !== ''}
								<Label label="Expected">
									<span class="text-xs text-secondary whitespace-pre-wrap break-words">
										{typeof openRow.expected === 'string'
											? openRow.expected
											: JSON.stringify(openRow.expected, null, 2)}
									</span>
								</Label>
							{/if}
							{#if experiment && (openRow.job_id || openRow.output != undefined)}
								<!-- The answer as the run recorded it. The job behind the row is the whole
								     iteration — the agent and then the scorers that measured it — so the answer
								     is read from the row, and the iteration is what the link opens. -->
								<div class="rounded-md border border-light overflow-hidden">
									<div
										class="flex items-center gap-2 px-2 py-1 border-b border-light bg-surface-secondary"
									>
										<span class="text-2xs font-semibold text-secondary truncate">
											Case result
										</span>
										<div class="grow"></div>
										{#if openRow.job_id}
											<a
												class="text-2xs text-secondary hover:underline inline-flex items-center gap-1 shrink-0"
												href={`${base}/run/${openRow.job_id}?workspace=${ws}`}
												target="_blank"
											>
												Open the run
												<ExternalLink size={12} />
											</a>
										{/if}
									</div>
									<div class="p-2">
										{#if openRow.output != undefined}
											<!-- Rendered: an agent writes prose, and its own headings and lists are how it
											     meant the answer to be read. -->
											<div class="text-xs text-secondary break-words">
												<GfmMarkdown md={openRow.output} noPadding />
											</div>
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
	hasUndeployedChanges={undeployedChanges}
	{running}
	onRun={runAll}
	onEditDataset={async (path) => {
		await useDataset(path)
		datasetDrawer?.openDrawer('edit')
	}}
	onNewDataset={() => datasetDrawer?.openDrawer('new')}
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
/>
