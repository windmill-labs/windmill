<script lang="ts">
	import { getContext, setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import { MessageCircleOff, SlidersHorizontal } from 'lucide-svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import PaneNotice from '$lib/components/common/paneNotice/PaneNotice.svelte'
	import { Alert, Button } from '$lib/components/common'
	import FlowChat from '../conversations/FlowChat.svelte'
	import {
		AGENT_CHAT_SCHEMA,
		agentChatFlow,
		agentChatGap,
		agentChatPath
	} from '../conversations/agentEditorChat'
	import { runFlowPreview } from '../utils.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$lib/navigation'
	import { deepEqual } from 'fast-equals'
	import type { Flow, FlowModule, InputTransform, Job, OpenFlow } from '$lib/gen'
	import { emptySchema, type StateStore } from '$lib/utils'
	import type { FlowInput } from '$lib/components/flows/types'
	import type { FlowEditorContext, FlowInputEditorState, FlowPanelDetachContext } from '../types'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { initFlowState, type FlowState } from '../flowState'
	import { insertAgentTool } from '../flowStateUtils.svelte'
	import { initHistory } from '$lib/history.svelte'
	import { StepsInputArgs } from '../stepsInputArgs.svelte'
	import { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { ModulesTestStates } from '$lib/components/modulesTest.svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'
	import { Drawer, DrawerContent } from '$lib/components/common'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import ModulePreviewResultViewer from '$lib/components/ModulePreviewResultViewer.svelte'
	import AiAgentStepInputs from './AiAgentStepInputs.svelte'
	import AgentToolWrapper from './AgentToolWrapper.svelte'
	import { getStepPropPicker } from '../previousResults'
	import {
		AGENT_BRAIN_KEYS,
		flowLocalAgentSchema,
		inputTransformsToAgentConfig,
		type AIAgentConfig
	} from '../agentResourceUtils'
	import { agentArgsToTransforms } from '../linkedAgentDrafts'
	import {
		AGENT_EDITOR_RUN_INPUTS,
		AGENT_TOOLS_ROW,
		DEFAULT_AGENT_MEMORY,
		keepsManagedMemory
	} from '../agentFormFields'
	import { toolDisplayName, type AgentTool } from '../agentToolUtils'
	import { useAgentDraft } from '../agentDraft.svelte'
	import Path from '$lib/components/Path.svelte'
	import Label from '$lib/components/Label.svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		/** The `ai_agent` resource being edited. */
		path: string
		workspace: string | undefined
		enableAi?: boolean
		/** The tool drilled into, if any. Owned by the caller so it can drive the breadcrumb. */
		toolId?: string | undefined
		onSelectTool?: (toolId: string | undefined) => void
		/** Ran after a successful deploy, with the path actually written, which a rename can move. */
		onSaved?: (path: string) => void | Promise<void>
		/** The path was minted for a new agent, so a missing row starts an empty one. */
		isNew?: boolean
		/** The deployed agent to run rather than a draft to edit: the run pane leads, and the form
		 *  beside it only shows the configuration. Fixed for the mount's lifetime. */
		view?: boolean
		/** Why this reader cannot run the agent, if they cannot. Shown in place of the run pane. */
		runBlockedReason?: string
		/** Opens the agent's configuration, offered beside the model it runs with. */
		onOpenConfig?: () => void
	}

	let {
		path,
		workspace,
		enableAi = false,
		toolId = undefined,
		onSelectTool = undefined,
		onSaved = undefined,
		isNew = false,
		view = false,
		runBlockedReason = undefined,
		onOpenConfig = undefined
	}: Props = $props()

	/** The one module the editor edits. Standalone (no `agent` key) so `initFlowState` loads a
	 *  schema per tool — the linked branch deliberately loads none. Reserved in `forbiddenIds`:
	 *  the root shares a flow-state map with the tools, so a tool of this id would lose its
	 *  schema to the root's. */
	const AGENT_ID = '__wm_agent_root'

	const draft = useAgentDraft({
		path: () => path,
		workspace: () => workspace,
		isNew: () => isNew,
		deployedOnly: () => view
	})

	/** Read access only. Everything that could write is blocked, down to the draft itself: an
	 *  autosave the server rejects would look like a save and lose the edit. Running the agent,
	 *  its history and its evals stay open, none of them being a write to the resource. */
	let readOnly = $derived(view || !draft.canWrite)

	const flowStore = $state({
		val: {
			summary: '',
			value: { modules: [] },
			extra_perms: {},
			schema: emptySchema()
		} as OpenFlow
	}) as StateStore<OpenFlow>
	const flowStateStore = $state({ val: {} }) as StateStore<FlowState>
	const previewArgs = $state({ val: {} })
	const stepsInputArgs = new StepsInputArgs()
	const modulesTestStates = new ModulesTestStates(() => {})
	const selectionManager = new SelectionManager()
	selectionManager.selectId(AGENT_ID)
	const history = initHistory(flowStore.val)
	// The agent's own path, naming a flow that exists only here. It must be a real workspace path:
	// `require_path_read_access_for_preview` rejects a first segment other than `u`/`f`/`hub` for
	// anyone who is not an admin. It is therefore not unique to this editor — a flow may carry the
	// same string — so telling the two apart reads `agentEditorHost` below, never the path.
	const pathStore = writable('')
	$effect(() => {
		pathStore.set(path)
	})

	// Drilling into a tool is navigation in this editor, not a graph selection.
	selectionManager.setOnSelectIntent?.((id: string) => {
		if (id === AGENT_ID) {
			onSelectTool?.(undefined)
			return true
		}
		if (tools.some((t) => t?.id === id)) {
			onSelectTool?.(id)
			return true
		}
		return false
	})

	// The panel-placement control belongs to a step panel, not to this editor: left alone the card
	// header would draw a placement picker and an X that closes the panel behind us.
	setContext<FlowPanelDetachContext>('flowPanelDetach', {
		claim: () => () => {},
		modalOpen: () => false,
		close: () => {},
		enabled: () => false,
		preference: () => 'auto',
		setPreference: () => {}
	})

	// Inherit the host's context where there is one (keeps "Edit the script's code" and the AI
	// chat's editor handle working), then override everything that must be private or the editor
	// would write into the host flow. Built once and mutated: Svelte snapshots the parent context
	// map per component on first access, so a later `setContext` is invisible to descendants.
	const outer = getContext<FlowEditorContext | undefined>('FlowEditorContext')
	setContext<FlowEditorContext>('FlowEditorContext', {
		scriptEditorDrawer: writable(undefined),
		workspaceScriptSettingsDrawer: writable(undefined),
		flowEditorDrawer: writable(undefined),
		initialPathStore: writable(''),
		fakeInitialPath: '',
		flowInputsStore: writable<FlowInput>({}),
		customUi: {},
		insertButtonOpen: writable(false),
		executionCount: writable(0),
		flowInputEditorState: writable<FlowInputEditorState>({
			selectedTab: undefined,
			editPanelSize: undefined,
			payloadData: undefined
		}),
		currentEditor: writable(undefined),
		outputPickerOpenFns: {},
		preserveOnBehalfOf: writable(false),
		savedOnBehalfOfEmail: writable<string | undefined>(undefined),
		savedOnBehalfOfPermissionedAs: writable<string | undefined>(undefined),
		...(outer ?? {}),
		flowStore,
		flowStateStore,
		previewArgs,
		stepsInputArgs,
		modulesTestStates,
		selectionManager,
		history,
		pathStore,
		opWorkspace: () => workspace,
		agentEditorHost: () => path,
		saveDraft: () => {}
	} as FlowEditorContext)

	// PropPickerWrapper destructures this without a guard, so it throws without it.
	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable(undefined),
		inModalPanel: () => true
	})

	// Its own, empty, rather than the flow's: the copilot addresses fields by name alone, and this
	// editor is a descendant of the flow it was opened from, so a step input it fills would also
	// land in a field of the agent that happens to share the name.
	setContext<FlowCopilotContext>('FlowCopilotContext', {
		shouldUpdatePropertyType: writable({}),
		stepInputsLoading: writable(false),
		generatedExprs: writable({}),
		exprsToSet: writable({})
	})

	let agentModule = $derived(flowStore.val.value.modules?.[0])
	let agentValue = $derived(
		agentModule?.value.type === 'aiagent' ? (agentModule.value as any) : undefined
	)
	// The resource's own array, never a copy: the tool drawer writes replacements back through
	// `tools[toolIndex]`, and a copy would take them instead of the value being edited. Only the
	// container is shape-checked — `tools` is JSON-authored, and a `.filter` on another shape
	// throws; entries are read with `?.` wherever they are dereferenced.
	let tools = $derived(Array.isArray(agentValue?.tools) ? (agentValue.tools as AgentTool[]) : [])
	let toolIndex = $derived(toolId ? tools.findIndex((t) => t?.id === toolId) : -1)
	let tool = $derived(toolIndex >= 0 ? tools[toolIndex] : undefined)

	// The caller owns which tool is open, so the drawer follows it rather than holding that state
	// itself; closing it (Escape, the X, the overlay) reports back through `on:close`.
	let toolDrawer: Drawer | undefined = $state(undefined)
	$effect(() => {
		const open = tool !== undefined
		untrack(() => (open ? toolDrawer?.openDrawer() : toolDrawer?.closeDrawer()))
	})

	// The config the synthetic module last carried, so the two directions below can tell an edit
	// apart from the echo of their own write.
	let lastArgs = $state<string | undefined>(undefined)

	// Picked on the first load, after which it is the reader's: following the memory setting live
	// would pull them out of a chat for switching memory off in the form.
	let testMode = $state<'form' | 'chat' | undefined>(undefined)
	// Mounted the first time it is shown, then only hidden: unmounting destroys the SDK chat,
	// which stops following the turn in flight, and its answer would never show.
	let chatMounted = $state(false)
	function showTestMode(mode: 'form' | 'chat') {
		testMode = mode
		if (mode === 'chat') chatMounted = true
	}

	// draft.args -> module. Only when the draft moved on its own (a load, an external write, a
	// discard); an edit made in the form arrives here as its own echo and is skipped.
	$effect(() => {
		const args = draft.state?.args
		if (draft.loading || !args) return
		const serialized = JSON.stringify(args)
		untrack(() => {
			if (serialized === lastArgs) return
			lastArgs = serialized
			const built: FlowModule = {
				id: AGENT_ID,
				value: {
					type: 'aiagent',
					tools: (args.tools ?? []) as any,
					input_transforms: agentArgsToTransforms(args) as any
				} as any
			}
			flowStore.val.value.modules = [built]
			if (testMode === undefined) showTestMode(keepsManagedMemory(args.memory) ? 'chat' : 'form')
			// Caught rather than left to float: a tool whose shape the schema loader cannot read
			// would otherwise reject into the global unhandled-rejection handler, which reports the
			// bare message and no stack — saying nothing about which agent or tool caused it. The
			// form still renders; only the inferred tool schemas are missing.
			initFlowState(flowStore.val as Flow, flowStateStore, workspace, path).catch((err) =>
				console.error('agent editor: could not infer tool schemas for', path, err)
			)
		})
	})

	/** Everything the form does not model. `inputTransformsToAgentConfig` rebuilds the value from
	 *  `AGENT_BRAIN_KEYS` alone, so a key this editor never renders — one a newer backend added, or
	 *  the `user_message` default a resource may carry, which the runtime does read when the step
	 *  supplies none — would be dropped into the draft merely by opening the agent. */
	const MODELLED_AGENT_KEYS = new Set<string>([...AGENT_BRAIN_KEYS, 'tools'])
	function unmodelledArgs(args: AIAgentConfig | undefined): Record<string, unknown> {
		return Object.fromEntries(
			Object.entries(args ?? {}).filter(([key]) => !MODELLED_AGENT_KEYS.has(key))
		)
	}

	// module -> draft.args. `inputTransformsToAgentConfig` drops the `{static, undefined}`
	// placeholders `loadFlowModuleState` backfills, so the round trip is idempotent.
	$effect(() => {
		const v = agentValue
		if (draft.loading || readOnly || !v || !draft.state) return
		const next = {
			...unmodelledArgs(draft.state.args),
			...(inputTransformsToAgentConfig(
				v.input_transforms as Record<string, InputTransform>,
				(v.tools ?? []) as AgentTool[]
			) as AIAgentConfig)
		} as AIAgentConfig
		// A config written before this editor spells "unset" its own way: an explicit `null` on a
		// field, or no `tools` key at all. `inputTransformsToAgentConfig` writes neither, and that
		// difference alone would read as an edit and autosave a draft for an agent nobody touched.
		// Only where the two spellings mean the same thing — a value the user actually cleared is
		// absent from `next` with something else in the draft, and stays cleared.
		for (const [key, value] of Object.entries(draft.state.args ?? {})) {
			if (value === null && !(key in next)) {
				;(next as Record<string, unknown>)[key] = null
			}
		}
		if (next.tools?.length === 0 && draft.state.args?.tools === undefined) {
			delete next.tools
		}
		const serialized = JSON.stringify(next)
		untrack(() => {
			if (serialized === lastArgs) return
			if (draft.state && !deepEqual(draft.state.args, next)) {
				lastArgs = serialized
				draft.state.args = next
			}
		})
	})

	let schema = $derived(flowStateStore.val[AGENT_ID]?.schema ?? {})
	const brainFilter = [...AGENT_BRAIN_KEYS, AGENT_TOOLS_ROW]

	let stepPropPicker = $derived(
		agentModule
			? getStepPropPicker(
					flowStateStore.val,
					undefined,
					undefined,
					AGENT_ID,
					flowStore.val,
					previewArgs.val,
					false
				)
			: undefined
	)

	let testJob: Job | undefined = $state(undefined)
	let testIsLoading = $state(false)
	let scriptProgress = $state(undefined)

	let chatPath = $derived(agentChatPath(path))

	/** One chat turn: the agent as edited, not as deployed, the same as a run from the form. */
	async function runChatTurn(
		userMessage: string,
		conversationId: string,
		inputs?: Record<string, any>
	): Promise<string | undefined> {
		if (!agentModule) return undefined
		return await runFlowPreview(
			{ ...(inputs ?? {}), user_message: userMessage },
			agentChatFlow($state.snapshot(agentModule) as FlowModule),
			chatPath,
			undefined,
			conversationId,
			undefined,
			workspace
		)
	}

	let runLoading = $state(false)
	let scheduledForStr = $state<string | undefined>(undefined)
	let invisible_to_owner = $state<boolean | undefined>(undefined)
	let overrideTag = $state<string | undefined>(undefined)

	/** One run from the form, as the same one-step flow a chat turn runs but outside any
	 *  conversation, then onto its run page. */
	async function runOnce(_scheduledFor: string | undefined, args: Record<string, any>) {
		if (!agentModule) return
		runLoading = true
		try {
			const flow = agentChatFlow($state.snapshot(agentModule) as FlowModule)
			flow.value.chat_input_enabled = false
			const id = await runFlowPreview(args, flow, path, undefined, undefined, undefined, workspace)
			await goto(`/run/${id}?workspace=${workspace}`)
		} finally {
			runLoading = false
		}
	}

	// What the composer reads for its model button and paperclip.
	let chatModules = $derived(agentModule ? agentChatFlow(agentModule).value.modules : undefined)
	let chatGap = $derived(agentChatGap(agentValue?.input_transforms))

	function turnOnMemory() {
		if (!agentValue) return
		agentValue.input_transforms.memory = {
			type: 'static',
			value: structuredClone(DEFAULT_AGENT_MEMORY)
		}
	}

	function turnOffMemory() {
		if (!agentValue) return
		agentValue.input_transforms.memory = { type: 'static', value: { kind: 'off' } }
	}

	function turnOnStreaming() {
		if (!agentValue) return
		agentValue.input_transforms.streaming = { type: 'static', value: true }
	}

	// Adding a tool goes straight into it: the editor has no graph to show the new node on, so the
	// tool it just created is the only place the click can land.
	async function addTool(detail: { kind: string; script?: any; flow?: any; inlineScript?: any }) {
		if (!agentValue) return
		// Adding a tool is the explicit action that replaces a JSON-authored non-list; without this
		// the insert goes into a value that cannot hold it.
		if (!Array.isArray(agentValue.tools)) agentValue.tools = []
		const id = await insertAgentTool(
			flowStore,
			flowStateStore,
			agentValue,
			detail,
			workspace,
			!enableAi
		)
		if (id) onSelectTool?.(id)
	}

	/** The mirror of `addTool`. These tools live in the resource rather than in a flow's module
	 *  list, so the graph's delete has nothing here to act on. */
	function deleteTool(id: string) {
		if (!agentValue) return
		const remaining = tools.filter((t) => t?.id !== id)
		if (remaining.length === tools.length) return
		agentValue.tools = remaining
		delete flowStateStore.val[id]
		if (toolId === id) onSelectTool?.(undefined)
	}

	/** The path field's own verdict (a taken path, an invalid name), which the server would otherwise
	 *  only report after the request. */
	let pathError = $state('')

	export function deploy(): Promise<boolean> {
		if (pathError) {
			sendUserToast(`Cannot deploy the agent: ${pathError}`, true)
			return Promise.resolve(false)
		}
		return draft.deploy().then(async (written) => {
			// The path the write landed on, which a rename moves off the one this editor opened.
			if (written) await onSaved?.(written)
			return written !== undefined
		})
	}
	export function draftHandle() {
		return draft
	}
	/** The arguments a tool takes, as this editor inferred them; undefined until it has. */
	export function toolSchema(id: string): any {
		return flowStateStore.val[id]?.schema
	}
	/** The switch between the form and the chat, which the dialog's header holds. `mode` is
	 *  undefined until the agent loads and picks the first one. */
	export function testPaneHandle() {
		return {
			get mode() {
				return testMode
			},
			set mode(mode: 'form' | 'chat' | undefined) {
				if (mode) showTestMode(mode)
			}
		}
	}
</script>

<!-- Beside the model in the chat's composer, as the AI session's assistant settings sit. -->
{#snippet configButton()}
	<Tooltip small placement="top">
		<Button
			unifiedSize="2xs"
			variant="subtle"
			iconOnly
			startIcon={{ icon: SlidersHorizontal }}
			aria-label="Agent configuration"
			onClick={() => onOpenConfig?.()}
		/>
		{#snippet text()}
			<div class="max-w-64 text-xs">
				<p class="font-semibold">Agent configuration</p>
				<p class="mt-1">The model, instructions and tools this agent runs with.</p>
			</div>
		{/snippet}
	</Tooltip>
{/snippet}

<!-- In the form's Run row, where a schedulable form keeps its Advanced options. -->
{#snippet configurationButton()}
	<Tooltip small placement="top">
		<Button
			unifiedSize="md"
			variant="default"
			iconOnly
			startIcon={{ icon: SlidersHorizontal }}
			aria-label="Agent configuration"
			onClick={() => onOpenConfig?.()}
		/>
		{#snippet text()}
			<div class="max-w-64 text-xs">
				<p class="font-semibold">Agent configuration</p>
				<p class="mt-1">The model, instructions and tools this agent runs with.</p>
			</div>
		{/snippet}
	</Tooltip>
{/snippet}

{#if draft.refusal}
	<div class="h-full flex items-center justify-center px-8">
		<Alert type="error" size="sm" title={draft.refusal} class="max-w-lg">
			Open it from the resources page to see what it holds. Any unsaved edits are kept as a draft
			there.
		</Alert>
	</div>
{:else if draft.loading}
	<div class="h-full flex items-center justify-center text-xs text-tertiary">Loading agent...</div>
{:else if agentModule && agentValue}
	<!-- Named and positioned so the tool picker's popover can portal here: the `#flow-editor` it
	     otherwise targets is behind this dialog, and does not exist at all on the resources page. -->
	<div id="agent-editor" class="relative h-full min-h-0">
		{#if view}
			<!-- A view only runs the agent: the page beside it summarizes the configuration. -->
			{@render runPane()}
		{:else}
			<!-- Resizable as the step panel's config and test are: a long system prompt and a long
			     answer want opposite splits, and only the reader knows which they are on. -->
			<Splitpanes class="h-full">
				<Pane size={55} minSize={30}>{@render configPane()}</Pane>
				<Pane size={45} minSize={20}>{@render runPane()}</Pane>
			</Splitpanes>
		{/if}
	</div>

	{#snippet configPane()}
		<div class="h-full min-h-0 overflow-auto">
			<!-- A view shows the path in its page header, and renaming is not for it anyway. -->
			{#if !view}
				<div class="px-4 pt-4">
					<Label label="Path">
						<Path
							bind:path={
								() => draft.state?.path,
								(v) => {
									if (draft.state && v !== undefined) draft.state.path = v
								}
							}
							bind:error={pathError}
							initialPath={draft.noDeployed ? '' : path}
							checkInitialPathExistence={draft.noDeployed}
							namePlaceholder="agent"
							kind="resource"
							workspaceOverride={workspace}
							autofocus={false}
							disabled={readOnly}
						/>
					</Label>
				</div>
			{/if}
			<PropPickerWrapper pickableProperties={stepPropPicker?.pickableProperties} noPadding sidePane>
				<AiAgentStepInputs
					class="px-4 pb-8"
					{schema}
					filter={brainFilter}
					previousModuleId={undefined}
					pickableProperties={stepPropPicker?.pickableProperties}
					extraLib={stepPropPicker?.extraLib ?? 'missing extraLib'}
					{enableAi}
					{workspace}
					staticOnly
					visibilityKey={`agent:${path}`}
					{tools}
					{readOnly}
					onSelectTool={(id) => onSelectTool?.(id)}
					onAddTool={readOnly ? undefined : addTool}
					onDeleteTool={readOnly ? undefined : deleteTool}
					toolPickerPortal="#agent-editor"
					bind:args={
						() => (agentValue?.input_transforms ?? {}) as Record<string, InputTransform>,
						(v) => agentValue && (agentValue.input_transforms = v)
					}
				/>
			</PropPickerWrapper>
		</div>
	{/snippet}

	{#snippet runPane()}
		{#if runBlockedReason}
			<div
				class="h-full flex flex-col items-center justify-center gap-4 px-8 text-center text-xs text-secondary"
			>
				{runBlockedReason}
				{#if onOpenConfig}
					{@render configurationButton()}
				{/if}
			</div>
		{:else}
			<div class="h-full min-h-0 flex flex-col">
				<!-- Laid out as the script editor's preview column is: what a run takes above what it
					     produced, both alongside what is being edited. -->
				{#if view}
					<!-- A run of the deployed agent is a run like any other item's, so it opens on its
					     own page rather than as a test result beside the form. -->
					<!-- Centered at the width the chat's column keeps, so switching modes moves nothing. -->
					<div class="flex-1 min-h-0 overflow-auto p-4 {testMode === 'chat' ? 'hidden' : ''}">
						<div class="max-w-3xl mx-auto">
							<RunForm
								runnable={{ schema: AGENT_CHAT_SCHEMA, path }}
								runAction={runOnce}
								schedulable={false}
								detailed={false}
								autofocus
								loading={runLoading}
								bind:scheduledForStr
								bind:invisible_to_owner
								bind:overrideTag
								actions={onOpenConfig ? configurationButton : undefined}
							/>
						</div>
					</div>
				{:else}
					<div class="flex-1 min-h-0 flex flex-col {testMode === 'chat' ? 'hidden' : ''}">
						<!-- An agent reused as a step has no use for memory unless its flow gives it a
						     conversation, and the form is where such an agent is tried. -->
						{#if !chatGap?.memory && !readOnly}
							<PaneNotice action={{ label: 'Turn off', onClick: turnOffMemory }}>
								Managed memory keeps the conversation between runs. It's useful if you chat with
								this agent, use it in a flow in chat mode, or pass it a memory id. Otherwise, you
								can turn it off.
							</PaneNotice>
						{/if}
						<Splitpanes horizontal class="flex-1 min-h-0">
							<Pane size={40} minSize={15}>
								<div class="h-full overflow-auto">
									<ModulePreview
										mod={agentModule as FlowModule}
										schema={flowLocalAgentSchema(schema)}
										pickableProperties={stepPropPicker?.pickableProperties}
										runInputKeys={AGENT_EDITOR_RUN_INPUTS}
										bind:testJob
										bind:testIsLoading
										bind:scriptProgress
									/>
								</div>
							</Pane>
							<Pane size={60} minSize={20}>
								<ModulePreviewResultViewer
									lang="deno"
									editor={undefined}
									diffEditor={undefined}
									mod={agentModule as FlowModule}
									{testJob}
									{testIsLoading}
									{scriptProgress}
									disableMock
									disableHistory
								/>
							</Pane>
						</Splitpanes>
					</div>
				{/if}
				{#if chatMounted}
					<div class={testMode === 'chat' ? 'flex flex-col flex-1 min-h-0' : 'hidden'}>
						{#if chatGap?.memory}
							<div class="flex-1 flex flex-col items-center justify-center gap-2 px-8 text-center">
								<MessageCircleOff size={48} class="text-tertiary opacity-50 mb-2" />
								<p class="text-sm font-semibold text-emphasis">Chat needs managed memory</p>
								<p class="text-xs text-secondary max-w-xs">
									{chatGap.memoryCanTurnOn
										? 'Without it, every message would be answered without the ones before it.'
										: 'This agent replays a fixed list of messages. Switch its memory to managed to chat with it.'}
								</p>
								{#if chatGap.memoryCanTurnOn && !readOnly}
									<Button
										unifiedSize="sm"
										variant="default"
										btnClasses="bg-surface mt-2"
										onClick={turnOnMemory}
									>
										Turn on managed memory
									</Button>
								{/if}
							</div>
						{:else if chatGap?.noStream}
							<PaneNotice
								action={chatGap.noStream === 'off' && !readOnly
									? { label: 'Turn on', onClick: turnOnStreaming }
									: undefined}
							>
								{chatGap.noStream === 'image'
									? 'Image answers do not stream: each one shows once its run ends.'
									: 'Streaming is off: each answer shows once its run ends.'}
							</PaneNotice>
						{/if}
						<!-- Hidden rather than unmounted while memory is off: switching it off mid-turn must
							     not end the chat following that turn. Test chats, since what runs is the agent as
							     edited. -->
						<div class={chatGap?.memory ? 'hidden' : 'flex flex-col flex-1 min-h-0'}>
							<FlowChat
								onRunFlow={runChatTurn}
								path={chatPath}
								conversationKind="test"
								subject="agent"
								frame="none"
								inputSchema={AGENT_CHAT_SCHEMA}
								flowModules={chatModules}
								composerSettings={onOpenConfig ? configButton : undefined}
							/>
						</div>
					</div>
				{/if}
			</div>
		{/if}
	{/snippet}

	<!-- A tool is a whole step editor, so it gets a surface of its own rather than a level of the
	     dialog: the agent stays visible behind it, along with the banner and Deploy that its edits
	     feed. -->
	<Drawer bind:this={toolDrawer} size="1200px" on:close={() => onSelectTool?.(undefined)}>
		<DrawerContent
			title={(tool ? toolDisplayName(tool) : undefined) ?? 'Edit tool'}
			on:close={() => toolDrawer?.closeDrawer()}
			noPadding
		>
			{#if tool}
				<!-- Inert to a read-only viewer for the same reason the form above is, and opened on its
				     configuration rather than on a test it cannot fill in. -->
				<div class="h-full min-h-0 flex flex-col {readOnly ? 'opacity-60' : ''}" inert={readOnly}>
					<AgentToolWrapper
						bind:tool={() => tools[toolIndex], (v) => (tools[toolIndex] = v)}
						parentModule={agentModule as FlowModule}
						{enableAi}
						staticOnly
						noToolNavigation
						forceTestTab={readOnly ? undefined : { [tool.id]: true }}
						siblingToolNames={tools.filter((t) => t?.id !== tool?.id).map((t) => t?.summary ?? '')}
					/>
				</div>
			{/if}
		</DrawerContent>
	</Drawer>
{/if}
