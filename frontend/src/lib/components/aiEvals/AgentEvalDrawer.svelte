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
		type Job
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { Plus, Trash2, Bot, FlaskConical } from 'lucide-svelte'
	import type { AgentTool } from '$lib/components/flows/agentToolUtils'
	import EvalCaseEditor from './EvalCaseEditor.svelte'
	import EvalRunResult from './EvalRunResult.svelte'
	import {
		caseLabel,
		caseRunPath,
		emptyCase,
		fromCaptureDraft,
		fromStoredCase,
		type CaseDraft
	} from './evalCaseUtils'

	let {
		agentPath = $bindable(undefined),
		open = $bindable(false),
		capture = undefined
	}: {
		/** The agent under test. Every entry point knows one, but it stays overridable so the same
		 * dataset can be pointed at another agent. */
		agentPath?: string
		open?: boolean
		/** A case captured from a run or a conversation, opened for review before saving. */
		capture?: EvalCaseDraft
	} = $props()

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

	async function loadDatasets() {
		if (!$workspaceStore) return
		datasets = await AiEvalsService.listEvalDatasets({ workspace: $workspaceStore })
	}

	const CASE_PAGE_SIZE = 100

	async function loadCases(path: string | undefined, page = 1) {
		if (!$workspaceStore || !path) {
			cases = []
			totalCases = 0
			return
		}
		loadingCases = true
		try {
			const res = await AiEvalsService.listEvalCases({
				workspace: $workspaceStore,
				path,
				page,
				perPage: CASE_PAGE_SIZE
			})
			cases = page === 1 ? (res.cases ?? []) : [...cases, ...(res.cases ?? [])]
			totalCases = res.total ?? cases.length
		} finally {
			loadingCases = false
		}
	}

	async function loadAgents() {
		if (!$workspaceStore) return
		const resources = await ResourceService.listResource({
			workspace: $workspaceStore,
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
		if (!$workspaceStore || !path) return
		const [resource, history] = await Promise.all([
			ResourceService.getResource({ workspace: $workspaceStore, path }),
			ResourceService.getResourceHistory({ workspace: $workspaceStore, path }).catch(
				() => undefined
			)
		])
		agentTools = ((resource.value as any)?.tools ?? []) as AgentTool[]
		agentVersion = history?.versions?.[0]?.id
	}

	$effect(() => {
		if (open) {
			loadDatasets()
			loadAgents()
		}
	})
	// Gated on `open`: the drawer is mounted next to every AI agent step in the flow editor, and
	// fetching each one's resource on mount would be a request per step for a panel nobody opened.
	$effect(() => {
		if (open) loadAgent(agentPath)
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

	function setDraft(next: CaseDraft) {
		draft = next
		draftGeneration += 1
		job = undefined
	}

	function selectCase(c: EvalCase) {
		setDraft(fromStoredCase(c))
	}

	function newCase() {
		setDraft(emptyCase())
	}

	async function createDataset() {
		if (!$workspaceStore || !newDatasetPath) return
		await AiEvalsService.createEvalDataset({
			workspace: $workspaceStore,
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
		if (!$workspaceStore || !selectedDataset) {
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
				workspace: $workspaceStore,
				path: selectedDataset,
				requestBody: { id: draft.id, ...body }
			})
			sendUserToast('Case updated')
		} else {
			const id = await AiEvalsService.addEvalCase({
				workspace: $workspaceStore,
				path: selectedDataset,
				requestBody: body
			})
			draft = { ...draft, id }
			sendUserToast('Case saved to dataset')
		}
		await loadCases(selectedDataset)
	}

	async function deleteCase(c: EvalCase) {
		if (!$workspaceStore || !selectedDataset) return
		await AiEvalsService.deleteEvalCase({
			workspace: $workspaceStore,
			path: selectedDataset,
			requestBody: { id: c.id }
		})
		if (draft.id === c.id) newCase()
		await loadCases(selectedDataset)
	}

	async function run() {
		if (!$workspaceStore || !agentPath) {
			sendUserToast('Pick an agent to run against', true)
			return
		}
		const workspace = $workspaceStore
		const path = agentPath
		// A saved case is run by reference so the job records which case it executed; an unsaved one
		// is sent inline and produces a run with no case to trace back to.
		const stored = draft.id && selectedDataset
		const callbacks: Callbacks = {
			done: (j) => {
				job = j
				running = false
			},
			doneError: ({ error }) => {
				running = false
				sendUserToast(String(error), true)
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

<Drawer bind:open size="1100px">
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
							The version this agent is at now. A run records the version it resolved to, so a
							result stays attributable to a prompt state. A version captures the resource only: a
							`$var:` it references can change underneath two identical versions.
						</Tooltip>
					</span>
				{/if}
			</div>
		{/snippet}

		<Splitpanes class="h-full">
			<Pane size={26} minSize={18}>
				<div class="flex flex-col h-full min-h-0 pr-2 gap-2">
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

					<Button variant="subtle" size="xs" startIcon={{ icon: FlaskConical }} onclick={newCase}>
						New case
					</Button>

					<div class="flex-1 min-h-0 overflow-auto">
						{#if loadingCases}
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
							<div class="flex flex-col">
								{#each cases as c (c.id)}
									<div
										class="flex items-center justify-between gap-1 group rounded-md px-2 py-1 hover:bg-surface-hover {draft.id ===
										c.id
											? 'bg-surface-selected'
											: ''}"
									>
										<button class="text-xs text-left truncate grow" onclick={() => selectCase(c)}>
											{caseLabel(c)}
										</button>
										<Button
											variant="subtle"
											size="xs2"
											startIcon={{ icon: Trash2 }}
											iconOnly
											btnClasses="opacity-0 group-hover:opacity-100"
											onclick={() => (deleting = c)}
										/>
									</div>
								{/each}
							</div>
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
				</div>
			</Pane>
			<Pane size={37} minSize={25}>
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
			<Pane size={37} minSize={25}>
				<div class="h-full pl-2">
					<EvalRunResult {job} tools={agentTools} {historyPath} />
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>
