<script lang="ts">
	import { Alert, Button, Skeleton } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Label from '$lib/components/Label.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Path from '$lib/components/Path.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ScriptEditorDrawer from '$lib/components/flows/content/ScriptEditorDrawer.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import {
		AiEvalsService,
		JobService,
		ScriptService,
		type EvalCase,
		type EvalDataset,
		type EvalExperiment,
		type ExperimentRow,
		type Job,
		type Scorer,
		type ScorerMean
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { onDestroy, untrack } from 'svelte'
	import {
		Plus,
		Play,
		Trash2,
		X,
		Check,
		Ban,
		FastForward,
		Loader2,
		Minus,
		Bot,
		Code2,
		ChevronDown,
		Pencil
	} from 'lucide-svelte'
	import EvalCaseEditor from './EvalCaseEditor.svelte'
	import EvalRunResult from './EvalRunResult.svelte'
	import AddScorer from './AddScorer.svelte'
	import { caseLabel, emptyCase, fromStoredCase, type CaseDraft } from './evalCaseUtils'
	import {
		formatDelta,
		formatScore,
		kindLabel,
		passedBy,
		scorerHref,
		scorerLabel,
		type ScorerKind
	} from './evalScorers'
	import { summaryToName, type Item } from '$lib/utils'
	import type { ScoreCaseResultResponse } from '$lib/gen'

	let {
		agentPath,
		opWorkspace = undefined,
		capture = undefined
	}: {
		/** The agent under test. A dataset and its runs belong to an agent, so an agent that has
		 * never been saved has nothing to hang them on. */
		agentPath: string
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
		/** A case captured from an AI agent run, opened for review before saving. */
		capture?: CaseDraft
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
	// Switching what is under test changes which experiments exist: reload rather than leave the
	// other subject's numbers on screen.
	$effect(() => {
		runDraft
		const path = untrack(() => selectedDataset)
		untrack(() => loadDataset(path))
	})

	let datasets = $state<EvalDataset[]>([])
	let dataset = $state<EvalDataset | undefined>(undefined)
	let selectedDataset = $state<string | undefined>(undefined)
	let experiments = $state<EvalExperiment[]>([])
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
	let loading = $state(false)
	let running = $state(false)
	let scoringAgain = $state(false)
	let scorers = $derived(dataset?.scorers ?? [])
	// Which columns have scoring in flight. Rescoring a deterministic scorer lands on the same
	// numbers, so without this the only sign it ran is a toast that has already gone.
	let scoringColumns = $derived(
		new Set(rows.flatMap((row) => row.scores.filter((s) => s.pending).map((s) => s.scorer_id)))
	)

	let selectedCaseId = $state<string | undefined>(undefined)
	let caseDraft = $state<CaseDraft | undefined>(undefined)
	// Bumped whenever `caseDraft` is replaced wholesale: the editor seeds local state from the
	// draft, so it is keyed on this and remounts rather than carrying one case's edits into the
	// next.
	let draftGeneration = $state(0)
	let job = $state<(Job & { result?: any }) | undefined>(undefined)

	let scorerDrawer: Drawer | undefined = $state()
	let scriptEditorDrawer: ScriptEditorDrawer | undefined = $state()
	// The kind is chosen before the drawer opens: the two are different enough that one form
	// asking which you meant is a form with half of it greyed out.
	let scorerKind = $state<ScorerKind>('agent')
	let removingScorer = $state<Scorer | undefined>(undefined)
	let removingCase = $state<ExperimentRow | undefined>(undefined)
	let thresholdScorer = $state<Scorer | undefined>(undefined)
	let thresholdValue = $state('')

	let newDatasetPath = $state('')
	let newDatasetSummary = $state('')
	let newDatasetPathError = $state('')
	// Set once the path is typed in, after which the summary stops driving it.
	let newDatasetPathDirty = $state(false)
	let newDatasetPathInput: Path | undefined = $state(undefined)
	// A dataset is named after the agent it tests, so it sorts next to it and a second one is the
	// next number rather than a naming decision made before there is anything in it. One segment
	// rather than a folder under the agent: a path is `<kind>/<owner>/<name>`, and the picker that
	// edits it cannot express a deeper one.
	let datasetPathBase = $derived(agentPath)
	let creatingDataset = $state(false)
	// Naming a dataset is the same form whether it has a name yet or not, so it is the same drawer:
	// inline, it either pushed the table down or replaced it, and neither reads as a small edit.
	let datasetDrawer: Drawer | undefined = $state()
	let datasetMode = $state<'new' | 'edit'>('new')
	let savingDataset = $state(false)

	/** The next free `<agent>_datasetN`, which is what a dataset is called until it is named. */
	function nextDatasetPath(): string {
		const taken = new Set(datasets.map((d) => d.path))
		let index = 1
		while (taken.has(`${datasetPathBase}_dataset${index}`)) index++
		return `${datasetPathBase}_dataset${index}`
	}

	function openDatasetDrawer(mode: 'new' | 'edit') {
		datasetMode = mode
		newDatasetPathError = ''
		if (mode === 'edit') {
			newDatasetPath = selectedDataset ?? ''
			newDatasetSummary = dataset?.summary ?? ''
			// A dataset that has a path keeps it: the summary names one that does not have one yet.
			newDatasetPathDirty = true
		} else {
			// Seeded rather than left empty: an empty path makes the picker invent a random name, and
			// a dataset named after the agent it tests sorts with the agent's own.
			newDatasetPath = nextDatasetPath()
			newDatasetSummary = ''
			newDatasetPathDirty = false
		}
		datasetDrawer?.openDrawer()
	}

	let experiment = $derived(experiments.find((e) => e.id === experimentId))

	// One agent, one history: runs of what is deployed and runs of the edits waiting on top of it
	// are both this agent's, and each says which it was. Keeping them in separate lists would only
	// hide the comparison that is the point of running the draft at all.
	async function listSubjectExperiments(): Promise<EvalExperiment[]> {
		if (!ws || !selectedDataset) return []
		return await AiEvalsService.listExperiments({
			workspace: ws,
			path: selectedDataset,
			subjectPath: agentPath
		})
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
		// leave the pane pointing at nothing.
		if (remembered && datasets.some((d) => d.path === remembered)) {
			selectedDataset = remembered
		}
	}

	// Switching datasets leaves the previous request in flight; only the newest may write, or a
	// slow response for the dataset you just left replaces the one you are looking at.
	let loadGeneration = 0

	async function loadDataset(path: string | undefined) {
		const generation = ++loadGeneration
		if (!ws || !path) {
			dataset = undefined
			experiments = []
			experimentId = undefined
			rows = []
			means = []
			return
		}
		loading = true
		try {
			const [row, list, cases] = await Promise.all([
				AiEvalsService.getEvalDataset({ workspace: ws, path }),
				listSubjectExperiments(),
				AiEvalsService.listEvalCases({ workspace: ws, path, perPage: CASE_PAGE_SIZE })
			])
			if (generation !== loadGeneration) return
			dataset = row
			experiments = list
			storedCases = Object.fromEntries(cases.cases.map((c) => [c.id, c]))
			// The newest run of this agent is what opens, whichever of its configurations ran it.
			experimentId = list[0]?.id
			baselineId = list[1]?.id
			await loadResults()
		} catch (e) {
			if (generation === loadGeneration) {
				sendUserToast(`Failed to load ${path}: ${e}`, true)
			}
		} finally {
			if (generation === loadGeneration) loading = false
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
	 * A trial run: one case, run now, to see what the agent does with it. It is a job and nothing
	 * more — looking at what it did is not a claim that it belongs in this dataset's history — so it
	 * never touches the table. It is scored where it stands and shown in the case panel, beside the
	 * recorded result it leaves alone.
	 */
	type TrialRun = {
		case_id: string
		job_id: string
		status: string
		/** The scoring job, once the run has finished and the scorers have been sent after it. */
		score_job_id?: string
		scores?: ScoreCaseResultResponse
	}
	let trial = $state<TrialRun | undefined>(undefined)

	async function refreshTrial() {
		if (!ws || !trial) return
		const current = $state.snapshot(trial)
		if (current.status !== 'running') return
		try {
			const loaded = (await JobService.getJob({
				workspace: ws,
				id: current.job_id,
				noLogs: true
			})) as Job & { result?: any; success?: boolean }
			if (trial?.job_id !== current.job_id) return
			if (loaded.type === 'CompletedJob') {
				trial = { ...current, status: loaded.success ? 'success' : 'failure' }
			}
			// While the trial's case is the one open, the panel shows the trial's own job: its answer
			// and its trajectory are what you asked to see.
			if (selectedCaseId === current.case_id) job = loaded as Job & { result?: any }
		} catch {
			// A job that has not been created yet: the panel says running until it says otherwise.
		}
	}

	/** A trial is scored where it stands: a run whose numbers you cannot see is a run you have to
	 *  eyeball, which is the thing scorers exist to replace. */
	async function scoreTrial() {
		if (!ws || !selectedDataset || scorers.length === 0 || !trial) return
		const current = $state.snapshot(trial)
		if (current.status !== 'success') return
		try {
			if (!current.score_job_id) {
				const scoreJob = await AiEvalsService.scoreCaseRun({
					workspace: ws,
					requestBody: {
						dataset: selectedDataset,
						case_id: current.case_id,
						job_id: current.job_id
					}
				})
				if (trial?.job_id === current.job_id) trial = { ...current, score_job_id: scoreJob }
				return
			}
			if (current.scores?.every((s) => !s.pending)) return
			const scores = await AiEvalsService.scoreCaseResult({
				workspace: ws,
				dataset: selectedDataset,
				jobId: current.score_job_id
			})
			if (trial?.job_id === current.job_id) trial = { ...current, scores }
		} catch (e) {
			sendUserToast(`Failed to score the trial run: ${e}`, true)
			// Dropped rather than retried every two seconds: the run itself is still there.
			if (trial?.job_id === current.job_id) trial = { ...current, scores: [] }
		}
	}

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

	// Set when runs are started, cleared once they have been handed to scoring. Scoring is a
	// deliberate act, so it follows a run rather than being retried for as long as a cell is empty:
	// a run with no answer to score would otherwise be scored again every two seconds forever.
	let pendingScore = $state(false)
	/** A case still running, a scoring job in flight, or answers waiting to be scored. */
	let pollNeeded = $derived(
		pendingScore ||
			trial?.status === 'running' ||
			(trial?.status === 'success' &&
				scorers.length > 0 &&
				(!trial.scores || trial.scores.some((s) => s.pending))) ||
			rows.some((row) => row.status === 'running' || row.scores.some((score) => score.pending))
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

	/** Runs finish, then their answers are scored. Scoring is a separate call because it is a
	 *  separate act: it reads stored answers and never calls the agent. */
	async function refresh() {
		// The trial first, and before the guard below: a dataset that has never been run has no
		// experiment to read, and a trial there is exactly what someone is looking at.
		await refreshTrial()
		await scoreTrial()
		if (!ws || !selectedDataset || !experimentId) return
		const stillRunning = rows.some((row) => row.status === 'running')
		if (pendingScore && !stillRunning && scorers.length > 0) {
			pendingScore = false
			try {
				await AiEvalsService.scoreExperiment({
					workspace: ws,
					requestBody: { dataset: selectedDataset, experiment_id: experimentId }
				})
			} catch (e) {
				sendUserToast(`Scoring failed: ${e}`, true)
			}
		} else if (pendingScore && !stillRunning) {
			pendingScore = false
		}
		await loadResults()
	}

	async function runAll() {
		if (!ws || !selectedDataset) return
		running = true
		try {
			const id = await AiEvalsService.runExperiment({
				workspace: ws,
				requestBody: { dataset: selectedDataset, subject }
			})
			experiments = await listSubjectExperiments()
			// The experiment just closed is the obvious thing to compare against.
			baselineId = experiments.find((e) => e.id !== id)?.id
			experimentId = id
			pendingScore = true
			await loadResults()
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to run the dataset: ${e}`, true)
		} finally {
			running = false
		}
	}

	/** Runs one case now, as a trial. Nothing is recorded: the table goes on showing what the
	 *  selected run recorded, and Run all is what produces numbers to compare. */
	/**
	 * Measure the selected run's answers again, with the scorers as they are now.
	 *
	 * A run is permanent, so this makes a run of its own rather than replacing the numbers on the one
	 * being looked at. It calls the agent for nothing: the answers are the expensive artifact and they
	 * are already stored, which is the whole reason scoring is a separate act from running.
	 */
	async function scoreAgain() {
		if (!ws || !selectedDataset || !experimentId) return
		scoringAgain = true
		try {
			const id = await AiEvalsService.scoreAgain({
				workspace: ws,
				requestBody: { dataset: selectedDataset, experiment_id: experimentId }
			})
			experiments = await listSubjectExperiments()
			baselineId = experimentId
			experimentId = id
			// The cells are copied already finished, so the scorers are sent after them on the next tick.
			pendingScore = true
			await loadResults()
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to score the run again: ${e}`, true)
		} finally {
			scoringAgain = false
		}
	}

	async function runCase(caseId: string) {
		if (!ws || !selectedDataset) return
		running = true
		try {
			const res = await AiEvalsService.runEval({
				workspace: ws,
				requestBody: { subject, dataset: selectedDataset, case_id: caseId }
			})
			trial = { case_id: caseId, job_id: res.job_id, status: 'running' }
			job = undefined
		} catch (e) {
			sendUserToast(`Failed to run the case: ${e}`, true)
		} finally {
			running = false
		}
	}

	async function saveScorers(next: Scorer[]) {
		if (!ws || !selectedDataset || !dataset) return
		await AiEvalsService.updateEvalDataset({
			workspace: ws,
			path: selectedDataset,
			requestBody: {
				summary: dataset.summary,
				description: dataset.description,
				default_subject: dataset.default_subject,
				scorers: next
			}
		})
		dataset = await AiEvalsService.getEvalDataset({ workspace: ws, path: selectedDataset })
		await loadResults()
	}

	let resourceEditorDrawer: ResourceEditorDrawer | undefined = $state()

	async function editScorer(scorer: Scorer) {
		if (!ws) return
		if (scorer.kind === 'agent') {
			await resourceEditorDrawer?.initEdit(scorer.path)
			return
		}
		try {
			// The editor drawer opens a script by hash, so the latest one is resolved here; saving
			// writes a new version, which is a new definition of that column.
			const script = await ScriptService.getScriptByPath({ workspace: ws, path: scorer.path })
			scriptEditorDrawer?.openDrawer(script.hash, () => loadResults())
		} catch (e) {
			sendUserToast(`Failed to open ${scorer.path}: ${e}`, true)
		}
	}

	let scorerFormGeneration = $state(0)
	function openScorerDrawer(kind: ScorerKind) {
		scorerKind = kind
		scorerFormGeneration += 1
		scorerDrawer?.openDrawer()
	}

	async function addScorer(scorer: Scorer) {
		try {
			await saveScorers([...scorers, scorer])
			scorerDrawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Failed to add the scorer: ${e}`, true)
		}
	}

	/** Where the pass line sits is an interpretation of the scores rather than part of producing
	 *  them, so moving it re-reads the runs already recorded instead of asking for them again. */
	async function saveThreshold(passIf: number | undefined) {
		const target = thresholdScorer
		if (!target) return
		try {
			await saveScorers(scorers.map((s) => (s.id === target.id ? { ...s, pass_if: passIf } : s)))
			thresholdScorer = undefined
		} catch (e) {
			sendUserToast(`Failed to save the threshold: ${e}`, true)
		}
	}

	/** Renaming moves the dataset: its cases and experiments follow it through the foreign keys. */
	async function saveDataset() {
		if (!ws || !selectedDataset || !dataset || !newDatasetPath || newDatasetPathError) return
		savingDataset = true
		try {
			await AiEvalsService.updateEvalDataset({
				workspace: ws,
				path: selectedDataset,
				requestBody: {
					path: newDatasetPath,
					summary: newDatasetSummary || undefined,
					description: dataset.description,
					default_subject: dataset.default_subject,
					scorers: dataset.scorers
				}
			})
			await loadDatasets()
			datasetDrawer?.closeDrawer()
			selectedDataset = newDatasetPath
		} catch (e) {
			sendUserToast(`Failed to save the dataset: ${e}`, true)
		} finally {
			savingDataset = false
		}
	}

	/** Creates the dataset the first case needs. Naming it is a decision worth making after you
	 *  know what is in it, so it is made for you here and renamed from the toolbar. */
	async function createDataset(): Promise<string | undefined> {
		if (!ws || !newDatasetPath || newDatasetPathError) return undefined
		creatingDataset = true
		try {
			const path = newDatasetPath
			await AiEvalsService.createEvalDataset({
				workspace: ws,
				requestBody: {
					path,
					summary: newDatasetSummary || undefined,
					default_subject: { kind: 'agent', path: agentPath }
				}
			})
			await loadDatasets()
			datasetDrawer?.closeDrawer()
			selectedDataset = path
			newDatasetPath = ''
			newDatasetSummary = ''
			newDatasetPathDirty = false
			return path
		} catch (e) {
			sendUserToast(`Failed to create the dataset: ${e}`, true)
			return undefined
		} finally {
			creatingDataset = false
		}
	}

	/** The summary names the dataset, as it does a script: what it is for is the thing you know
	 *  first, and a path derived from it beats one you have to invent. Until the path is typed in,
	 *  after which it is the reader's. */
	$effect(() => {
		const summary = newDatasetSummary
		untrack(() => {
			if (newDatasetPathDirty || !summary) return
			// Named after the agent as well as after itself, so it sorts with the agent's own and reads
			// as belonging to it. The whole path stays editable.
			const agentName = agentPath.split('/').pop() ?? agentPath
			newDatasetPathInput?.setName(`${agentName}_${summaryToName(summary)}`)
		})
	})

	function openCase(row: ExperimentRow) {
		selectedCaseId = row.case_id
		const stored = storedCases[row.case_id]
		caseDraft = stored
			? (fromStoredCase(stored) as CaseDraft)
			: ({ id: row.case_id, name: row.name, input: row.input, expected: row.expected } as CaseDraft)
		draftGeneration += 1
		loadJob(row.job_id)
	}

	/** A capture is the one case that exists before the dataset has it: it is opened for review,
	 *  and saving is what puts it in. */
	function openCapture() {
		if (!capture) return
		selectedCaseId = undefined
		caseDraft = structuredClone($state.snapshot(capture)) as CaseDraft
		draftGeneration += 1
		job = undefined
	}

	/** Adding a case adds the row. A case is a row of the dataset, so asking for one writes it and
	 *  the panel edits it in place; running it is a separate decision. */
	async function addCase() {
		const path = selectedDataset
		if (!ws || !path) return
		try {
			const id = await AiEvalsService.addEvalCase({
				workspace: ws,
				path,
				requestBody: emptyCase()
			})
			await reloadCases()
			selectedCaseId = id
			caseDraft = { ...emptyCase(), id }
			draftGeneration += 1
			job = undefined
		} catch (e) {
			sendUserToast(`Failed to add a case: ${e}`, true)
		}
	}

	let jobGeneration = 0
	async function loadJob(id: string | undefined) {
		const generation = ++jobGeneration
		job = undefined
		if (!ws || !id) return
		try {
			const loaded = await JobService.getJob({ workspace: ws, id, noLogs: true })
			if (generation === jobGeneration) job = loaded as Job & { result?: any }
		} catch {
			// A job that has not been created yet, or was retained away: the row already says so.
		}
	}

	async function saveCase() {
		if (!ws || !selectedDataset || !caseDraft) return
		try {
			if (caseDraft.id) {
				await AiEvalsService.updateEvalCase({
					workspace: ws,
					path: selectedDataset,
					requestBody: { ...caseDraft, id: caseDraft.id }
				})
			} else {
				// The id the write returns is adopted, so saving twice edits the case rather than
				// adding a second one.
				const id = await AiEvalsService.addEvalCase({
					workspace: ws,
					path: selectedDataset,
					requestBody: caseDraft
				})
				caseDraft = { ...caseDraft, id }
				selectedCaseId = id
			}
			await reloadCases()
			await loadResults()
		} catch (e) {
			sendUserToast(`Failed to save the case: ${e}`, true)
		}
	}

	async function deleteCase(caseId: string) {
		if (!ws || !selectedDataset) return
		try {
			await AiEvalsService.deleteEvalCase({
				workspace: ws,
				path: selectedDataset,
				requestBody: { id: caseId }
			})
			if (selectedCaseId === caseId) {
				selectedCaseId = undefined
				caseDraft = undefined
			}
			await reloadCases()
			await loadResults()
		} catch (e) {
			sendUserToast(`Failed to delete the case: ${e}`, true)
		}
	}

	$effect(() => {
		if (ws) untrack(() => loadDatasets())
	})
	$effect(() => {
		const path = selectedDataset
		untrack(() => {
			rememberDataset(path)
			loadDataset(path)
		})
	})
	$effect(() => {
		// Picking a run is asking to see it, and picking a baseline is what turns every column into
		// a comparison. Both are a different table.
		experimentId
		baselineId
		untrack(() => loadResults())
	})
	$effect(() => {
		if (capture) untrack(() => openCapture())
	})

	/** An experiment is called by the run it is, which is short enough to say in a menu item. */
	function experimentName(e: EvalExperiment): string {
		return `Run ${e.run_number}`
	}

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

	/** A deployed version, or a version with edits sitting on top of it. A draft run whose
	 *  configuration was later deployed is a run of that version, whatever it was when it ran. */
	function subjectLabelOf(e: EvalExperiment): string {
		const deployed =
			e.subject.kind === 'agent' ||
			(e.subject.draft_hash != undefined && e.subject.draft_hash === deployedHash)
		if (deployed) {
			const version = e.subject.kind === 'agent' ? e.subject.version : currentVersion
			return version ? `v${version}` : 'deployed'
		}
		return e.subject.version ? `v${e.subject.version} + edits` : 'unsaved edits'
	}

	let experimentItems = $derived(
		experiments.map((e) => ({ label: experimentTitle(e), value: e.id }))
	)
	// This agent's own datasets first. A dataset is named under what it tests, so they cluster
	// anyway; the rest stay in the list because running one dataset against a second agent is the
	// comparison the picker exists for, and filtering them out is how nobody finds it.
	let datasetItems = $derived(
		[...datasets]
			.sort((a, b) => {
				// Named after the agent, or under it: datasets predating the one-segment naming sit in a
				// folder of the agent's name, and they are as much this agent's as the rest.
				const own = (d: EvalDataset) =>
					d.path.startsWith(`${agentPath}_`) || d.path.startsWith(`${agentPath}/`) ? 0 : 1
				return own(a) - own(b) || a.path.localeCompare(b.path)
			})
			.map((d) => ({ label: d.path, value: d.path }))
	)

	/**
	 * The column header is the scorer's control: everything a column can do lives here.
	 *
	 * Scoring is not among them. A run is scored when it is run, and a column added later has no
	 * number on the runs that predate it — an action here that scored other runs read as acting on
	 * the one on screen, and one that rescored this one made a permanent run editable after all.
	 * Trying an edited scorer is a case away, in the panel.
	 */
	function scorerMenu(scorer: Scorer): Item[] {
		return [
			{
				// Editing a column is editing the runnable it points at, so it opens here rather
				// than sending you to another tab to find it.
				displayName: scorer.kind === 'agent' ? 'Edit agent' : 'Edit script',
				action: () => editScorer(scorer)
			},
			{
				displayName: 'Open in a new tab',
				href: scorerHref(scorer, ws),
				hrefTarget: '_blank'
			},
			{
				displayName:
					scorer.pass_if == undefined
						? 'Set a pass threshold'
						: `Pass threshold · at least ${scorer.pass_if}`,
				action: () => {
					thresholdScorer = scorer
					thresholdValue = scorer.pass_if == undefined ? '' : String(scorer.pass_if)
				}
			},
			{
				displayName: 'Remove scorer',
				type: 'delete',
				action: () => (removingScorer = scorer)
			}
		]
	}

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
	 * The run's label is `v23 + edits`, the agent is still on v23 with edits waiting, and they are
	 * not the same edits.
	 *
	 * That one case, because it is the only one the label cannot express. A run of an older version
	 * is history and says so (`Run 14 · v23` beside an agent on v24), and a run whose edits were
	 * deployed is a run of that version — the results endpoint recognises it and restamps it. Only
	 * two runs both reading `v23 + edits` can silently be two different things.
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
	     control that read badly interleaved, and the row of pickers no longer wraps. -->
	<div class="flex flex-wrap items-end gap-2 px-3 pt-2 pb-1">
		<Label label="Dataset" class="w-52 shrink">
			<Select
				items={datasetItems}
				bind:value={selectedDataset}
				placeholder="Select a dataset"
				class="text-xs"
			>
				<!-- What you can do to the dataset lives in its own picker: renaming it and starting
				     another are both about which dataset you are in. -->
				{#snippet bottomSnippet({ close })}
					<div class="flex flex-col border-t">
						{#if selectedDataset}
							<button
								type="button"
								class="flex items-center gap-2 px-3 py-2 text-xs text-secondary hover:bg-surface-hover"
								onclick={() => {
									openDatasetDrawer('edit')
									close()
								}}
							>
								<Pencil size={13} />
								Edit this dataset
							</button>
						{/if}
						<button
							type="button"
							class="flex items-center gap-2 px-3 py-2 text-xs text-secondary hover:bg-surface-hover"
							onclick={() => {
								openDatasetDrawer('new')
								close()
							}}
						>
							<Plus size={13} />
							New dataset
						</button>
					</div>
				{/snippet}
			</Select>
		</Label>
		{#if experiments.length > 0}
			<Label label="Run" class="w-48 shrink">
				<Select items={experimentItems} bind:value={experimentId} class="text-xs" />
			</Label>
		{/if}
		<div class="grow"></div>
		{#if selectedDataset}
			<!-- Always shown once there is a dataset: a comparison you cannot see the control for is
			     a comparison nobody knows they can make. -->
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
	</div>
	<div class="flex items-center gap-2 px-3 pb-2 border-b">
		<div class="grow"></div>
		<!-- Only with a dataset to add it to: a menu trigger cannot be un-disabled once melt has
		     mounted it disabled, and until a dataset is picked the pane is the create-dataset
		     screen. -->
		{#if selectedDataset}
			<DropdownV2
				items={[
					{ displayName: 'Judge agent', icon: Bot, action: () => openScorerDrawer('agent') },
					{ displayName: 'Script', icon: Code2, action: () => openScorerDrawer('script') }
				]}
				placement="bottom-end"
			>
				{#snippet buttonReplacement()}
					<Button
						nonCaptureEvent
						size="xs"
						variant="default"
						startIcon={{ icon: Plus }}
						endIcon={{ icon: ChevronDown }}
					>
						Add scorer
					</Button>
				{/snippet}
			</DropdownV2>
		{/if}
		<!-- Running every case is the button; measuring what a run already answered is beside it,
		     because they produce the same thing at very different prices. -->
		<Button
			size="xs"
			variant="accent"
			startIcon={{ icon: Play }}
			loading={running || scoringAgain}
			disabled={!selectedDataset || running || displayRows.length === 0 || !subject.path}
			onclick={runAll}
			dropdownItems={[
				{
					label: 'Run scorers only',
					tooltip:
						'Measures the answers this run already stored, with the scorers as they are now. Opens a run of its own and calls the agent for nothing.',
					disabled: !experimentId || scorers.length === 0 || rows.length === 0,
					onClick: scoreAgain
				}
			]}
		>
			Run all
		</Button>
	</div>

	{#if staleRun}
		<!-- No button: Run all is one row up, and a second way to start the same run is a second thing
		     to explain. -->
		<div class="px-3 py-2 border-b">
			<Alert
				type="warning"
				size="xs"
				title={`This run executed an earlier state of the draft on v${currentVersion}`}
				collapsible={false}
			/>
		</div>
	{/if}

	{#if thresholdScorer}
		<div class="flex items-end gap-2 px-3 py-2 border-b bg-surface-secondary">
			<Label label={`Pass threshold for ${scorerLabel(thresholdScorer)}`} class="w-64">
				<TextInput bind:value={thresholdValue} size="sm" inputProps={{ placeholder: '0.7' }} />
			</Label>
			<span class="text-2xs text-tertiary pb-2">
				A score at or above this counts as a pass. It reads the scores already recorded, so the
				column reports a pass rate for every run without any of them being run again.
			</span>
			<div class="grow"></div>
			<Button
				size="xs"
				variant="default"
				disabled={!thresholdValue || Number.isNaN(Number(thresholdValue))}
				onclick={() => saveThreshold(Number(thresholdValue))}
			>
				Save
			</Button>
			{#if thresholdScorer.pass_if != undefined}
				<Button size="xs" variant="subtle" onclick={() => saveThreshold(undefined)}>
					Score only
				</Button>
			{/if}
			<Button size="xs" variant="subtle" onclick={() => (thresholdScorer = undefined)}
				>Cancel</Button
			>
		</div>
	{/if}

	<div class="grow min-h-0">
		<Splitpanes class="h-full">
			<Pane size={caseDraft ? 60 : 100} minSize={35}>
				<div class="h-full overflow-auto">
					{#if loading}
						<div class="p-3 flex flex-col gap-1">
							<Skeleton layout={[[2], 0.5, [2], 0.5, [2]]} />
						</div>
					{:else if !selectedDataset}
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
								onclick={() => openDatasetDrawer('new')}
							>
								New dataset
							</Button>
						</div>
					{:else}
						<DataTable size="sm" tableFixed>
							<!-- A score column is as wide as a score and its column name need; the question and the
							     answer share what is left. Sized rather than divided equally, because the text will
							     take any width it is given and leave the numbers squeezed against each other. -->
							<colgroup>
								<col style="width: 22%" />
								<col style="width: 28%" />
								{#each scorers as scorer (scorer.id)}
									<col style="width: 9rem" />
								{/each}
								<col style="width: 4.5rem" />
							</colgroup>
							<Head>
								<tr>
									<Cell head first>Case</Cell>
									<Cell head>Answer</Cell>
									{#each scorers as scorer (scorer.id)}
										{@const mean = means.find((m) => m.scorer_id === scorer.id)}
										{@const headline = columnHeadline(scorer, mean)}
										<Cell head numeric>
											<!-- Two rows, always: the name, and the number under the name it is a number of.
											     The second row keeps its height while there is nothing in it, so the table
											     does not move when the first score lands. -->
											<div class="flex flex-col items-end min-w-0">
												<DropdownV2 items={() => scorerMenu(scorer)} placement="bottom-end">
													{#snippet buttonReplacement()}
														<span
															class="flex items-center gap-1 min-w-0 cursor-pointer"
															title={columnTitle(scorer, mean)}
														>
															{#if scorer.kind === 'agent'}
																<Bot size={13} class="text-tertiary shrink-0" />
															{:else}
																<Code2 size={13} class="text-tertiary shrink-0" />
															{/if}
															<span class="truncate">{scorerLabel(scorer)}</span>
															{#if scoringColumns.has(scorer.id ?? '')}
																<Loader2 size={12} class="animate-spin text-blue-500 shrink-0" />
															{:else}
																<ChevronDown size={12} class="text-tertiary shrink-0" />
															{/if}
														</span>
													{/snippet}
												</DropdownV2>
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
									<Cell head last></Cell>
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each displayRows as row (row.case_id)}
									{@const status = statusOf(row.status)}
									<Row selected={row.case_id === selectedCaseId} on:click={() => openCase(row)}>
										<Cell first>
											<span class="truncate block text-emphasis">{caseLabel(row)}</span>
										</Cell>
										<Cell>
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
										{#each scorers as scorer (scorer.id)}
											{@const cell = row.scores.find((s) => s.scorer_id === scorer.id)}
											<Cell numeric>
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
										<Cell last>
											<!-- Running a case is a decision made with the case open, so it lives in the
											     panel; the row keeps only the one action that is about the row itself. -->
											<div class="flex items-center justify-end">
												<Button
													size="xs2"
													variant="subtle"
													startIcon={{ icon: Trash2 }}
													iconOnly
													title="Delete this case"
													on:click={(e) => {
														e.stopPropagation()
														removingCase = row
													}}
												/>
											</div>
										</Cell>
									</Row>
								{/each}
								<tr>
									<td colspan={scorers.length + 3} class="p-2">
										<Button
											size="xs2"
											variant="subtle"
											startIcon={{ icon: Plus }}
											onclick={addCase}
										>
											Add a case
										</Button>
									</td>
								</tr>
							</tbody>
						</DataTable>
					{/if}
				</div>
			</Pane>
			{#if caseDraft}
				<Pane size={40} minSize={25}>
					<div class="h-full overflow-auto flex flex-col">
						<div class="flex items-center gap-2 px-3 py-2 border-b">
							<span class="text-xs font-semibold text-emphasis truncate">
								{caseLabel(caseDraft)}
							</span>
							<div class="grow"></div>
							<Button
								size="xs2"
								variant="subtle"
								startIcon={{ icon: X }}
								iconOnly
								title="Close"
								onclick={() => {
									caseDraft = undefined
									selectedCaseId = undefined
								}}
							/>
						</div>
						<div class="p-3 flex flex-col gap-4">
							{#key draftGeneration}
								<EvalCaseEditor
									bind:draft={caseDraft}
									{running}
									canSave={!!selectedDataset}
									onSave={saveCase}
									onRun={() => caseDraft?.id && runCase(caseDraft.id)}
								/>
							{/key}
							{#if trial && trial.case_id === selectedCaseId}
								{@const trialStatus = statusOf(trial.status)}
								<!-- Dashed, named and beside the table rather than in it: what a trial produced is
								     worth seeing, and is not a result the selected run ever had. -->
								<div class="rounded-md border border-dashed p-3 flex flex-col gap-2">
									<div class="flex items-center gap-2">
										<trialStatus.icon size={14} class={trialStatus.class} />
										<span class="text-xs font-semibold text-emphasis">Trial run</span>
										<div class="grow"></div>
										<Button size="xs2" variant="subtle" onclick={() => (trial = undefined)}>
											Clear
										</Button>
									</div>
									<span class="text-2xs text-tertiary">
										Not part of {experiment ? experimentName(experiment) : 'any run'}, which still
										shows what it recorded. Run all is what makes a run these numbers would belong
										to.
									</span>
									{#if scorers.length > 0}
										<div class="flex flex-wrap gap-x-4 gap-y-1">
											{#each scorers as scorer (scorer.id)}
												{@const cell = trial.scores?.find((s) => s.scorer_id === scorer.id)}
												{@const passed = passedBy(scorer, cell?.score)}
												<span
													class="text-2xs flex items-baseline gap-1.5"
													title={cell?.reason ?? ''}
												>
													<span class="text-tertiary">{scorerLabel(scorer)}</span>
													{#if trial.status === 'running' || !cell || cell.pending}
														<Loader2 size={12} class="animate-spin text-blue-500" />
													{:else if cell.score != undefined}
														{#if passed != undefined}
															<span class={passed ? 'text-green-500' : 'text-red-500'}>
																{passed ? '✓' : '✗'}
															</span>
														{/if}
														<span class="tabular-nums font-medium text-emphasis">
															{formatScore(cell.score)}
														</span>
													{:else if cell.error}
														<span class="text-red-500" title={cell.error}>failed</span>
													{:else}
														<span class="text-tertiary">—</span>
													{/if}
												</span>
											{/each}
										</div>
									{/if}
								</div>
							{/if}
							{#if job}
								<EvalRunResult
									{job}
									title={trial && trial.case_id === selectedCaseId
										? 'Trial run'
										: experiment
											? experimentName(experiment)
											: 'Result'}
								/>
							{/if}
						</div>
					</div>
				</Pane>
			{/if}
		</Splitpanes>
	</div>
</div>

<Drawer bind:this={datasetDrawer} size="600px">
	<DrawerContent
		title={datasetMode === 'edit' ? 'Edit dataset' : 'New dataset'}
		on:close={() => datasetDrawer?.closeDrawer()}
	>
		<!-- Keyed so the path field is seeded for the dataset it was opened for, rather than carrying
		     the one before it. -->
		{#key datasetMode + (selectedDataset ?? '')}
			<div class="flex flex-col gap-6">
				<Label label="Summary">
					<TextInput
						bind:value={newDatasetSummary}
						size="sm"
						inputProps={{ placeholder: 'What this set of cases is for' }}
					/>
				</Label>
				<Path
					bind:this={newDatasetPathInput}
					bind:path={newDatasetPath}
					bind:error={newDatasetPathError}
					bind:dirty={newDatasetPathDirty}
					initialPath={datasetMode === 'edit' ? (selectedDataset ?? '') : ''}
					checkInitialPathExistence={false}
					namePlaceholder="cases"
					kind="resource"
					workspaceOverride={ws}
					autofocus={false}
					size="sm"
				/>
				{#if datasetMode === 'edit'}
					<span class="text-2xs text-tertiary">
						Renaming moves the dataset: its cases and its runs follow it.
					</span>
				{/if}
			</div>
		{/key}
		{#snippet actions()}
			<Button
				size="xs"
				variant="accent"
				startIcon={{ icon: datasetMode === 'edit' ? Pencil : Plus }}
				loading={creatingDataset || savingDataset}
				disabled={creatingDataset ||
					savingDataset ||
					!newDatasetPath ||
					!!newDatasetPathError ||
					(datasetMode === 'edit' &&
						newDatasetPath === selectedDataset &&
						(newDatasetSummary || '') === (dataset?.summary ?? ''))}
				onclick={() => (datasetMode === 'edit' ? saveDataset() : createDataset())}
			>
				{datasetMode === 'edit' ? 'Save' : 'Create dataset'}
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<Drawer bind:this={scorerDrawer} size="700px">
	<DrawerContent
		title={`Add a ${kindLabel(scorerKind).toLowerCase()}`}
		on:close={() => scorerDrawer?.closeDrawer()}
	>
		{#if ws && selectedDataset}
			<!-- Keyed so the form is seeded for the kind it is opened for, rather than carrying the
			     path and prompt of the one added before it. -->
			{#key scorerFormGeneration}
				<AddScorer
					workspace={ws}
					datasetPath={selectedDataset}
					kind={scorerKind}
					onAdd={addScorer}
					onEditScript={(hash) => scriptEditorDrawer?.openDrawer(hash, () => loadResults())}
				/>
			{/key}
		{/if}
	</DrawerContent>
</Drawer>

<!-- Saving here writes a new version of the script, which is a new definition of that column:
     re-reading the results is what makes the table say so. -->
<ScriptEditorDrawer bind:this={scriptEditorDrawer} />

<!-- Deploying the agent moves the version every run is measured against, and restoring an older
     one moves it back: both are read again rather than left on screen as they were. -->
<ResourceEditorDrawer
	bind:this={resourceEditorDrawer}
	workspace={ws}
	onRestored={loadResults}
	on:refresh={() => {
		readSubjectState()
		loadResults()
	}}
/>

<ConfirmationModal
	open={removingCase != undefined}
	title="Delete this case"
	confirmationText="Delete"
	on:canceled={() => (removingCase = undefined)}
	on:confirmed={async () => {
		const target = removingCase
		removingCase = undefined
		if (target) await deleteCase(target.case_id)
	}}
>
	<span class="text-sm">
		{caseLabel(removingCase ?? { input: {} })} goes from the dataset. The runs that executed it keep
		their results: a run that happened is not undone by curating the case away.
	</span>
</ConfirmationModal>

<ConfirmationModal
	open={removingScorer != undefined}
	title="Remove this column"
	confirmationText="Remove"
	on:canceled={() => (removingScorer = undefined)}
	on:confirmed={async () => {
		const target = removingScorer
		removingScorer = undefined
		if (target) await saveScorers(scorers.filter((s) => s.id !== target.id))
	}}
>
	<span class="text-sm">
		The column goes; the scores it already produced stay on the runs that carry them.
	</span>
</ConfirmationModal>
