<script lang="ts">
	import { getContext, setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import { Alert } from '$lib/components/common'
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
	import { AGENT_TOOLS_ROW } from '../agentFormFields'
	import { toolDisplayName, type AgentTool } from '../agentToolUtils'
	import { useAgentDraft } from '../agentDraft.svelte'

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
	}

	let {
		path,
		workspace,
		enableAi = false,
		toolId = undefined,
		onSelectTool = undefined,
		onSaved = undefined
	}: Props = $props()

	/** The one module the editor edits. Standalone (no `agent` key) so `initFlowState` loads a
	 *  schema per tool — the linked branch deliberately loads none. Reserved in `forbiddenIds`:
	 *  the root shares a flow-state map with the tools, so a tool of this id would lose its
	 *  schema to the root's. */
	const AGENT_ID = '__wm_agent_root'

	const draft = useAgentDraft({ path: () => path, workspace: () => workspace })

	/** Read access only. Everything that could write is blocked, down to the draft itself: an
	 *  autosave the server rejects would look like a save and lose the edit. Running the agent,
	 *  its history and its evals stay open, none of them being a write to the resource. */
	let readOnly = $derived(!draft.canWrite)

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
			// Caught rather than left to float: a tool whose shape the schema loader cannot read
			// would otherwise reject into the global unhandled-rejection handler, which reports the
			// bare message and no stack — saying nothing about which agent or tool caused it. The
			// form still renders; only the inferred tool schemas are missing.
			initFlowState(flowStore.val as Flow, flowStateStore, workspace, path).catch((err) =>
				console.error('agent editor: could not infer tool schemas for', path, err)
			)
		})
	})

	/** Every argument the resource carries, as a static transform. `tools` is the roster rather than
	 *  a field, so it rides on the module's own key instead. Not only the keys the form renders: a
	 *  run reads them all, and an agent holding its own `user_message` answers with it when nothing
	 *  overrides it, so a test here has to run the configuration a linked step would. */
	function agentArgsToTransforms(args: AIAgentConfig): Record<string, InputTransform> {
		const it: Record<string, InputTransform> = {}
		for (const [key, value] of Object.entries(args ?? {})) {
			if (key === 'tools' || value === undefined) continue
			it[key] = { type: 'static', value } as InputTransform
		}
		return it
	}

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

	export function deploy(): Promise<boolean> {
		return draft.deploy().then(async (ok) => {
			// The path this editor opened, not the draft's live one: `deploy` refuses a renaming draft,
			// so the write always lands here, while the shared draft can be repointed by another tab
			// mid-request and would send the reconciliation after a resource nobody wrote.
			if (ok) await onSaved?.(path)
			return ok
		})
	}
	export function draftHandle() {
		return draft
	}
</script>

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
		<!-- Resizable as the step panel's config and test are: a long system prompt and a long
		     answer want opposite splits, and only the reader knows which they are on. -->
		<Splitpanes class="h-full">
			<Pane size={66} minSize={30}>
				<div class="h-full min-h-0 overflow-auto">
					<PropPickerWrapper
						pickableProperties={stepPropPicker?.pickableProperties}
						noPadding
						sidePane
					>
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
			</Pane>
			<!-- Laid out as the script editor's preview column is: what a run takes above what it
			     produced, both alongside what is being edited. -->
			<Pane size={34} minSize={20}>
				<Splitpanes horizontal class="h-full">
					<Pane size={40} minSize={15}>
						<div class="h-full overflow-auto">
							<ModulePreview
								mod={agentModule as FlowModule}
								schema={flowLocalAgentSchema(schema)}
								pickableProperties={stepPropPicker?.pickableProperties}
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
			</Pane>
		</Splitpanes>
	</div>

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
