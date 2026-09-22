<script lang="ts">
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import type { Flow, FlowModule, Job, OpenFlow } from '$lib/gen'
	import { emptySchema, type StateStore } from '$lib/utils'
	import type { FlowInput } from '$lib/components/flows/types'
	import type { FlowEditorContext, FlowInputEditorState, FlowPanelDetachContext } from '../types'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { initFlowState, type FlowState } from '../flowState'
	import { initHistory } from '$lib/history.svelte'
	import { StepsInputArgs } from '../stepsInputArgs.svelte'
	import { SelectionManager } from '$lib/components/graph/selectionUtils.svelte'
	import { ModulesTestStates } from '$lib/components/modulesTest.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import ModulePreviewResultViewer from '$lib/components/ModulePreviewResultViewer.svelte'
	import { flowLocalAgentSchema, type AIAgentConfig } from '../agentResourceUtils'
	import { agentArgsToTransforms } from '../linkedAgentDrafts'
	import { AGENT_EDITOR_RUN_INPUTS } from '../agentFormFields'
	import type { AgentTool } from '../agentToolUtils'

	/**
	 * Runs one agent configuration as a step test, the way the agent editor's test pane does, with
	 * nothing to edit around it: the config is whatever the caller hands over, deployed or not.
	 * The module test machinery reads a flow editor's context, so this builds a private one around
	 * a flow holding that single step.
	 */
	interface Props {
		/** The path the preview runs under. Must be a real workspace path (`u/`, `f/`). */
		path: string
		workspace: string | undefined
		config: AIAgentConfig
	}

	let { path, workspace, config }: Props = $props()

	const AGENT_ID = '__wm_agent_root'

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
	const pathStore = writable(path)

	setContext<FlowPanelDetachContext>('flowPanelDetach', {
		claim: () => () => {},
		modalOpen: () => false,
		close: () => {},
		enabled: () => false,
		preference: () => 'auto',
		setPreference: () => {}
	})

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

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable(undefined),
		inModalPanel: () => true
	})

	setContext<FlowCopilotContext>('FlowCopilotContext', {
		shouldUpdatePropertyType: writable({}),
		stepInputsLoading: writable(false),
		generatedExprs: writable({}),
		exprsToSet: writable({})
	})

	// The config as the one module of the private flow, built once: the caller keys this pane on
	// what it hands over, so another config is another pane rather than a rebuild.
	flowStore.val.value.modules = [
		{
			id: AGENT_ID,
			value: {
				type: 'aiagent',
				tools: (Array.isArray(config.tools) ? config.tools : []) as AgentTool[] as any,
				input_transforms: agentArgsToTransforms(config) as any
			} as any
		} satisfies FlowModule
	]
	initFlowState(flowStore.val as Flow, flowStateStore, workspace, path).catch((err) =>
		console.error('agent run pane: could not infer tool schemas for', path, err)
	)

	let agentModule = $derived(flowStore.val.value.modules?.[0])
	let schema = $derived(flowStateStore.val[AGENT_ID]?.schema ?? {})

	let testJob: Job | undefined = $state(undefined)
	let testIsLoading = $state(false)
	let scriptProgress = $state(undefined)
</script>

{#if agentModule}
	<!-- A column rather than a split: the inputs take the height they need, capped so the result
	     always has room, and the result gets the rest. -->
	<div class="h-full min-h-0 flex flex-col">
		<div class="shrink-0 max-h-[45%] overflow-auto border-b">
			<ModulePreview
				mod={agentModule as FlowModule}
				schema={flowLocalAgentSchema(schema)}
				pickableProperties={undefined}
				runInputKeys={AGENT_EDITOR_RUN_INPUTS}
				bind:testJob
				bind:testIsLoading
				bind:scriptProgress
			/>
		</div>
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
{/if}
