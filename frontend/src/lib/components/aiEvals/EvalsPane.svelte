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
	import Path from '$lib/components/Path.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ScriptEditorDrawer from '$lib/components/flows/content/ScriptEditorDrawer.svelte'
	import ResourceEditorDrawer from '$lib/components/ResourceEditorDrawer.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import {
		AiEvalsService,
		JobService,
		ScriptService,
		type CellScore,
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
		Pencil,
		RefreshCw
	} from 'lucide-svelte'
	import EvalCaseEditor from './EvalCaseEditor.svelte'
	import EvalRunResult from './EvalRunResult.svelte'
	import AddScorer from './AddScorer.svelte'
	import {
		caseLabel,
		caseRunPath,
		emptyCase,
		fromStoredCase,
		type CaseDraft
	} from './evalCaseUtils'
	import {
		formatDelta,
		formatScore,
		kindLabel,
		scorerHref,
		scorerLabel,
		type ScorerKind
	} from './evalScorers'
	import type { Item } from '$lib/utils'
	import type { AgentDraft, ScoreCaseResultResponse } from '$lib/gen'

	let {
		agentPath = undefined,
		draft = undefined,
		subjectLabel = undefined,
		flowPath = undefined,
		originAgentPath = undefined,
		opWorkspace = undefined,
		capture = undefined
	}: {
		/** The saved agent under test. */
		agentPath?: string
		/** The step as authored, when it has not been saved as an agent yet. */
		draft?: AgentDraft
		/** What a draft run is filed under, so it can be found again. */
		subjectLabel?: string
		/** The flow the step belongs to, which names the dataset when the step has no agent yet. */
		flowPath?: string
		/** The saved agent this step was forked from for editing, when it was. What runs is still
		 * the step, but it is an edit of that agent's current version rather than an anonymous
		 * draft, and the runs say so. */
		originAgentPath?: string
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
		/** A case captured from a run, a conversation or a step test, opened for review before
		 * saving. */
		capture?: CaseDraft
	} = $props()

	let ws = $derived(opWorkspace ?? $workspaceStore)
	// Running the agent's draft is a subject of its own: the server inlines the draft's value, so
	// the run executes what is being edited rather than what is deployed, and its history stays
	// separate from the deployed agent's.
	let runDraft = $state(false)
	let subject = $derived(
		draft
			? {
					kind: 'draft' as const,
					path: subjectLabel || agentPath || 'draft',
					draft,
					origin_path: originAgentPath
				}
			: {
					kind: (runDraft ? 'agent_draft' : 'agent') as 'agent' | 'agent_draft',
					path: agentPath ?? ''
				}
	)
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
	let regressed = $state(0)
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
	let scorers = $derived(dataset?.scorers ?? [])

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

	let newDatasetPath = $state('')
	// A dataset is named under what it tests, so it sits next to it and a second one is the next
	// number rather than a naming decision made before you know what the thing is. A step with no
	// agent of its own is named under its flow.
	let datasetPathBase = $derived(agentPath || flowPath || '')
	$effect(() => {
		const base = datasetPathBase
		const taken = new Set(datasets.map((d) => d.path))
		untrack(() => {
			if (newDatasetPath || !base) return
			let index = 1
			while (taken.has(`${base}/dataset${index}`)) index++
			newDatasetPath = `${base}/dataset${index}`
		})
	})
	let creatingDataset = $state(false)
	let renaming = $state(false)
	let renamePath = $state('')
	let renameError = $state('')

	let experiment = $derived(experiments.find((e) => e.id === experimentId))
	let baseline = $derived(experiments.find((e) => e.id === baselineId))

	// One agent, one history: the runs of what is deployed and the runs of configurations of it
	// that were not — including the ones a step brought with it when it was saved as this agent.
	// Both say which they were; keeping them in separate tables would only hide the comparison.
	let alsoKind: 'agent' | 'agent_draft' | undefined = $derived(
		subject.kind === 'agent' ? 'agent_draft' : subject.kind === 'agent_draft' ? 'agent' : undefined
	)

	async function listSubjectExperiments(): Promise<EvalExperiment[]> {
		if (!ws || !selectedDataset) return []
		return await AiEvalsService.listExperiments({
			workspace: ws,
			path: selectedDataset,
			subjectPath: subject.path,
			subjectKind: subject.kind,
			alsoSubjectPath: alsoKind ? subject.path : undefined,
			alsoSubjectKind: alsoKind
		})
	}

	// Which dataset this subject was last worked in. Opening on someone else's dataset would read
	// as this agent's history when it is not, but opening on the one you were in yesterday is
	// exactly where you left off.
	let lastDatasetKey = $derived(`evals:dataset:${ws}:${subject.kind}:${subject.path}`)
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
			// The writable experiment is what you are working in, so it is what opens. A former
			// draft's runs are all closed, so this never lands on one of them.
			// The newest run of this subject is what opens; a former draft's runs sort below it.
			experimentId = (list.find((e) => e.subject.kind === subject.kind) ?? list[0])?.id
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
			regressed = 0
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
		regressed = results.regressed
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
					scores: [],
					carried: false
				}
		)
		return [...ordered, ...rows.filter((row) => !storedCases[row.case_id])]
	})

	/**
	 * Case runs that have not been saved as a run. Running one case is a job and nothing more —
	 * looking at what it did is not a claim that it belongs in the dataset's history — so the
	 * result sits here, over the row it belongs to, until it is saved or the pane is left.
	 */
	type UnsavedRun = {
		job_id: string
		status: string
		output?: string
		/** The scoring job, once the run has finished and the scorers have been sent after it. */
		score_job_id?: string
		scores?: ScoreCaseResultResponse
	}
	let unsaved = $state<Record<string, UnsavedRun>>({})
	let unsavedCount = $derived(Object.keys(unsaved).length)
	let savingRun = $state(false)

	async function refreshUnsaved() {
		if (!ws) return
		const entries = Object.entries($state.snapshot(unsaved))
		for (const [caseId, cell] of entries) {
			if (cell.status !== 'running') continue
			try {
				const job = (await JobService.getJob({
					workspace: ws,
					id: cell.job_id,
					noLogs: true
				})) as Job & { result?: any; success?: boolean }
				if (job.type === 'CompletedJob') {
					unsaved[caseId] = {
						...cell,
						status: job.success ? 'success' : 'failure',
						output: agentAnswer(job.result)
					}
				}
			} catch {
				// A job that has not been created yet, or was retained away: the cell says running
				// until it says otherwise.
			}
		}
	}

	/**
	 * A finished run is scored where it stands, so its numbers are there before the decision to
	 * keep it: the scoring job is carried into the run when it is saved, so what was looked at is
	 * what is stored.
	 */
	async function scoreUnsaved() {
		if (!ws || !selectedDataset || scorers.length === 0) return
		for (const [caseId, cell] of Object.entries($state.snapshot(unsaved))) {
			if (cell.status !== 'success') continue
			try {
				if (!cell.score_job_id) {
					const scoreJob = await AiEvalsService.scoreCaseRun({
						workspace: ws,
						requestBody: { dataset: selectedDataset, case_id: caseId, job_id: cell.job_id }
					})
					unsaved[caseId] = { ...unsaved[caseId], score_job_id: scoreJob }
					continue
				}
				if (cell.scores?.every((s) => !s.pending)) continue
				const scores = await AiEvalsService.scoreCaseResult({
					workspace: ws,
					dataset: selectedDataset,
					jobId: cell.score_job_id
				})
				unsaved[caseId] = { ...unsaved[caseId], scores }
			} catch (e) {
				sendUserToast(`Failed to score the rerun: ${e}`, true)
				// Dropped rather than retried every two seconds: the run is still there to save.
				unsaved[caseId] = { ...unsaved[caseId], scores: [] }
			}
		}
	}

	/** The agent's own result is `{output, messages}`; the answer is its `output`. */
	function agentAnswer(result: any): string | undefined {
		const output = result?.output
		if (output == undefined) return undefined
		return typeof output === 'string' ? output : JSON.stringify(output)
	}

	/** Makes a run of what has been rerun: seeded from the one on screen, so it is a whole run. */
	async function saveUnsavedRun() {
		if (!ws || !selectedDataset || unsavedCount === 0) return
		savingRun = true
		try {
			const id = await AiEvalsService.saveRun({
				workspace: ws,
				requestBody: {
					dataset: selectedDataset,
					seeded_from: experimentId,
					runs: Object.values($state.snapshot(unsaved)).map((cell) => ({
						job_id: cell.job_id,
						score_job_id: cell.score_job_id
					}))
				}
			})
			unsaved = {}
			experiments = await listSubjectExperiments()
			baselineId = experimentId
			experimentId = id
			pendingScore = true
			await loadResults()
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to save the run: ${e}`, true)
		} finally {
			savingRun = false
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
			const state = await AiEvalsService.evalSubjectState({
				workspace: ws,
				kind: subject.kind,
				path: subject.path
			})
			currentVersion = state.version
			currentDraftHash = state.draft_hash
			undeployedChanges = state.has_undeployed_changes
		} catch {
			// The agent was deleted or is no longer readable: the table keeps what it last knew
			// rather than claiming everything went stale.
		}
	}
	$effect(() => {
		// Restarted when the subject changes, so switching to the draft watches the draft.
		subject.kind
		subject.path
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
			Object.values(unsaved).some(
				(cell) =>
					cell.status === 'running' ||
					(cell.status === 'success' &&
						scorers.length > 0 &&
						(!cell.scores || cell.scores.some((s) => s.pending)))
			) ||
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
		await refreshUnsaved()
		await scoreUnsaved()
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
			unsaved = {}
			pendingScore = true
			await loadResults()
			await refresh()
		} catch (e) {
			sendUserToast(`Failed to run the dataset: ${e}`, true)
		} finally {
			running = false
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
			unsaved[caseId] = { job_id: res.job_id, status: 'running' }
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

	async function score(options: {
		experiment: string
		scorerIds?: string[]
		force?: boolean
		caseIds?: string[]
	}) {
		if (!ws || !selectedDataset) return
		try {
			const res = await AiEvalsService.scoreExperiment({
				workspace: ws,
				requestBody: {
					dataset: selectedDataset,
					experiment_id: options.experiment,
					scorer_ids: options.scorerIds,
					case_ids: options.caseIds,
					force: options.force
				}
			})
			sendUserToast(
				`Scoring ${res.jobs} run${res.jobs === 1 ? '' : 's'}` +
					(res.scored ? `, ${res.scored} settled without a job` : '') +
					(res.unscorable ? `, ${res.unscorable} with no answer to score` : '')
			)
			await loadResults()
		} catch (e) {
			sendUserToast(`Scoring failed: ${e}`, true)
		}
	}

	/** Renaming moves the dataset: its cases and experiments follow it through the foreign keys. */
	async function renameDataset() {
		if (!ws || !selectedDataset || !dataset || !renamePath || renameError) return
		try {
			await AiEvalsService.updateEvalDataset({
				workspace: ws,
				path: selectedDataset,
				requestBody: {
					path: renamePath,
					summary: dataset.summary,
					description: dataset.description,
					default_subject: dataset.default_subject,
					scorers: dataset.scorers
				}
			})
			await loadDatasets()
			renaming = false
			selectedDataset = renamePath
		} catch (e) {
			sendUserToast(`Failed to rename the dataset: ${e}`, true)
		}
	}

	/** Creates the dataset the first case needs. Naming it is a decision worth making after you
	 *  know what is in it, so it is made for you here and renamed from the toolbar. */
	async function createDataset(): Promise<string | undefined> {
		if (!ws || !newDatasetPath) return undefined
		creatingDataset = true
		try {
			const path = newDatasetPath
			await AiEvalsService.createEvalDataset({
				workspace: ws,
				requestBody: {
					path,
					default_subject: draft ? undefined : { kind: 'agent', path: agentPath ?? '' }
				}
			})
			await loadDatasets()
			selectedDataset = path
			newDatasetPath = ''
			return path
		} catch (e) {
			sendUserToast(`Failed to create the dataset: ${e}`, true)
			return undefined
		} finally {
			creatingDataset = false
		}
	}

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
		if (!ws) return
		// The first case is what a dataset is for, so asking for one is what creates it.
		const path = selectedDataset ?? (await createDataset())
		if (!path) return
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
		// Re-reading with a baseline is what turns every column into a comparison.
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
	 * What a run is called in the picker. The star means the definition that ran is not a
	 * deployed version: an edited agent is `v15*`, the version its edits are on top of, and a
	 * step with no agent of its own has nothing to number, so it is `draft*`.
	 */
	function experimentTitle(e: EvalExperiment): string {
		// A draft run whose configuration was then deployed is a run of that version: the star
		// says "not a deployed version", and once it is one, it no longer applies.
		const deployed =
			e.subject.kind === 'agent' ||
			(e.subject.draft_hash != undefined && e.subject.draft_hash === deployedHash)
		const version = deployed
			? currentVersionOf(e)
			: e.subject.version
				? `v${e.subject.version}*`
				: 'draft*'
		return `${experimentName(e)} · ${version} · ${e.case_count}`
	}

	/** A run of what is deployed is named by the version, which for a draft is the current one. */
	function currentVersionOf(e: EvalExperiment): string {
		const version = e.subject.kind === 'agent' ? e.subject.version : currentVersion
		return version ? `v${version}` : 'deployed'
	}

	let experimentItems = $derived(
		experiments.map((e) => ({ label: experimentTitle(e), value: e.id }))
	)
	let datasetItems = $derived(datasets.map((d) => ({ label: d.path, value: d.path })))

	/** The column header is the scorer's control: everything a column can do lives here. */
	function scorerMenu(scorer: Scorer, mean: ScorerMean | undefined): Item[] {
		const runs = rows.filter((row) => row.status === 'success').length
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
				displayName: `Rescore this experiment · ${runs} run${runs === 1 ? '' : 's'}`,
				disabled: !experimentId || runs === 0,
				action: () =>
					experimentId && score({ experiment: experimentId, scorerIds: [scorer.id!], force: true })
			},
			{
				// The baseline is scored from the answers it stored, so this costs scoring calls and
				// no agent calls. The count is the price, which is why it is on the item.
				displayName: `Score ${baseline ? experimentName(baseline) : 'the baseline'} · ${
					mean?.missing_in_baseline ?? 0
				} runs`,
				hide: !baseline || (mean?.missing_in_baseline ?? 0) === 0,
				action: () => baselineId && score({ experiment: baselineId, scorerIds: [scorer.id!] })
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

	// The case is the widest thing on the row and the scores are the point of it, so the columns
	// are sized rather than divided equally: an empty gap between a case and its numbers is what
	// made the table read as a list of unrelated values.
	let scorerColumnWidth = $derived(scorers.length > 0 ? Math.min(18, 46 / scorers.length) : 0)
	let caseColumnWidth = $derived(82 - scorerColumnWidth * scorers.length)

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
	 * A row describes an agent that no longer exists: it ran a version the agent has moved off, or
	 * — for a draft, which moves without its version changing — a configuration since edited.
	 */
	function isStale(row: ExperimentRow): boolean {
		if (row.subject_draft_hash != undefined) {
			// The edits it ran were saved: it is a run of what is deployed, whatever it was called
			// when it ran.
			if (row.subject_draft_hash === deployedHash) return false
			if (currentDraftHash != undefined && row.subject_draft_hash !== currentDraftHash) {
				return true
			}
		}
		return (
			row.subject_version != undefined &&
			currentVersion != undefined &&
			row.subject_version !== currentVersion
		)
	}
	let staleRows = $derived(rows.filter(isStale))
	let staleFrom = $derived(
		staleRows.length > 0 ? Math.min(...staleRows.map((row) => row.subject_version ?? 0)) : undefined
	)
	// Which of the two the alert is about: a draft that was edited, or a version left behind.
	let staleDraft = $derived(staleRows.some((row) => row.subject_draft_hash != undefined))
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
									renamePath = selectedDataset ?? ''
									renaming = true
									close()
								}}
							>
								<Pencil size={13} />
								Rename this dataset
							</button>
						{/if}
						<button
							type="button"
							class="flex items-center gap-2 px-3 py-2 text-xs text-secondary hover:bg-surface-hover"
							onclick={() => {
								selectedDataset = undefined
								newDatasetPath = ''
								renaming = false
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
		{#if baseline && regressed > 0}
			<span class="text-2xs text-red-500" title="Cells scoring lower than the baseline">
				{regressed} regressed
			</span>
		{/if}
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
		<Button
			size="xs"
			variant="accent"
			startIcon={{ icon: Play }}
			disabled={!selectedDataset || running || displayRows.length === 0 || !subject.path}
			onclick={runAll}
		>
			Run all
		</Button>
	</div>

	{#if unsavedCount > 0}
		<div class="px-3 py-2 border-b">
			<Alert type="info" size="xs" title="Unsaved case runs">
				<div class="flex items-center gap-3 flex-wrap">
					<span class="text-xs">
						{unsavedCount}
						{unsavedCount === 1 ? 'case has' : 'cases have'} been rerun and {unsavedCount === 1
							? 'is'
							: 'are'} shown over {experiment ? experimentName(experiment) : 'this run'}. Saving
						makes a run of them, carrying the cases you did not rerun; leaving loses them.
					</span>
					<Button
						size="xs2"
						variant="accent"
						disabled={savingRun || Object.values(unsaved).some((c) => c.status === 'running')}
						onclick={saveUnsavedRun}
					>
						Save as a new run
					</Button>
					<Button size="xs2" variant="subtle" disabled={savingRun} onclick={() => (unsaved = {})}>
						Discard
					</Button>
				</div>
			</Alert>
		</div>
	{/if}

	{#if undeployedChanges || runDraft}
		<div class="px-3 py-2 border-b">
			<Alert
				type={undeployedChanges ? 'warning' : 'info'}
				size="xs"
				title={runDraft ? "You are running the agent's draft" : 'The agent has undeployed changes'}
			>
				<div class="flex items-center gap-3 flex-wrap">
					<span class="text-xs">
						{#if runDraft && undeployedChanges}
							These runs execute the draft as it is now. Their results are kept apart from the
							deployed agent's, and they carry no version until it is deployed.
						{:else if runDraft}
							The draft is gone: it was deployed or discarded. These runs are the history of what it
							was, and a new one would run the same agent as the deployed one.
						{:else}
							A run resolves the agent as it is deployed, so these numbers describe v{currentVersion},
							not the edits waiting in its draft.
						{/if}
					</span>
					<Button size="xs2" variant="default" onclick={() => (runDraft = !runDraft)}>
						{runDraft ? 'Run the deployed agent' : 'Run the draft instead'}
					</Button>
				</div>
			</Alert>
		</div>
	{/if}

	{#if staleRows.length > 0}
		<div class="px-3 py-2 border-b">
			<Alert
				type="warning"
				size="xs"
				title="The agent changed since these runs"
				collapsible={false}
			>
				<div class="flex items-center gap-3 flex-wrap">
					<span class="text-xs">
						{staleRows.length}
						{staleRows.length === 1 ? 'row ran' : 'rows ran'}
						{#if staleDraft}
							an earlier state of the draft
						{:else}
							against v{staleFrom}, and the agent is on v{currentVersion}
						{/if}. Their scores describe an agent that no longer exists.
					</span>
					<Button
						size="xs2"
						variant="accent"
						startIcon={{ icon: Play }}
						disabled={running || !subject.path}
						onclick={runAll}
					>
						Rerun
					</Button>
				</div>
			</Alert>
		</div>
	{/if}

	{#if renaming}
		<div class="flex items-end gap-2 px-3 py-2 border-b bg-surface-secondary">
			<div class="grow">
				<Path
					bind:path={renamePath}
					bind:error={renameError}
					initialPath={selectedDataset ?? ''}
					checkInitialPathExistence={false}
					namePlaceholder="dataset1"
					kind="resource"
					workspaceOverride={ws}
					autofocus={false}
					size="sm"
				/>
			</div>
			<Button
				size="xs"
				variant="default"
				disabled={!renamePath || !!renameError || renamePath === selectedDataset}
				onclick={renameDataset}
			>
				Rename
			</Button>
			<Button size="xs" variant="subtle" onclick={() => (renaming = false)}>Cancel</Button>
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
					{:else}
						<DataTable size="sm" tableFixed>
							<colgroup>
								<col style={`width: ${caseColumnWidth}%`} />
								<col style="width: 9%" />
								{#each scorers as scorer (scorer.id)}
									<col style={`width: ${scorerColumnWidth}%`} />
								{/each}
								<col style="width: 9%" />
							</colgroup>
							<Head>
								<tr>
									<Cell head first>Case</Cell>
									<Cell head>Status</Cell>
									{#each scorers as scorer (scorer.id)}
										{@const mean = means.find((m) => m.scorer_id === scorer.id)}
										<Cell head numeric>
											<!-- The trigger lays itself out, so it is wrapped rather than trusted to
											     inherit the cell's alignment: a header that does not sit over its own
											     numbers is why a column reads as unrelated values. -->
											<div class="flex flex-col items-end gap-0.5">
												<DropdownV2 items={() => scorerMenu(scorer, mean)} placement="bottom-end">
													{#snippet buttonReplacement()}
														<span
															class="inline-flex items-center gap-1 cursor-pointer max-w-full"
															title={scorer.path}
														>
															{#if scorer.kind === 'agent'}
																<Bot size={13} class="text-tertiary shrink-0" />
															{:else}
																<Code2 size={13} class="text-tertiary shrink-0" />
															{/if}
															<span class="truncate">{scorerLabel(scorer)}</span>
															{#if mean?.definition_changed}
																<span
																	class="text-yellow-500 shrink-0"
																	title="This scorer changed between the two experiments, so the delta is a change of scorer as much as of agent"
																>
																	●
																</span>
															{/if}
															<ChevronDown size={12} class="text-tertiary shrink-0" />
														</span>
													{/snippet}
												</DropdownV2>
												{#if mean?.mean != undefined}
													<!-- The column's mean sits under the name it is a mean of: a number
													     away from its column is a number whose meaning has to be guessed. -->
													<span
														class={`inline-flex items-baseline gap-1.5 ${staleRows.length > 0 ? 'opacity-40' : ''}`}
														title={`Mean of ${mean.scored} scored case${mean.scored === 1 ? '' : 's'}`}
													>
														<span class="tabular-nums text-emphasis font-semibold">
															{formatScore(mean.mean)}
														</span>
														{#if mean.baseline_mean != undefined}
															{@const delta = mean.mean - mean.baseline_mean}
															<span
																class={`text-2xs tabular-nums font-normal ${delta > 0 ? 'text-green-500' : delta < 0 ? 'text-red-500' : 'text-tertiary'}`}
															>
																{formatDelta(delta)}
															</span>
														{/if}
													</span>
												{/if}
											</div>
										</Cell>
									{/each}
									<Cell head last></Cell>
								</tr>
							</Head>
							<tbody class="divide-y">
								{#each displayRows as row (row.case_id)}
									{@const pending = unsaved[row.case_id]}
									{@const status = statusOf(pending?.status ?? row.status)}
									{@const stale = !pending && isStale(row)}
									<Row selected={row.case_id === selectedCaseId} on:click={() => openCase(row)}>
										<Cell first>
											<div class="flex flex-col min-w-0">
												<span class="truncate text-emphasis">{caseLabel(row)}</span>
												<span
													class={`truncate text-2xs ${pending ? 'text-blue-500' : 'text-secondary'}`}
												>
													{pending?.output ?? row.output ?? row.input?.user_message ?? ''}
												</span>
											</div>
										</Cell>
										<Cell>
											<!-- The icon is the status and nothing else: the version belongs to the run,
											     which the picker names, and a second number here was read as the case's.
											     The spin is on the icon, not on the cell around it. -->
											<span
												class="inline-flex items-center"
												title={stale
													? `${status.label} · ran an earlier state of this agent`
													: row.subject_version
														? `${status.label} · v${row.subject_version}`
														: status.label}
											>
												<status.icon size={14} class={status.class} />
											</span>
										</Cell>
										{#each scorers as scorer (scorer.id)}
											{@const cell = pending
												? pending.scores?.find((s) => s.scorer_id === scorer.id)
												: row.scores.find((s) => s.scorer_id === scorer.id)}
											<Cell numeric>
												{#if pending && (pending.status === 'running' || !cell || cell.pending)}
													<!-- A rerun is scored where it stands, so the wait here is the scorers
													     running, not a number withheld until it is saved. -->
													<span
														class="inline-flex justify-end"
														title={pending.status === 'running' ? 'Running' : 'Scoring'}
													>
														<Loader2 size={13} class="animate-spin text-blue-500" />
													</span>
												{:else if cell?.pending}
													<span class="inline-flex justify-end" title="Scoring">
														<Loader2 size={13} class="animate-spin text-blue-500" />
													</span>
												{:else if cell?.score != undefined}
													<!-- The number is the verdict; why it was given is the part worth
													     reading, so it is one hover away rather than in a browser
													     tooltip that arrives after a second and wraps at 80 columns. -->
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
																<!-- The judge is not deterministic and a scorer gets edited:
																     rescoring one cell costs one scoring call and no agent
																     call, so it belongs where the number is. -->
																<Button
																	size="xs2"
																	variant="subtle"
																	startIcon={{ icon: RefreshCw }}
																	disabled={!experimentId}
																	on:click={(e) => {
																		e.stopPropagation()
																		experimentId &&
																			score({
																				experiment: experimentId,
																				scorerIds: [scorer.id!],
																				caseIds: [row.case_id],
																				force: true
																			})
																	}}
																>
																	Score again
																</Button>
															</div>
														{/snippet}
														<span
															class={`inline-flex items-baseline gap-1.5 justify-end ${stale ? 'opacity-40' : ''}`}
														>
															<span
																class={`tabular-nums font-medium ${pending ? 'text-blue-500' : 'text-emphasis'}`}
															>
																{formatScore(cell.score)}{#if pending}<span
																		title="Scored just now, and not saved yet">*</span
																	>{/if}
															</span>
															{#if !pending && (cell as CellScore).baseline != undefined}
																{@const delta = cell.score - (cell as CellScore).baseline!}
																<span
																	class={`text-2xs tabular-nums ${delta > 0 ? 'text-green-500' : delta < 0 ? 'text-red-500' : 'text-tertiary'}`}
																>
																	{formatDelta(delta)}
																</span>
															{/if}
														</span>
													</Popover>
												{:else if cell?.error}
													<span class="text-2xs text-red-500" title={cell.error}>failed</span>
												{:else}
													<span class="text-2xs text-tertiary">—</span>
												{/if}
											</Cell>
										{/each}
										<Cell last>
											<div class="flex items-center gap-1 justify-end">
												<Button
													size="xs2"
													variant="default"
													startIcon={{ icon: Play }}
													iconOnly
													title="Run this case"
													disabled={running || !subject.path}
													on:click={(e) => {
														e.stopPropagation()
														runCase(row.case_id)
													}}
												/>
												<Button
													size="xs2"
													variant="subtle"
													startIcon={{ icon: Trash2 }}
													iconOnly
													title="Delete this case"
													on:click={(e) => {
														e.stopPropagation()
														deleteCase(row.case_id)
													}}
												/>
											</div>
										</Cell>
									</Row>
								{/each}
								<tr>
									<td colspan={scorers.length + 3} class="p-2">
										<div class="flex items-center gap-3">
											<Button
												size="xs2"
												variant="subtle"
												startIcon={{ icon: Plus }}
												disabled={creatingDataset || (!selectedDataset && !newDatasetPath)}
												onclick={addCase}
											>
												Add a case
											</Button>
											{#if !selectedDataset && newDatasetPath}
												<!-- Said before it happens rather than after: the first case creates the
												     dataset, whose name is renameable from the toolbar. -->
												<span class="text-2xs text-tertiary">
													starts the dataset {newDatasetPath}
												</span>
											{/if}
										</div>
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
							{#if job}
								<EvalRunResult
									{job}
									workspace={ws}
									historyPath={agentPath && selectedDataset && selectedCaseId
										? caseRunPath(agentPath, selectedDataset, selectedCaseId)
										: undefined}
								/>
							{/if}
						</div>
					</div>
				</Pane>
			{/if}
		</Splitpanes>
	</div>
</div>

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

<ResourceEditorDrawer bind:this={resourceEditorDrawer} workspace={ws} onRestored={loadResults} />

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
