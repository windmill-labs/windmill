<script lang="ts">
	import { getContext, setContext, untrack } from 'svelte'
	import { writable } from 'svelte/store'
	import { deepEqual } from 'fast-equals'
	import type { Flow, FlowModule, InputTransform, Job, OpenFlow } from '$lib/gen'
	import { emptySchema, type StateStore } from '$lib/utils'
	import type { FlowInput } from '$lib/components/flows/types'
	import type { FlowEditorContext, FlowInputEditorState, FlowPanelDetachContext } from '../types'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import { initFlowState, type FlowState } from '../flowState'
	import { initHistory } from '$lib/history.svelte'
	import { StepsInputArgs } from '../stepsInputArgs.svelte'
	import { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { ModulesTestStates } from '$lib/components/modulesTest.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import ModulePreviewResultViewer from '$lib/components/ModulePreviewResultViewer.svelte'
	import AiAgentStepInputs from './AiAgentStepInputs.svelte'
	import AgentToolWrapper from './AgentToolWrapper.svelte'
	import { getStepPropPicker } from '../previousResults'
	import {
		AGENT_BRAIN_KEYS,
		agentConfigToInputTransforms,
		flowLocalAgentSchema,
		inputTransformsToAgentConfig,
		type AIAgentConfig
	} from '../agentResourceUtils'
	import { AGENT_TOOLS_ROW } from '../agentFormFields'
	import type { AgentTool } from '../agentToolUtils'
	import { useAgentDraft } from '../agentDraft.svelte'

	interface Props {
		/** The `ai_agent` resource being edited. */
		path: string
		workspace: string | undefined
		enableAi?: boolean
		/** The tool drilled into, if any. Owned by the caller so it can drive the breadcrumb. */
		toolId?: string | undefined
		onSelectTool?: (toolId: string | undefined) => void
		/** Ran after a successful deploy, so a host flow can re-resolve its graph. */
		onSaved?: (path: string) => void
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
	 *  schema per tool — the linked branch deliberately loads none. */
	const AGENT_ID = 'agent'

	const draft = useAgentDraft({ path: () => path, workspace: () => workspace })

	// A path of its own, so the linked-agent tools scope, the preview job label and the form's
	// remembered open fields can never collide with a host flow's.
	let syntheticPath = $derived(`agent-editor:${path}`)

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
	const pathStore = writable('')
	$effect(() => {
		pathStore.set(syntheticPath)
	})

	// Drilling into a tool is navigation in this editor, not a graph selection.
	selectionManager.setOnSelectIntent?.((id: string) => {
		if (id === AGENT_ID) {
			onSelectTool?.(undefined)
			return true
		}
		if ((agentValue?.tools ?? []).some((t) => t.id === id)) {
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
		saveDraft: () => {}
	} as FlowEditorContext)

	// PropPickerWrapper destructures this without a guard, so it throws without it.
	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable(undefined),
		inModalPanel: () => true
	})

	let agentModule = $derived(flowStore.val.value.modules?.[0])
	let agentValue = $derived(
		agentModule?.value.type === 'aiagent' ? (agentModule.value as any) : undefined
	)
	let tools = $derived((agentValue?.tools ?? []) as AgentTool[])
	let toolIndex = $derived(toolId ? tools.findIndex((t) => t.id === toolId) : -1)
	let tool = $derived(toolIndex >= 0 ? tools[toolIndex] : undefined)

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
					input_transforms: agentConfigToInputTransforms(args) as any
				} as any
			}
			flowStore.val.value.modules = [built]
			void initFlowState(flowStore.val as Flow, flowStateStore, workspace, syntheticPath)
		})
	})

	// module -> draft.args. `inputTransformsToAgentConfig` drops the `{static, undefined}`
	// placeholders `loadFlowModuleState` backfills, so the round trip is idempotent.
	$effect(() => {
		const v = agentValue
		if (draft.loading || !v || !draft.state) return
		const next = inputTransformsToAgentConfig(
			v.input_transforms as Record<string, InputTransform>,
			(v.tools ?? []) as AgentTool[]
		) as AIAgentConfig
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

	export function deploy(): Promise<boolean> {
		return draft.deploy().then((ok) => {
			if (ok) onSaved?.(draft.state?.path ?? path)
			return ok
		})
	}
	export function draftHandle() {
		return draft
	}
	/** Re-read the resource, dropping whatever the editor holds. For a restore, which replaces the
	 *  deployed value under the editor. */
	export function reloadFromResource() {
		draft.reload()
	}
</script>

{#if draft.loading}
	<div class="h-full flex items-center justify-center text-xs text-tertiary">Loading agent...</div>
{:else if agentModule && agentValue}
	{#if tool}
		<div class="h-full min-h-0 flex flex-col">
			<AgentToolWrapper
				bind:tool={() => tools[toolIndex], (v) => (tools[toolIndex] = v)}
				parentModule={agentModule as FlowModule}
				{enableAi}
				forceTestTab={{ [tool.id]: true }}
				siblingToolNames={tools.filter((t) => t.id !== tool?.id).map((t) => t.summary ?? '')}
			/>
		</div>
	{:else}
		<div class="h-full min-h-0 flex flex-row">
			<div class="w-2/3 min-w-0 flex flex-col min-h-0 border-r border-light">
				<div class="flex-1 min-h-0 overflow-auto">
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
							onSelectTool={(id) => onSelectTool?.(id)}
							onAddTool={undefined}
							bind:args={
								() => (agentValue?.input_transforms ?? {}) as Record<string, InputTransform>,
								(v) => agentValue && (agentValue.input_transforms = v)
							}
						/>
					</PropPickerWrapper>
				</div>
			</div>
			<div class="wm-agent-lab w-1/3 min-w-0 flex flex-col min-h-0">
				<ModulePreview
					mod={agentModule as FlowModule}
					schema={flowLocalAgentSchema(schema)}
					pickableProperties={stepPropPicker?.pickableProperties}
					bind:testJob
					bind:testIsLoading
					bind:scriptProgress
				/>
				<div class="flex-1 min-h-0">
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
				</div>
			</div>
		</div>
	{/if}
{/if}
