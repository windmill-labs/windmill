<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Disposable from '$lib/components/common/drawer/Disposable.svelte'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import { agentEditorTarget, type AgentEditorTarget } from './agentEditorStore.svelte'
	import AgentEditorModal from './content/AgentEditorModal.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import type { OpenInSessionSource } from '$lib/components/sessions/OpenInSessionButton.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, onDestroy, onMount, setContext, untrack } from 'svelte'
	import type { FlowEditorContext, FlowPanelDetachContext } from './types'
	import { getOverlayHost } from '$lib/components/common/overlayHost.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { isFlowLevelPanelTarget } from '$lib/components/graph/selectionUtils.svelte'
	import { useFlowPanelMode } from './flowPanelMode.svelte'
	import { useFlowPanelPlacementTelemetry } from './flowEditorTelemetry'

	import { get, writable } from 'svelte/store'
	import type { PropPickerContext, FlowPropPickerConfig } from '$lib/components/prop_picker'
	import type { PickableProperties } from '$lib/components/flows/previousResults'
	import type { Flow, Job } from '$lib/gen'
	import type { Trigger } from '$lib/components/triggers/utils'
	import FlowAIChat from '../copilot/chat/flow/FlowAIChat.svelte'
	import {
		AIChatManager,
		aiChatManager as singletonAiChatManager,
		AIMode
	} from '../copilot/chat/AIChatManager.svelte'
	import type { GraphModuleState } from '../graph'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import type { ModulesTestStates } from '../modulesTest.svelte'
	import type { StateStore } from '$lib/utils'
	import type { FlowOptions } from '../copilot/chat/ContextManager.svelte'
	import { extractAllModules } from '../copilot/chat/shared'
	import type { Snippet } from 'svelte'
	import { Button } from '../common'
	import { MousePointerClick, X } from 'lucide-svelte'
	import FlowPanelPlacementPicker from './common/FlowPanelPlacementPicker.svelte'
	import { prefersSessionHandoff } from '../copilot/chat/global/gate'
	import { openSourceInSession } from '$lib/components/sessions/sessionSwitch.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	const { flowStore, selectionManager, pathStore, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')
	// Flow paths repeat across workspaces, and a session keeps every tab it has visited alive, so two
	// editors can hold the same path at once. Both halves are needed to tell them apart.
	let editorWorkspace = $derived(opWorkspace?.() ?? $workspaceStore)
	function targetWorkspace(t: AgentEditorTarget): string | undefined {
		return t.workspace ?? $workspaceStore
	}

	const sessionScopedManager = getContext<AIChatManager>('aiChatManager')
	const aiChatManager = sessionScopedManager ?? singletonAiChatManager

	interface Props {
		loading: boolean
		disableStaticInputs?: boolean
		disableTutorials?: boolean
		disableAi?: boolean
		disableSettings?: boolean
		disabledFlowInputs?: boolean
		smallErrorHandler?: boolean
		newFlow?: boolean
		showJobStatus?: boolean
		savedFlow?:
			| (Flow & {
					draft?: Flow | undefined
			  })
			| undefined
		onDeployTrigger?: (trigger: Trigger) => void
		onTestUpTo?: ((id: string) => void) | undefined
		onEditInput?: ((moduleId: string, key: string) => void) | undefined
		forceTestTab?: Record<string, boolean>
		highlightArg?: Record<string, string | undefined>
		aiChatOpen?: boolean
		showFlowAiButton?: boolean
		toggleAiChat?: () => void
		sessionOpen?: OpenInSessionSource
		localModuleStates?: Record<string, GraphModuleState>
		testModuleStates?: ModulesTestStates
		isOwner?: boolean
		onTestFlow?: (conversationId?: string) => Promise<string | undefined>
		isRunning?: boolean
		onCancelTestFlow?: () => void
		onOpenPreview?: () => void
		onHideJobStatus?: () => void
		individualStepTests?: boolean
		job?: Job
		suspendStatus?: StateStore<Record<string, { job: Job; nb: number }>>
		onDelete?: (id: string) => void
		flowHasChanged?: boolean
		previewOpen: boolean
		graphOverlay?: Snippet
		/** Allow the step-details pane to open as a modal. Whitelabel embeds turn this off
		 *  to keep the classic always-docked pane. */
		modalPanel?: boolean
	}

	let {
		loading,
		disableStaticInputs = false,
		disableTutorials = false,
		disableAi = false,
		disableSettings = false,
		disabledFlowInputs = false,
		smallErrorHandler = false,
		showJobStatus = false,
		newFlow = false,
		savedFlow = undefined,
		onDeployTrigger = () => {},
		onTestUpTo = undefined,
		onEditInput = undefined,
		forceTestTab,
		highlightArg,
		localModuleStates = {},
		testModuleStates = undefined,
		aiChatOpen,
		showFlowAiButton,
		toggleAiChat,
		sessionOpen,
		isOwner,
		onTestFlow,
		isRunning,
		onCancelTestFlow,
		onOpenPreview,
		onHideJobStatus,
		individualStepTests = false,
		job,
		suspendStatus,
		onDelete,
		flowHasChanged,
		previewOpen,
		graphOverlay,
		modalPanel = true
	}: Props = $props()

	let flowModuleSchemaMap: FlowModuleSchemaMap | undefined = $state()

	// 'docked' = normal split pane; 'modal' = graph full-width, panel in a modal opened by
	// double-clicking a node. The controller resolves it from the user's Auto/Attached/
	// Detached preference and the width measured below.
	const panelController = useFlowPanelMode({ enabled: () => modalPanel })
	const panelMode = $derived(panelController.mode)
	let panelModalOpen = $state(false)

	// Owned by FlowBuilder: this component is inside a `{#key}` that rebuilds it on a reload,
	// and the crossing count belongs to the editing session rather than to one mount.
	const placementTelemetry = useFlowPanelPlacementTelemetry()
	$effect(() => {
		placementTelemetry.observe(panelController.preference, panelMode, panelController.measured)
	})

	// Auto can move the panel back into the pane under a modal that is open — leaving it
	// open would keep an overlay registered for a modal nothing renders, swallowing Escape.
	$effect(() => {
		if (panelMode === 'docked' && untrack(() => panelModalOpen)) {
			panelModalOpen = false
		}
	})

	let panelDisposable: Disposable | undefined = $state(undefined)
	// Disposable joins the stack through its methods, not by watching `open` — same sync
	// as Drawer and Modal, so setting `panelModalOpen` anywhere still registers the overlay.
	$effect(() => {
		panelModalOpen
		untrack(() => {
			panelModalOpen ? panelDisposable?.openDrawer() : panelDisposable?.closeDrawer()
		})
	})

	const overlayHost = getOverlayHost()
	const modalHost = $derived(overlayHost?.el())

	// Only nodes that can take the selection, or the modal would open on whatever was
	// selected before — asset and note nodes are deliberately unselectable.
	//
	// The In/Out bar sits inside the node but is a picker of its own: it toggles open and
	// shut on click, so opening then closing it is a double-click the graph must not read
	// as "show me this step".
	function selectableNodeAt(e: MouseEvent): HTMLElement | null {
		const target = e.target as HTMLElement | null
		if (target?.closest('[data-prop-picker]')) return null
		return target?.closest('.svelte-flow__node.selectable') ?? null
	}

	function openPanelModalFromGraph(e: MouseEvent) {
		if (selectableNodeAt(e)) {
			panelModalOpen = true
		}
	}

	// A click on the step that is already selected is the second half of "select it, then
	// show it". Read in the capture phase: by the time the click bubbles here the graph has
	// applied its own selection, so a first click would look indistinguishable from this.
	let clickStartedOnSelected = false
	function noteSelectionBeforeClick(e: MouseEvent) {
		const node = selectableNodeAt(e)
		clickStartedOnSelected =
			Boolean(node?.classList.contains('selected')) && selectionManager.selectedIds.length === 1
	}

	function openPanelModalIfReselected(e: MouseEvent) {
		if (clickStartedOnSelected && selectableNodeAt(e)) {
			panelModalOpen = true
		}
	}

	// In modal mode a step's editor is a click or two away but invisible until then —
	// keep a standing hint whenever the graph is showing (modal closed).
	const showStepHint = $derived.by(() => panelMode === 'modal' && !panelModalOpen)
	const stepHintText = $derived.by(() => {
		const ids = selectionManager.selectedIds
		return ids.length === 1 && !isFlowLevelPanelTarget(ids[0])
			? 'Click the selected step to explore its content'
			: 'Double click a step to explore its content'
	})

	// When the graph pane is narrow, fall back to a top-centered overlay so the
	// preview buttons don't overlap the rightmost node ports (matches the dev
	// page layout).
	let graphPaneWidth = $state(0)
	const compactGraphOverlay = $derived(graphPaneWidth > 0 && graphPaneWidth < 800)

	export function isNodeVisible(nodeId: string): boolean {
		return flowModuleSchemaMap?.isNodeVisible(nodeId) ?? false
	}

	export function enableNotes(): void {
		flowModuleSchemaMap?.enableNotes?.()
	}

	const flowPropPickerConfig = writable<FlowPropPickerConfig | undefined>(undefined)
	// Closing the modal unmounts the panel that started a graph connect, but the config
	// outlives it — a later pick would run a closure over a step nobody is editing.
	$effect(() => {
		if (!panelModalOpen) {
			flowPropPickerConfig.set(undefined)
		}
	})

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig,
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined),
		// The agent editor is a dialog over the same graph, so a connect started inside it has the
		// same closure hazard as one started from the modal panel. Only this flow's own, on the same
		// rule the mount below claims one by: a session keeps every visited tab alive, and a target
		// belonging to another of them is not a dialog over this graph.
		inModalPanel: () => {
			if (panelMode === 'modal') return true
			const t = agentEditorTarget()
			return (
				t !== undefined &&
				t.host?.flowPath === get(pathStore) &&
				targetWorkspace(t) === editorWorkspace
			)
		}
	})

	// Read by graph step items (VirtualItem) to show a per-step "explore" hint on hover,
	// since in modal mode a step's editor is hidden until a double-click, or a click on the
	// step that is already selected.
	setContext<() => boolean>('flowGraphStepExploreHint', () => panelMode === 'modal')

	// The panel's chrome lives inline in its card header (no dedicated row); panels without
	// a card header get FlowEditor's fallback strip instead, driven by the claim count.
	let detachClaims = $state(0)
	setContext<FlowPanelDetachContext>('flowPanelDetach', {
		claim: () => {
			detachClaims++
			return () => detachClaims--
		},
		modalOpen: () => modalPanel && panelMode === 'modal' && panelModalOpen,
		close: () => (panelModalOpen = false),
		enabled: () => modalPanel,
		preference: () => panelController.preference,
		setPreference: (preference) => {
			// Picking the row that is already active is not a move, and counting it would
			// report a placement being forced that the panel was already in.
			if (preference === panelController.preference) return
			// Moving the panel must not lose what it was showing: docked, it is always on
			// screen, so the modal it becomes has to open on arrival. The reverse is handled
			// by the effect above, which closes a modal that is no longer rendered.
			const wasVisible = panelMode === 'docked' || panelModalOpen
			placementTelemetry.forced(preference, panelMode)
			panelController.preference = preference
			panelModalOpen = panelController.mode === 'modal' && wasVisible
		}
	})

	$effect(() => {
		const options: FlowOptions = {
			currentFlow: flowStore.val,
			lastDeployedFlow: savedFlow,
			lastSavedFlow: savedFlow?.draft,
			path: savedFlow?.path,
			modules: extractAllModules(flowStore.val.value.modules)
		}
		aiChatManager.flowOptions = options
	})

	// The step exists but is empty, so name it: a GLOBAL-mode request carries no
	// implicit "current step" the way the old SCRIPT-mode generateStep did.
	function stepInstructionsPrompt(moduleId: string, instructions: string): string {
		return `Write the code for step \`${moduleId}\` of the flow open in the editor:\n\n${instructions}`
	}

	onMount(() => {
		if (modalPanel) {
			selectionManager.setOnSelectIntent((id, opts) => {
				if (opts?.openPanel === false) return
				if (panelMode === 'modal' && (opts?.openPanel || isFlowLevelPanelTarget(id))) {
					panelModalOpen = true
				}
			})
		}
		if (!sessionScopedManager) {
			aiChatManager.saveAndClear()
			aiChatManager.changeMode(AIMode.FLOW)
		}
	})

	onDestroy(() => {
		aiChatManager.flowOptions = undefined
		if (modalPanel) {
			selectionManager.setOnSelectIntent(undefined)
		}
		if (!sessionScopedManager) {
			aiChatManager.saveAndClear()
			aiChatManager.changeMode(AIMode.NAVIGATOR)
		}
	})
</script>

{#snippet panelBody()}
	<FlowEditorPanel
		{disabledFlowInputs}
		{newFlow}
		{savedFlow}
		enableAi={!disableAi}
		on:applyArgs
		on:testWithArgs
		{onDeployTrigger}
		{forceTestTab}
		{highlightArg}
		{onTestFlow}
		{job}
		{isOwner}
		{suspendStatus}
		onOpenDetails={onOpenPreview}
		{previewOpen}
		{flowModuleSchemaMap}
	/>
{/snippet}

<div
	bind:clientWidth={null, (w) => panelController.measure(w)}
	id="flow-editor"
	class={'relative h-full overflow-hidden transition-colors duration-[400ms] ease-linear border-t'}
	use:triggerableByAI={{
		id: 'flow-editor',
		description: 'Component to edit a flow'
	}}
>
	<Splitpanes>
		<Pane size={panelMode === 'docked' ? 50 : 100} minSize={15} class="h-full relative z-0">
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				bind:clientWidth={graphPaneWidth}
				ondblclick={panelMode === 'modal' ? openPanelModalFromGraph : undefined}
				onpointerdowncapture={panelMode === 'modal' ? noteSelectionBeforeClick : undefined}
				onclick={panelMode === 'modal' ? openPanelModalIfReselected : undefined}
				class="grow overflow-hidden bg-gray h-full bg-surface-secondary relative"
			>
				{#if graphOverlay}
					<div
						class="absolute z-30 flex gap-2 {compactGraphOverlay
							? 'top-14 left-1/2 -translate-x-1/2'
							: 'top-2 right-2'}"
					>
						{@render graphOverlay()}
					</div>
				{/if}
				{#if loading}
					<div class="p-2 pt-10">
						{#each new Array(6) as _}
							<Skeleton layout={[[2], 1.5]} />
						{/each}
					</div>
				{:else if flowStore.val.value.modules}
					<FlowModuleSchemaMap
						bind:this={flowModuleSchemaMap}
						controlsPosition={compactGraphOverlay ? 'bottom' : 'top'}
						{disableStaticInputs}
						{disableTutorials}
						{disableAi}
						{disableSettings}
						{smallErrorHandler}
						{newFlow}
						{showJobStatus}
						on:reload
						on:generateStep={({ detail }) => {
							// The step is already inserted; the prompt describes what it should
							// contain. Hand it to a session opened on that step rather than the
							// docked chat, which sessions leave unmounted. Sent on arrival: the
							// user already said what they wanted in the description field.
							if (
								!sessionScopedManager &&
								sessionOpen &&
								prefersSessionHandoff($userStore?.operator)
							) {
								void openSourceInSession(sessionOpen, {
									previewParams: { selected: detail.moduleId },
									seedPrompt: stepInstructionsPrompt(detail.moduleId, detail.instructions),
									autoSend: true
								})
								return
							}
							// Already in a session: its chat is on screen, so ask it directly.
							// Not `generateStep` — that forces the request into SCRIPT mode, and
							// changeMode is persistent, so it would strand the session outside
							// GLOBAL. Global mode writes step code through set_flow_module_code.
							if (sessionScopedManager) {
								sessionScopedManager.sendOrQueue(
									stepInstructionsPrompt(detail.moduleId, detail.instructions)
								)
								return
							}
							if (!aiChatManager.open) {
								aiChatManager.openChat()
							}
							aiChatManager.generateStep(detail.moduleId, detail.lang, detail.instructions)
						}}
						{onTestUpTo}
						{onEditInput}
						{localModuleStates}
						{testModuleStates}
						{aiChatOpen}
						{showFlowAiButton}
						{toggleAiChat}
						{sessionOpen}
						{isOwner}
						{onTestFlow}
						{isRunning}
						{onCancelTestFlow}
						{onOpenPreview}
						{onHideJobStatus}
						{individualStepTests}
						flowJob={job}
						{suspendStatus}
						{onDelete}
						{flowHasChanged}
					/>
				{/if}
			</div>
		</Pane>
		{#if panelMode === 'docked'}
			<!-- Panels manage their own scrolling, so the pane must not scroll as well or a second
			     scrollbar appears beside theirs. `!` because splitpanes' own `overflow: auto` rule
			     has equal specificity and wins on cascade order. -->
			<Pane class="relative z-10 !overflow-hidden" size={50} minSize={20}>
				{#if loading}
					<div class="w-full h-full">
						<div class="block m-auto pt-40 w-10">
							<WindmillIcon height="40px" width="40px" spin="fast" />
						</div>
					</div>
				{:else if modalPanel}
					<div class="flex h-full flex-col">
						<!-- Fallback for panels without a card header hosting the placement
						     picker: a slim strip so moving the panel stays reachable. Toggled
						     around a stable panelBody — re-parenting it would re-mount
						     the claiming header and loop. -->
						{#if detachClaims === 0}
							<div class="flex items-center justify-end border-b px-1">
								<FlowPanelPlacementPicker variant="header" />
							</div>
						{/if}
						<div class="min-h-0 flex-1">
							{@render panelBody()}
						</div>
					</div>
				{:else}
					{@render panelBody()}
				{/if}
			</Pane>
		{/if}
		{#if !disableAi}
			<FlowAIChat {flowModuleSchemaMap} {onTestFlow} />
		{/if}
	</Splitpanes>

	{#if showStepHint}
		<div
			class="pointer-events-none absolute bottom-2 left-3 z-30 flex items-center gap-1.5 text-xs text-hint"
		>
			<MousePointerClick size={13} />
			{stepHintText}
		</div>
	{/if}
</div>

<!-- Portalled out of `#flow-editor` so the modal covers the chrome around the editor
     (sidebar, top bar) rather than only the editor's own box. A host that embeds the
     editor in its own box provides an anchor element instead, keeping the modal inside
     it — one flow editor's modal must never cover a sibling's tab. -->
<!-- Disposable owns the overlay stack: it takes a place while open, arbitrates Escape against
     whatever else is open in this pane, and stays quiet while the pane is hidden. -->
<Disposable bind:open={panelModalOpen} bind:this={panelDisposable}>
	{#snippet children({ zIndex })}
		{#if panelMode === 'modal' && panelModalOpen}
			<Portal target={modalHost ?? 'body'} class="contents">
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="{modalHost ? 'absolute' : 'fixed'} inset-0 flex justify-center px-2 py-6"
					style="z-index: {zIndex}"
					role="dialog"
				>
					<div class="absolute inset-0 bg-black/20" onclick={() => (panelModalOpen = false)}></div>
					<div
						class="relative flex w-full max-w-4xl flex-col overflow-hidden rounded-md border bg-surface shadow-xl"
					>
						<!-- Same fallback as the docked strip: a panel whose body has a card header
						     hosts the id, placement and close inline, so this bar would double it. -->
						{#if detachClaims === 0}
							<div class="flex items-center justify-end gap-2 border-b px-2 py-1">
								<div class="flex items-center gap-0.5">
									<FlowPanelPlacementPicker variant="header" />
									<Button
										size="xs2"
										variant="subtle"
										iconOnly
										startIcon={{ icon: X }}
										title="Close"
										on:click={() => (panelModalOpen = false)}
									/>
								</div>
							</div>
						{/if}
						<div class="min-h-0 flex-1 overflow-auto">
							{@render panelBody()}
						</div>
					</div>
				</div>
			</Portal>
		{/if}
	{/snippet}
</Disposable>

<!-- Mounted here rather than in the panel, which is keyed on the selection and would take the
     dialog down with it the moment the graph selection moved. Claims only agents opened from this
     flow: a session keeps every visited tab alive, and each would otherwise build its own editor
     over the same draft. -->
<AgentEditorModal
	enableAi={!disableAi}
	owns={(t) => t.host?.flowPath === $pathStore && targetWorkspace(t) === editorWorkspace}
/>
