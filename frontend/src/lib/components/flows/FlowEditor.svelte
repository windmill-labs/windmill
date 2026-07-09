<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowEditorPanel from './content/FlowEditorPanel.svelte'
	import FlowModuleSchemaMap from './map/FlowModuleSchemaMap.svelte'
	import type { OpenInSessionSource } from '$lib/components/sessions/OpenInSessionButton.svelte'
	import WindmillIcon from '../icons/WindmillIcon.svelte'
	import { Skeleton } from '../common'
	import { getContext, onDestroy, onMount, setContext } from 'svelte'
	import type { FlowEditorContext } from './types'

	import { writable } from 'svelte/store'
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
	import Badge from '../common/badge/Badge.svelte'
	import { Maximize2, MousePointerClick, PanelRight, X } from 'lucide-svelte'
	const { flowStore, selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')
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
		/** Opt into the responsive modal panel (sessions embed). When true and the
		 *  editor mounts below MODAL_PANEL_BREAKPOINT, the right pane starts as a
		 *  modal opened by double-clicking a graph node. */
		allowModalPanel?: boolean
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
		allowModalPanel = false
	}: Props = $props()

	let flowModuleSchemaMap: FlowModuleSchemaMap | undefined = $state()

	// Below this editor width the step-details pane doesn't fit alongside the
	// graph, so `allowModalPanel` embeds start in modal mode instead.
	const MODAL_PANEL_BREAKPOINT = 1024
	let rootEl: HTMLDivElement | undefined = $state()
	// 'docked' = normal split pane; 'modal' = graph full-width, panel in a modal
	// opened by double-clicking a node. Decided once at mount from the editor's
	// own width; the user can then toggle either way (Dock right / open in modal).
	let panelMode: 'docked' | 'modal' = $state('docked')
	let panelModalOpen = $state(false)

	function openPanelModalFromGraph(e: MouseEvent) {
		if ((e.target as HTMLElement | null)?.closest('.svelte-flow__node')) {
			panelModalOpen = true
		}
	}

	// Flow-level panels (Settings, error handler, env vars, triggers, input, …) are
	// reached via toolbar buttons / dedicated nodes, not step-node double-clicks. In
	// modal mode a single click on any of them should open the modal too — step
	// modules stay double-click-only so graph select/drag doesn't pop the modal.
	const NON_MODULE_PANEL_IDS = new Set([
		'constants',
		'failure',
		'preprocessor',
		'Input',
		'Trigger',
		'Result'
	])
	function isNonModulePanelTarget(id: string): boolean {
		return id.startsWith('settings') || NON_MODULE_PANEL_IDS.has(id)
	}

	// In modal mode a step's editor is a double-click away but invisible until then —
	// keep a standing hint whenever the graph is showing (modal closed).
	const showStepHint = $derived.by(() => panelMode === 'modal' && !panelModalOpen)

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

	setContext<PropPickerContext>('PropPickerContext', {
		flowPropPickerConfig: writable<FlowPropPickerConfig | undefined>(undefined),
		pickablePropertiesFiltered: writable<PickableProperties | undefined>(undefined),
		collapsePropPickerUntilConnect: () => panelMode === 'modal'
	})

	// Read by graph step items (VirtualItem) to show a per-step "double click to
	// explore" hint on hover, since in modal mode a step's editor only opens on
	// double-click.
	setContext<() => boolean>('flowGraphStepExploreHint', () => panelMode === 'modal')

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

	onMount(() => {
		if (allowModalPanel && (rootEl?.clientWidth ?? Infinity) < MODAL_PANEL_BREAKPOINT) {
			panelMode = 'modal'
		}
		if (allowModalPanel) {
			selectionManager.setOnSelectIntent((id, opts) => {
				if (panelMode === 'modal' && (opts?.openPanel || isNonModulePanelTarget(id))) {
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
		if (allowModalPanel) {
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
	bind:this={rootEl}
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
			<Pane class="relative z-10" size={50} minSize={20}>
				{#if loading}
					<div class="w-full h-full">
						<div class="block m-auto pt-40 w-10">
							<WindmillIcon height="40px" width="40px" spin="fast" />
						</div>
					</div>
				{:else}
					{#if allowModalPanel}
						<div class="absolute top-2 right-2 z-30">
							<Button
								size="xs2"
								color="light"
								variant="border"
								iconOnly
								startIcon={{ icon: Maximize2 }}
								title="Open panel in a modal"
								on:click={() => (panelMode = 'modal')}
							/>
						</div>
					{/if}
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
			class="pointer-events-none absolute bottom-2 left-3 z-30 flex items-center gap-1.5 text-xs text-tertiary"
		>
			<MousePointerClick size={13} />
			Double click a step to explore its content
		</div>
	{/if}

	<!-- Overlay is `absolute` within this component's own `#flow-editor` box, so
	     it always covers THIS editor instance (never the whole viewport, never a
	     sibling flow editor) — anchoring is by DOM containment, not a lookup. -->
	{#if panelMode === 'modal' && panelModalOpen}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="absolute inset-0 z-40 flex" role="dialog">
			<div class="absolute inset-0 bg-black/20" onclick={() => (panelModalOpen = false)}></div>
			<div
				class="absolute inset-2 flex flex-col overflow-hidden rounded-md border bg-surface shadow-xl"
			>
				<div class="flex items-center justify-between gap-2 border-b px-2 py-1">
					<Badge
						color="indigo"
						wrapperClass="min-w-0 max-w-full"
						baseClass="!px-1"
						title={selectionManager.getSelectedId()}
					>
						<span class="max-w-full truncate text-2xs">{selectionManager.getSelectedId()}</span>
					</Badge>
					<div class="flex items-center gap-0.5">
						<Button
							size="xs2"
							variant="subtle"
							iconOnly
							startIcon={{ icon: PanelRight }}
							title="Dock to the right"
							on:click={() => {
								panelMode = 'docked'
								panelModalOpen = false
							}}
						/>
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
				<div class="min-h-0 flex-1 overflow-auto">
					{@render panelBody()}
				</div>
			</div>
		</div>
	{/if}
</div>

<svelte:window
	onkeydown={(e) => {
		if (panelMode === 'modal' && panelModalOpen && e.key === 'Escape') {
			e.stopPropagation()
			panelModalOpen = false
		}
	}}
/>
