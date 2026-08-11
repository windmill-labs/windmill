<script lang="ts">
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import JobLoader, { type Callbacks } from '$lib/components/JobLoader.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import {
		AiEvalsService,
		ResourceService,
		type EvalCase,
		type EvalCaseDraft,
		type EvalDataset,
		type Job,
		JobService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { Plus, Trash2, Bot, FlaskConical, MessagesSquare, Play } from 'lucide-svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import type { AgentTool } from '$lib/components/flows/agentToolUtils'
	import EvalCaseEditor from './EvalCaseEditor.svelte'
	import EvalRunResult from './EvalRunResult.svelte'
	import ExperimentResults from './ExperimentResults.svelte'
	import ScorerPicker from './ScorerPicker.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { Tab } from '$lib/components/common'
	import { deepEqual } from 'fast-equals'
	import {
		caseLabel,
		caseRunPath,
		comparableCase,
		emptyCase,
		fromCaptureDraft,
		fromStoredCase,
		type CaseDraft
	} from './evalCaseUtils'

	let {
		agentPath = $bindable(),
		open = $bindable(),
		capture = undefined,
		opWorkspace = undefined
	}: {
		/** The agent under test. Every entry point knows one, but it stays overridable so the same
		 * dataset can be pointed at another agent. */
		agentPath?: string
		open?: boolean
		/** A case captured from a run or a conversation, opened for review before saving. */
		capture?: EvalCaseDraft
		/** The workspace the opening editor operates on, which differs from the nav workspace in
		 * fork and session editors. Every read and write targets it. */
		opWorkspace?: string
	} = $props()

	let isOpen = $derived(open ?? false)
	let ws = $derived(opWorkspace ?? $workspaceStore)

	let datasets = $state<EvalDataset[]>([])
	let selectedDataset = $state<string | undefined>(undefined)
	let cases = $state<EvalCase[]>([])
	let totalCases = $state(0)
	let loadingCases = $state(false)
	let draft = $state<CaseDraft>(emptyCase())
	let agentVersion = $state<number | undefined>(undefined)
	let agentTools = $state<AgentTool[]>([])
	let job = $state<(Job & { result?: any }) | undefined>(undefined)
	let running = $state(false)
	let jobLoader: JobLoader | undefined = $state(undefined)
	let agentOptions = $state<{ label: string; value: string }[]>([])
	let newDatasetPath = $state('')
	let deleting = $state<EvalCase | undefined>(undefined)
	// Bumped whenever `draft` is replaced wholesale. The case editor seeds local state from the
	// draft, so it is keyed on this and remounts rather than carrying one case's edits into the
	// next.
	let draftGeneration = $state(0)
	let paneTab = $state<'cases' | 'results'>('cases')
	let scorers = $state<{ kind: 'script' | 'flow' | 'agent'; path: string; name?: string }[]>([])
	let experimentToken = $state(0)
	let startedExperimentId = $state<string | undefined>(undefined)
	let startingExperiment = $state(false)

	async function loadDatasets() {
		if (!ws) return
		datasets = await AiEvalsService.listEvalDatasets({ workspace: ws })
	}

	const CASE_PAGE_SIZE = 100

	// Switching datasets leaves the previous request in flight; only the newest may write, or a
	// slow response for the dataset you just left replaces the one you are looking at.
	let caseLoadGeneration = 0
	let shownDataset: string | undefined = undefined

	async function loadCases(path: string | undefined, page = 1) {
		const generation = ++caseLoadGeneration
		if (!ws || !path) {
			cases = []
			totalCases = 0
			loadingCases = false
			return
		}
		if (page === 1 && path !== shownDataset) {
			// Cleared before the request, not after it: a failed load would otherwise leave the
			// previous dataset's cases rendered under the newly selected one. Reloading the same
			// dataset after a save keeps its rows rather than flashing a skeleton.
			cases = []
			totalCases = 0
		}
		shownDataset = path
		loadingCases = true
		try {
			const res = await AiEvalsService.listEvalCases({
				workspace: ws,
				path,
				page,
				perPage: CASE_PAGE_SIZE
			})
			if (generation !== caseLoadGeneration) return
			cases = page === 1 ? (res.cases ?? []) : [...cases, ...(res.cases ?? [])]
			totalCases = res.total ?? cases.length
		} catch (err) {
			if (generation === caseLoadGeneration) {
				sendUserToast((err as any)?.body ?? String(err), true)
			}
		} finally {
			if (generation === caseLoadGeneration) {
				loadingCases = false
			}
		}
	}

	async function loadAgents() {
		if (!ws) return
		const resources = await ResourceService.listResource({
			workspace: ws,
			resourceType: 'ai_agent',
			perPage: 1000
		})
		agentOptions = resources.map((r) => ({ label: r.path, value: r.path }))
	}

	// The agent's tools label the tool calls in the trajectory, and its version number is what makes
	// a run attributable to a prompt state later.
	async function loadAgent(path: string | undefined) {
		agentTools = []
		agentVersion = undefined
		if (!ws || !path) return
		const [resource, history] = await Promise.all([
			ResourceService.getResource({ workspace: ws, path }),
			ResourceService.getResourceHistory({ workspace: ws, path }).catch(() => undefined)
		])
		agentTools = ((resource.value as any)?.tools ?? []) as AgentTool[]
		agentVersion = history?.versions?.[0]?.id
	}

	$effect(() => {
		if (isOpen) {
			loadDatasets()
			loadAgents()
		}
	})
	// Gated on `open`: the drawer is mounted next to every AI agent step in the flow editor, and
	// fetching each one's resource on mount would be a request per step for a panel nobody opened.
	$effect(() => {
		if (isOpen) loadAgent(agentPath)
	})
	// A case id belongs to one dataset. Keeping the draft across a dataset switch would aim Save
	// and Run at a case the new dataset does not have, so the draft starts fresh — except for a
	// capture, whose whole point is to survive until it is saved somewhere.
	let lastDataset: string | undefined = undefined
	$effect(() => {
		const dataset = selectedDataset
		untrack(() => {
			if (dataset !== lastDataset) {
				lastDataset = dataset
				if (draft.id) newCase()
			}
			loadCases(dataset)
		})
	})
	// Applied once per capture, by identity: `setDraft` writes state this effect would otherwise
	// re-read, and re-applying would also throw away whatever the user had started editing.
	let appliedCapture: EvalCaseDraft | undefined = undefined
	$effect(() => {
		const captured = capture
		if (!captured || captured === appliedCapture) return
		appliedCapture = captured
		untrack(() => {
			setDraft(fromCaptureDraft(captured))
			if (captured.agent_path) agentPath = captured.agent_path
		})
	})

	// The stored form of the selected case, kept independently of the loaded pages: looking the
	// baseline up in `cases` made an off-page case read as unedited, so Run sent it by reference
	// and evaluated the persisted inputs instead of the visible ones.
	let draftBaseline = $state<unknown | undefined>(undefined)

	function setDraft(next: CaseDraft, baseline?: unknown) {
		draftBaseline = baseline
		draft = next
		draftGeneration += 1
		job = undefined
	}

	function selectCase(c: EvalCase) {
		const stored = fromStoredCase(c)
		setDraft(stored, comparableCase(stored))
	}

	function newCase() {
		setDraft(emptyCase())
	}

	async function createDataset() {
		if (!ws || !newDatasetPath) return
		await AiEvalsService.createEvalDataset({
			workspace: ws,
			requestBody: {
				path: newDatasetPath,
				default_subject: agentPath ? { kind: 'agent', path: agentPath } : undefined
			}
		})
		sendUserToast(`Created eval dataset ${newDatasetPath}`)
		selectedDataset = newDatasetPath
		newDatasetPath = ''
		await loadDatasets()
	}

	async function saveCase() {
		if (!ws || !selectedDataset) {
			sendUserToast('Select a dataset to save this case to', true)
			return
		}
		const body = {
			name: draft.name,
			input: draft.input,
			host_flow_path: draft.host_flow_path,
			tool_inputs: draft.tool_inputs,
			expected: draft.expected,
			tags: draft.tags,
			source: draft.source
		}
		if (draft.id) {
			await AiEvalsService.updateEvalCase({
				workspace: ws,
				path: selectedDataset,
				requestBody: { id: draft.id, ...body }
			})
			draftBaseline = comparableCase(draft)
			sendUserToast('Case updated')
		} else {
			const id = await AiEvalsService.addEvalCase({
				workspace: ws,
				path: selectedDataset,
				requestBody: body
			})
			draft = { ...draft, id }
			draftBaseline = comparableCase(draft)
			sendUserToast('Case saved to dataset')
		}
		await reloadCases()
	}

	// Re-fetch as many pages as were loaded, so a write does not silently collapse the list back
	// to its first page.
	async function reloadCases() {
		const pages = Math.max(1, Math.ceil(cases.length / CASE_PAGE_SIZE))
		for (let page = 1; page <= pages; page++) {
			await loadCases(selectedDataset, page)
		}
	}

	async function deleteCase(c: EvalCase) {
		if (!ws || !selectedDataset) return
		await AiEvalsService.deleteEvalCase({
			workspace: ws,
			path: selectedDataset,
			requestBody: { id: c.id }
		})
		if (draft.id === c.id) newCase()
		await reloadCases()
	}

	async function runExperiment() {
		if (!ws || !agentPath || !selectedDataset) {
			sendUserToast('Pick an agent and a dataset first', true)
			return
		}
		startingExperiment = true
		try {
			startedExperimentId = await AiEvalsService.runExperiment({
				workspace: ws,
				requestBody: {
					dataset: selectedDataset,
					subject: { kind: 'agent', path: agentPath },
					scorers
				}
			})
			paneTab = 'results'
			// The rows fill in as the per-case jobs complete, so the table opens on a running
			// experiment rather than waiting for one.
			experimentToken += 1
			sendUserToast('Experiment started')
		} catch (err) {
			sendUserToast((err as any)?.body ?? String(err), true)
		} finally {
			startingExperiment = false
		}
	}

	async function run() {
		if (!ws || !agentPath) {
			sendUserToast('Pick an agent to run against', true)
			return
		}
		const workspace = ws
		const path = agentPath
		// A saved case is run by reference so the job records which case it executed. Unsaved edits
		// have to go inline instead — running the stored case while the editor shows something else
		// would silently test the wrong thing — and such a run has no case to trace back to.
		// No baseline for a case that claims an id means we cannot prove it is unchanged, so treat
		// it as edited: running the persisted case while the editor shows something else is the
		// silent failure, and an inline run is merely unstamped.
		const edited = draft.id != undefined && !deepEqual(draftBaseline, comparableCase(draft))
		const stored = draft.id && selectedDataset && !edited
		if (edited) {
			sendUserToast('Running the unsaved edits; save the case to record the run against it')
		}
		const callbacks: Callbacks = {
			done: (j) => {
				job = j
				running = false
			},
			doneError: ({ error }) => {
				running = false
				sendUserToast((error as any)?.body ?? error?.message ?? String(error), true)
			}
		}
		running = true
		job = undefined
		await jobLoader?.abstractRun(
			() =>
				AiEvalsService.runEval({
					workspace,
					requestBody: {
						subject: { kind: 'agent', path },
						...(stored
							? { dataset: selectedDataset, case_id: draft.id }
							: {
									case: {
										name: draft.name,
										input: draft.input,
										host_flow_path: draft.host_flow_path,
										tool_inputs: draft.tool_inputs
									}
								})
					}
				}),
			callbacks
		)
	}

	// One query for the whole table: every run of this dataset's cases is stamped with a path under
	// `<agent>/<dataset>/`, so the last run per case is a group-by on that rather than a request
	// per row. The child agent job shares the prefix with a `/a` suffix, so only exact case
	// segments count.
	let lastRunByCase = $state<Record<string, Job>>({})
	// True when the page bound was hit before every case was accounted for, so an empty cell means
	// "not found in recent runs" rather than "never ran".
	let runsTruncated = $state(false)
	let runsGeneration = 0
	async function loadLastRuns(dataset: string | undefined, agent: string | undefined) {
		const generation = ++runsGeneration
		lastRunByCase = {}
		runsTruncated = false
		if (!ws || !dataset || !agent) return
		// Read untracked: naming `cases` in a tracked position would make this whole job-history
		// query a dependency of the case list, refetching it on every save, delete and Load more.
		const wanted = untrack(() => new Set(cases.map((c) => c.id)))
		const prefix = `${agent}/${dataset}/`
		const RUNS_PAGE_SIZE = 200
		// Paged until every loaded case has been seen, because one newest-first page covers only
		// the most recent runs: a dataset with more history than that reported cases as never run.
		// Bounded, so a long history cannot turn opening a dataset into a crawl; a case not reached
		// within the bound reads as unknown rather than claiming it never ran.
		const MAX_RUN_PAGES = 5
		try {
			const byCase: Record<string, Job> = {}
			// listJobs pages by a created_before cursor, not a page number.
			let before: string | undefined = undefined
			for (let page = 0; page < MAX_RUN_PAGES; page++) {
				const jobs: Job[] = await JobService.listJobs({
					workspace: ws,
					scriptPathStart: prefix,
					isFlowStep: false,
					createdBefore: before,
					perPage: RUNS_PAGE_SIZE
				})
				if (generation !== runsGeneration) return
				for (const job of jobs) {
					const caseId = job.script_path?.slice(prefix.length)
					// listJobs is newest first, so the first hit per case is its latest run.
					if (caseId && !caseId.includes('/') && !byCase[caseId]) byCase[caseId] = job
				}
				const covered = [...wanted].every((id) => byCase[id])
				if (covered || jobs.length < RUNS_PAGE_SIZE) break
				before = jobs[jobs.length - 1]?.created_at
				if (!before) break
			}
			lastRunByCase = byCase
			runsTruncated = [...wanted].some((id) => !byCase[id])
		} catch {
			// A missing run history must not empty the table.
		}
	}
	$effect(() => {
		if (isOpen) loadLastRuns(selectedDataset, agentPath)
	})

	function caseSource(c: EvalCase): string {
		if (c.source?.job_id) return 'run'
		if (c.source?.conversation_id) return 'conversation'
		return 'manual'
	}

	let historyPath = $derived(
		draft.id && selectedDataset && agentPath
			? caseRunPath(agentPath, selectedDataset, draft.id)
			: undefined
	)
</script>

<JobLoader bind:this={jobLoader} bind:job />

<ConfirmationModal
	open={deleting != undefined}
	title="Delete case"
	confirmationText="Delete"
	on:canceled={() => (deleting = undefined)}
	on:confirmed={async () => {
		const c = deleting
		deleting = undefined
		if (c) await deleteCase(c)
	}}
>
	<span class="text-sm">Delete “{deleting ? caseLabel(deleting) : ''}” from this dataset?</span>
</ConfirmationModal>

<Drawer bind:open size="1400px">
	<DrawerContent
		title="Evals"
		tooltip="Run this agent on its own, and curate the cases you want it to keep handling."
		on:close={() => (open = false)}
	>
		{#snippet actions()}
			<div class="flex items-center gap-2">
				<Bot size={14} />
				<div class="w-64">
					<Select
						items={agentOptions}
						bind:value={agentPath}
						placeholder="Agent to run"
						clearable
						class="text-xs"
					/>
				</div>
				{#if agentVersion != undefined}
					<span class="text-2xs text-tertiary whitespace-nowrap">
						v{agentVersion}
						<Tooltip>
							The version this agent is at now. A run records the version at the moment it is
							enqueued, so a result stays attributable to a prompt state — a run that waits in
							the queue while the agent is edited executes a newer value than the one recorded.
							A version captures the resource only: a `$var:` it references can change underneath
							two identical versions.
						</Tooltip>
					</span>
				{/if}
			</div>
		{/snippet}

		<Splitpanes class="h-full">
			<Pane size={40} minSize={25}>
				<div class="flex flex-col h-full min-h-0 pr-2 gap-2">
					<Tabs bind:selected={paneTab}>
						<Tab value="cases" label="Cases" />
						<Tab value="results" label="Results" />
					</Tabs>
					<div class="flex items-center gap-1">
						<div class="grow min-w-0">
							<Select
								items={datasets.map((d) => ({ label: d.path, value: d.path }))}
								bind:value={selectedDataset}
								placeholder="Dataset"
								clearable
								class="text-xs"
							/>
						</div>
						<Popover>
							{#snippet trigger()}
								<Button
									variant="default"
									size="xs2"
									startIcon={{ icon: Plus }}
									iconOnly
									nonCaptureEvent
								/>
							{/snippet}
							{#snippet content()}
								<div class="flex flex-col gap-2 p-2 w-72">
									<span class="text-xs font-semibold">New dataset</span>
									<TextInput
										bind:value={newDatasetPath}
										size="sm"
										inputProps={{ placeholder: 'f/folder/name' }}
									/>
									<Button variant="accent" size="xs" onclick={createDataset}>Create</Button>
								</div>
							{/snippet}
						</Popover>
					</div>

					{#if paneTab === 'cases'}
						<Button variant="subtle" size="xs" startIcon={{ icon: FlaskConical }} onclick={newCase}>
							New case
						</Button>

						<div class="flex-1 min-h-0 overflow-auto">
							{#if loadingCases && cases.length === 0}
								<Skeleton layout={[[2], [2], [2]]} />
							{:else if !selectedDataset}
								<div class="text-xs text-tertiary p-2">
									Pick a dataset to see its cases, or run a one-off case on the right.
								</div>
							{:else if cases.length === 0}
								<div class="text-xs text-tertiary p-2">
									No case yet. Cases are best captured from real runs: open an AI agent run and add it
									here.
								</div>
							{:else}
								<DataTable size="xs" noBorder shouldHidePagination>
									<Head>
										<tr>
											<Cell head first>Case</Cell>
											<Cell head>From</Cell>
											<Cell head>Last run</Cell>
											<Cell head last></Cell>
										</tr>
									</Head>
									<tbody>
										{#each cases as c (c.id)}
											{@const lastRun = lastRunByCase[c.id]}
											<tr
												class="border-b last:border-b-0 cursor-pointer group hover:bg-surface-hover {draft.id ===
												c.id
													? 'bg-surface-selected'
													: ''}"
												onclick={() => selectCase(c)}
											>
												<Cell first>
													<div class="flex items-center gap-1 min-w-0">
														<span class="truncate">{caseLabel(c)}</span>
														{#if c.input?.messages?.length}
															<Tooltip><MessagesSquare size={12} /></Tooltip>
														{/if}
													</div>
												</Cell>
												<Cell>
													<span class="text-tertiary">{caseSource(c)}</span>
												</Cell>
												<Cell>
													{#if lastRun}
														<span
															class={lastRun.type === 'CompletedJob'
																? lastRun.success
																	? 'text-green-600'
																	: 'text-red-600'
																: 'text-tertiary'}
														>
															{lastRun.type === 'CompletedJob'
																? lastRun.success
																	? 'success'
																	: 'failure'
																: 'running'}
														</span>
													{:else}
														<span class="text-tertiary" title={runsTruncated
															? 'No run found in the most recent runs of this dataset'
															: 'Never run'}>
															{runsTruncated ? '—' : 'never'}
														</span>
													{/if}
												</Cell>
												<Cell last>
													<Button
														variant="subtle"
														size="xs2"
														startIcon={{ icon: Trash2 }}
														iconOnly
														btnClasses="opacity-0 group-hover:opacity-100"
														onclick={(e) => {
															e?.stopPropagation()
															deleting = c
														}}
													/>
												</Cell>
											</tr>
										{/each}
									</tbody>
								</DataTable>
								{#if totalCases > cases.length}
									<Button
										variant="subtle"
										size="xs2"
										disabled={loadingCases}
										onclick={() =>
											loadCases(selectedDataset, Math.floor(cases.length / CASE_PAGE_SIZE) + 1)}
									>
										Load more ({cases.length} of {totalCases})
									</Button>
								{/if}
							{/if}
						</div>
					{:else}
						<ScorerPicker bind:scorers workspace={ws} />
						<Button
							variant="accent"
							size="xs"
							startIcon={{ icon: Play }}
							disabled={!selectedDataset || !agentPath || startingExperiment}
							onclick={runExperiment}
						>
							{startingExperiment ? 'Starting' : 'Run dataset'}
						</Button>
						<div class="flex-1 min-h-0">
							<ExperimentResults
								dataset={selectedDataset}
								workspace={ws}
								refreshToken={experimentToken}
								selectExperimentId={startedExperimentId}
							/>
						</div>
					{/if}
				</div>
			</Pane>
			<Pane size={30} minSize={22}>
				<div class="h-full overflow-auto px-2">
					{#key draftGeneration}
						<EvalCaseEditor
							bind:draft
							{running}
							canSave={!!selectedDataset}
							saveLabel={draft.id ? 'Update case' : 'Save to dataset'}
							onRun={run}
							onSave={saveCase}
						/>
					{/key}
				</div>
			</Pane>
			<Pane size={30} minSize={22}>
				<div class="h-full pl-2">
					<EvalRunResult {job} tools={agentTools} {historyPath} workspace={ws} />
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>
