<script lang="ts">
	import { getContext, setContext, untrack } from 'svelte'
	import Modal, { type ModalTrailSegment } from '$lib/components/common/modal/Modal.svelte'
	import type { FlowEditorContext, FlowPanelDetachContext } from '../types'
	import type { FlowModule, InputTransform, Job } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { findFlowNode } from '../flowTree'
	import {
		agentEditorTarget,
		closeAgentEditor,
		showAgentEditorTool
	} from '../agentEditorStore.svelte'
	import AgentResourceBar from './AgentResourceBar.svelte'
	import AgentToolWrapper from './AgentToolWrapper.svelte'
	import AiAgentStepInputs from './AiAgentStepInputs.svelte'
	import PropPickerWrapper from '../propPicker/PropPickerWrapper.svelte'
	import ModulePreview from '$lib/components/ModulePreview.svelte'
	import ModulePreviewResultViewer from '$lib/components/ModulePreviewResultViewer.svelte'
	import { getStepPropPicker } from '../previousResults'
	import { getAgentEditingPath } from '../agentEditStore.svelte'
	import { AGENT_BRAIN_KEYS, flowLocalAgentSchema } from '../agentResourceUtils'
	import { AGENT_TOOLS_ROW } from '../agentFormFields'
	import type { AgentTool } from '../agentToolUtils'
	import { toolDisplayName } from '../agentToolUtils'

	interface Props {
		enableAi?: boolean
	}

	let { enableAi = false }: Props = $props()

	const { flowStore, flowStateStore, previewArgs, pathStore, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let opWs = $derived(opWorkspace?.() ?? $workspaceStore)

	// The panel-placement control belongs to the step panel, not to a dialog: left alone, the card
	// header inside here would draw a placement picker and an X that closes the panel behind us, and
	// its claim would be counted against the panel's own chrome.
	setContext<FlowPanelDetachContext>('flowPanelDetach', {
		claim: () => () => {},
		modalOpen: () => false,
		close: () => {},
		enabled: () => false,
		preference: () => 'auto',
		setPreference: () => {}
	})

	let target = $derived(agentEditorTarget())
	let agentNode = $derived(target ? findFlowNode(flowStore.val.value, target.agentId) : undefined)
	let agentModule = $derived(agentNode?.module)
	// The linked-tools store is keyed by ancestry for a nested agent, whose id comes from a resource
	// and is not flow-global. The bar publishes into it, so it must use the key the graph reads.
	let linkedToolsModuleId = $derived(
		agentNode?.location.type === 'aiagent'
			? `${agentNode.location.parentId}/${target?.agentId}`
			: (target?.agentId ?? '')
	)
	let agentValue = $derived(agentModule?.value.type === 'aiagent' ? agentModule.value : undefined)
	let tools = $derived((agentValue?.tools ?? []) as AgentTool[])
	let toolIndex = $derived(target?.toolId ? tools.findIndex((t) => t.id === target?.toolId) : -1)
	let tool = $derived(toolIndex >= 0 ? tools[toolIndex] : undefined)

	// The dialog edits a step forked from an agent, so it has nothing left to show once that fork
	// ends: the step deleted or replaced under it, or re-linked by Save changes or Cancel.
	$effect(() => {
		const over = target !== undefined && (agentValue === undefined || agentValue.agent !== undefined)
		if (over) untrack(() => closeAgentEditor())
	})
	// The tool the dialog was on can go the same way; drop back to the agent rather than close.
	$effect(() => {
		const missing = target?.toolId !== undefined && agentValue !== undefined && toolIndex < 0
		if (missing) untrack(() => showAgentEditorTool(undefined))
	})

	// While the step is forked for editing it carries no link, so the path being edited lives on the
	// edit session, keyed by the forked tools array.
	let editingPath = $derived(agentValue?.agent ?? getAgentEditingPath(tools))
	let title = $derived(editingPath ?? 'AI agent')

	let trail = $derived<ModalTrailSegment[]>(
		tool
			? [
					{ label: title, onclick: () => showAgentEditorTool(undefined) },
					{ label: toolDisplayName(tool) }
				]
			: [{ label: title }]
	)

	let schema = $derived(agentModule ? (flowStateStore.val[agentModule.id]?.schema ?? {}) : {})
	// Everything the resource holds, plus the tool roster; the message and attachments are supplied
	// per flow, so they stay on the step rather than on the agent.
	const brainFilter = [...AGENT_BRAIN_KEYS, AGENT_TOOLS_ROW]

	let stepPropPicker = $derived(
		agentModule
			? getStepPropPicker(
					flowStateStore.val,
					undefined,
					undefined,
					agentModule.id,
					flowStore.val,
					previewArgs.val,
					false
				)
			: undefined
	)

	let testJob: Job | undefined = $state(undefined)
	let testIsLoading = $state(false)
	let scriptProgress = $state(undefined)

	function setInputTransforms(v: Record<string, InputTransform>) {
		if (agentValue) agentValue.input_transforms = v as typeof agentValue.input_transforms
	}
</script>

{#if agentModule && agentValue}
	<Modal
		open={true}
		kind="X"
		fillHeight
		enterConfirms={false}
		{title}
		{trail}
		description="Changes here update the saved agent, and every flow that links to it."
		class="w-[92vw] sm:w-[92vw] max-w-[1500px] sm:max-w-[1500px] h-[88vh]"
		on:canceled={closeAgentEditor}
	>
		{#if tool}
			<div class="h-full min-h-0 flex flex-col">
				<AgentToolWrapper
					bind:tool={() => tools[toolIndex], (v) => (tools[toolIndex] = v)}
					parentModule={agentModule}
					{enableAi}
					forceTestTab={{ [tool.id]: true }}
					siblingToolNames={tools.filter((t) => t.id !== tool?.id).map((t) => t.summary ?? '')}
				/>
			</div>
		{:else}
			<div class="h-full min-h-0 flex flex-row">
				<div class="w-2/3 min-w-0 flex flex-col min-h-0 border-r border-light">
					<AgentResourceBar
						agentNodeId={agentModule.id}
						moduleId={linkedToolsModuleId}
						opWorkspace={opWs}
						flowPath={$pathStore}
						bind:agent={() => agentValue?.agent, (v) => agentValue && (agentValue.agent = v)}
						bind:inputTransforms={
							() => (agentValue?.input_transforms ?? {}) as Record<string, InputTransform>,
							setInputTransforms
						}
						bind:tools={
							() => (agentValue?.tools ?? []) as AgentTool[],
							(v) => agentValue && (agentValue.tools = v)
						}
						bind:toolInputs={
							() => agentValue?.tool_inputs ?? {},
							(v) =>
								agentValue && (agentValue.tool_inputs = Object.keys(v).length > 0 ? v : undefined)
						}
					/>
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
								workspace={opWs}
								visibilityKey={`agent-editor:${$pathStore}:${agentModule.id}`}
								{tools}
								onSelectTool={(toolId) => showAgentEditorTool(toolId)}
								bind:args={
									() => (agentValue?.input_transforms ?? {}) as Record<string, InputTransform>,
									setInputTransforms
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
	</Modal>
{/if}
