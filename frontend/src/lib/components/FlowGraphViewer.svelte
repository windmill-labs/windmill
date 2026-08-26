<script lang="ts">
	import type { FlowModule, FlowValue, TriggersCount } from '$lib/gen'
	import type { TriggerContext } from '$lib/components/triggers'
	import { Triggers } from '$lib/components/triggers/triggers.svelte'

	import { createEventDispatcher, hasContext, setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { twMerge } from 'tailwind-merge'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { MousePointerClick } from 'lucide-svelte'
	import Modal from './common/modal/Modal.svelte'
	import Badge from './common/badge/Badge.svelte'
	import FlowPanelPlacementPicker from './flows/common/FlowPanelPlacementPicker.svelte'
	import { useFlowPanelMode } from './flows/flowPanelMode.svelte'
	import type { FlowPanelDetachContext } from './flows/types'
	import { stepLabel } from './flows/stepLabel'

	import FlowGraphViewerStep from './FlowGraphViewerStep.svelte'
	import FlowGraphV2 from './graph/FlowGraphV2.svelte'
	import { dfs } from './flows/dfs'
	import { workspaceStore } from '$lib/stores'
	import { untrack } from 'svelte'
	import { publishLinkedAgentTools } from './flows/flowState'
	import { linkedToolsScope } from './flows/linkedAgentToolsStore.svelte'

	interface Props {
		flow: {
			summary: string
			description?: string
			value: FlowValue
			schema?: any
			path?: string
		}
		overflowAuto?: boolean
		noSide?: boolean
		download?: boolean
		noGraph?: boolean
		triggerNode?: boolean
		stepDetail?: FlowModule | string | undefined
		workspace?: string | undefined
		minHeight?: number
		noBorder?: boolean
		hideDefaultInputs?: boolean
		provideTriggerContext?: boolean
		fillAvailableHeight?: boolean
	}

	let {
		flow,
		overflowAuto = false,
		noSide = false,
		download = false,
		noGraph = false,
		triggerNode = false,
		stepDetail = $bindable(undefined),
		workspace = $workspaceStore,
		minHeight = 400,
		noBorder = false,
		hideDefaultInputs = false,
		provideTriggerContext = false,
		fillAvailableHeight = false
	}: Props = $props()

	let availableHeight = $state(0)
	let availableWidth = $state(0)

	// Same placement rule as the flow editor, off the same breakpoint: 'docked' puts the step
	// panel in a pane beside the graph, 'modal' gives the graph the full width and moves the
	// panel into a dialog opened by double-clicking a step. Measured on this component rather
	// than the window, because the viewer is often embedded in a pane far narrower than it.
	const panelController = useFlowPanelMode({ enabled: () => !noSide && !noGraph })
	$effect(() => panelController.measure(availableWidth))
	let panelMode = $derived(panelController.mode)
	let stepModalOpen = $state(false)

	// Supplying this context is what puts the Auto/Attached/Detached picker in the graph's own
	// control bar — FlowGraphV2 renders it already and hides it wherever the context is absent.
	setContext<FlowPanelDetachContext>('flowPanelDetach', {
		// The viewer's panel draws no card header, so nothing claims the chrome.
		claim: () => () => {},
		modalOpen: () => panelMode === 'modal' && stepModalOpen,
		close: () => (stepModalOpen = false),
		enabled: () => !noSide && !noGraph,
		preference: () => panelController.preference,
		setPreference: (preference) => {
			if (preference === panelController.preference) return
			// Moving the panel must not lose what it was showing, so a panel that was on screen
			// reopens as a dialog. Docked is not enough on its own: under hideDefaultInputs with
			// nothing selected the pane renders nothing, and detaching would open an empty dialog.
			// The reverse is handled by the effect below, which closes a dialog no longer rendered.
			const wasVisible = (panelMode === 'docked' && hasSideContent) || stepModalOpen
			panelController.preference = preference
			stepModalOpen = panelController.mode === 'modal' && wasVisible
		}
	})
	// Whether the panel has anything to show. Kept apart from panelMode so the double-click
	// gesture stays live under hideDefaultInputs, where nothing shows until a step is picked.
	let hasSideContent = $derived(!noSide && !(hideDefaultInputs && stepDetail == undefined))
	let panelDocked = $derived(hasSideContent && panelMode === 'docked')

	// A move back to 'docked' — the viewer got wider — would otherwise leave a dialog open
	// over a panel that is already visible beside the graph.
	$effect(() => {
		if (panelMode === 'docked' && untrack(() => stepModalOpen)) {
			stepModalOpen = false
		}
	})

	// Asset and note nodes are deliberately unselectable, and the In/Out bar inside a node is
	// a picker that opens and shuts on click — neither is a request to see a step's details.
	function selectableNodeAt(e: MouseEvent): HTMLElement | null {
		const target = e.target as HTMLElement | null
		if (target?.closest('[data-prop-picker]')) return null
		return target?.closest('.svelte-flow__node.selectable') ?? null
	}

	function openStepModalFromGraph(e: MouseEvent) {
		if (selectableNodeAt(e)) stepModalOpen = true
	}

	// Clicking the step that is already selected is the second half of "select it, then show
	// it". Read in the capture phase: once the click bubbles, the graph has applied its own
	// selection and a first click looks identical to this one.
	let clickStartedOnSelected = false
	function noteSelectionBeforeClick(e: MouseEvent) {
		clickStartedOnSelected = Boolean(selectableNodeAt(e)?.classList.contains('selected'))
	}

	function openStepModalIfReselected(e: MouseEvent) {
		if (clickStartedOnSelected && selectableNodeAt(e)) stepModalOpen = true
	}

	let stepModalStep = $derived(
		typeof stepDetail === 'object' && stepDetail != undefined ? stepDetail : undefined
	)
	// Flow-level targets ('Input', 'Result', …) reach the panel as a bare string and have no id
	// or label of their own — the string is the name. With nothing selected the panel shows the
	// flow's inputs, which is what detaching from an empty selection opens on.
	let stepModalTitle = $derived(
		stepModalStep
			? stepLabel(stepModalStep)
			: typeof stepDetail === 'string'
				? stepDetail
				: 'Flow inputs'
	)
	let stepModalBadge = $derived(
		stepModalStep?.id && stepModalStep.id != 'failure' && stepModalStep.id != 'preprocessor'
			? stepModalStep.id
			: undefined
	)

	let stepHintText = $derived(
		typeof stepDetail === 'object' && stepDetail != undefined
			? 'Click the selected step to see its details'
			: 'Double click a step to see its details'
	)

	if (provideTriggerContext && !hasContext('TriggerContext')) {
		const triggersCount = writable<TriggersCount | undefined>(undefined)
		setContext<TriggerContext>('TriggerContext', {
			triggersCount,
			simplifiedPoll: writable(false),
			showCaptureHint: writable(undefined),
			triggersState: new Triggers()
		})
	}

	const dispatch = createEventDispatcher()

	// This read-only viewer doesn't run initFlowState, so linked agents' tools would otherwise never
	// resolve. Resolve them for display, keyed by module id. Best-effort: publishLinkedAgentTools
	// swallows access errors and publishes [], so an inaccessible agent simply shows no tool nodes
	// (its label still names the link) — this never affects a run.
	$effect(() => {
		// Flow modules only: resource-imported tool ids are not flow-global, so publishing a nested
		// linked agent under its bare id would supersede a top-level step that happens to share it.
		const modules = dfs(flow?.value?.modules ?? [], (m) => m, { skipToolNodes: true })
		const ws = workspace
		untrack(() => {
			for (const m of modules) {
				const value = m?.value as { type?: string; agent?: string } | undefined
				if (value?.type === 'aiagent' && value.agent) {
					publishLinkedAgentTools(value.agent, ws, linkedToolsScope(ws, flow?.path), m.id)
				}
			}
		})
	})
</script>

<div
	bind:clientHeight={availableHeight}
	bind:clientWidth={availableWidth}
	class="w-full h-full min-h-0 relative"
>
	{#if noGraph}
		{#if hasSideContent}
			{@render side()}
		{/if}
	{:else}
		<!-- The graph keeps one pane across every transition, and only the step pane comes and
		     goes. Rendering it in two branches instead would re-create FlowGraphV2 whenever the
		     panel moves — losing pan, zoom and the selected node — and every embed under 1280
		     would mount it twice, since width starts at 0 and 0 resolves to docked. -->
		<Splitpanes class="w-full h-full">
			<Pane size={panelDocked ? 66 : 100} minSize={25}>
				{@render graph()}
			</Pane>
			{#if panelDocked}
				<Pane size={34} minSize={15}>
					{@render side()}
				</Pane>
			{/if}
		</Splitpanes>
	{/if}

	{#if panelMode === 'modal' && !stepModalOpen}
		<div
			class="pointer-events-none absolute bottom-2 left-3 z-30 flex items-center gap-1.5 text-xs text-hint"
		>
			<MousePointerClick size={13} />
			{stepHintText}
		</div>
	{/if}
</div>

<Modal
	bind:open={stepModalOpen}
	title={stepModalTitle}
	kind="X"
	titleBadgeFirst
	class="sm:max-w-4xl"
>
	{#snippet titleBadge()}
		{#if stepModalBadge}
			<Badge color="indigo" small class="shrink-0 !py-0 leading-4">{stepModalBadge}</Badge>
		{/if}
	{/snippet}
	<!-- The picker in the graph's control bar is behind this dialog, so re-attaching needs its
	     own way back from in here. -->
	{#snippet settings()}
		<FlowPanelPlacementPicker variant="header" />
	{/snippet}
	<!-- The dialog supplies the padding and names the step, and hugs a short step while
	     scrolling a long one: without fillHeight it sizes to its content, which would
	     otherwise run past the viewport on a step with a long script. -->
	<FlowGraphViewerStep
		schema={flow?.schema}
		{stepDetail}
		{hideDefaultInputs}
		{workspace}
		hideHeader
		class="p-0 max-h-[70vh] overflow-y-auto"
	/>
</Modal>

{#snippet graph()}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="w-full h-full min-h-0 max-h-full"
		class:overflow-auto={overflowAuto}
		class:border={!noBorder}
		ondblclick={panelMode === 'modal' ? openStepModalFromGraph : undefined}
		onpointerdowncapture={panelMode === 'modal' ? noteSelectionBeforeClick : undefined}
		onclick={panelMode === 'modal' ? openStepModalIfReselected : undefined}
	>
		<FlowGraphV2
			{triggerNode}
			earlyStop={flow?.value?.skip_expr !== undefined}
			cache={flow?.value?.cache_ttl !== undefined}
			path={flow?.path}
			{download}
			minHeight={fillAvailableHeight ? Math.max(minHeight, availableHeight) : minHeight}
			{workspace}
			modules={flow?.value?.modules}
			failureModule={flow?.value?.failure_module}
			preprocessorModule={flow?.value?.preprocessor_module}
			notes={flow?.value?.notes}
			groups={flow?.value?.groups}
			onSelect={(nodeId) => {
				if (nodeId === 'Trigger') {
					dispatch('triggerDetail')
					return
				} else if (nodeId === 'failure') {
					stepDetail = flow?.value?.failure_module
				} else if (nodeId === 'preprocessor') {
					stepDetail = flow?.value?.preprocessor_module
				} else {
					stepDetail = dfs(flow?.value?.modules ?? [], (m) => m).find((m) => m?.id === nodeId)
				}
				stepDetail = stepDetail ?? nodeId
				dispatch('select', stepDetail)
			}}
		/>
	</div>
{/snippet}

{#snippet side()}
	<div
		class={twMerge(
			fillAvailableHeight
				? 'relative w-full h-full min-h-0 border-r border-b border-t p-2 pt-0 overflow-auto flex flex-col gap-4'
				: 'relative w-full h-full min-h-[150px] max-h-[90vh] border-r border-b border-t p-2 pt-0 overflow-auto flex flex-col gap-4',
			noGraph ? 'border-0 w-max' : ''
		)}
	>
		<FlowGraphViewerStep schema={flow?.schema} {stepDetail} {hideDefaultInputs} {workspace} />
	</div>
{/snippet}
