<script lang="ts">
	import type { Snippet } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import HideButton from '$lib/components/apps/editor/settingsPanel/HideButton.svelte'
	import AssetGraphCanvas from './AssetGraphCanvas.svelte'
	import AssetGraphDetailsPane from './AssetGraphDetailsPane.svelte'
	import PipelineEventLog from './PipelineEventLog.svelte'
	import GlobalReviewButtons from '$lib/components/copilot/chat/GlobalReviewButtons.svelte'
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind,
		PipelineMode
	} from './types'
	import type { AssetKind, ScriptLang } from '$lib/gen'
	import type { RunnableRunState, PipelineEvent } from './activeRunnables.svelte'
	import type { PipelineOutputKind } from './pipelineTemplates'
	import type { PipelineEditorState } from './pipelineEditorState.svelte'

	type RunProducer = { kind: 'script' | 'flow'; path: string; unsaved?: boolean; cascade?: boolean }

	let {
		editor,
		displayGraph,
		mode,
		workspace,
		pathPrefix,
		defaultPathSuffix = 'new_pipeline_script',
		panelHidden = false,
		onTogglePanelHidden,
		prefetchingAssets = false,
		hasAiPending = false,
		onAcceptAllProposals,
		onRejectAllProposals,
		hoveredPaths = [],
		selectedRunPaths = [],
		activeRunnable,
		activeRunnableIds,
		runStates,
		eventLogEvents = [],
		runsRefreshKey = 0,
		runsPendingJobId,
		boundPick,
		validStartPaths,
		onStartBoundedRun,
		onPickEnd,
		panToNodeId,
		boundBar,
		onCreateMissingTrigger,
		onEditTrigger,
		onDeleteTrigger,
		onOpenWebhook,
		onOpenDataUpload,
		onSelect,
		onAddScriptForAsset,
		onAddPipelineScript,
		onRunnableMenuRemove,
		onRunProducer,
		idlePane,
		onRequestEdit,
		canRunByPath = false,
		onRunByPath,
		selectionProducers = [],
		downstreamSubscribers = 0,
		onStartBoundedRunForOpen,
		canBoundedRunOpenScript = false,
		onRunCompleted,
		onTestStateChange,
		requestRemoveSignal = 0,
		requestRunSignal = 0,
		requestRunCascadeSignal = 0,
		focusUploadSignal = 0,
		onDraftPathChange,
		onClose = () => {},
		onDiscard,
		onDraftSaved,
		onPersistedSaved,
		onScriptRenamed,
		onScriptRemoved
	}: {
		editor: PipelineEditorState
		displayGraph: AssetGraphResponse
		mode: PipelineMode
		workspace: string | undefined
		pathPrefix: string
		defaultPathSuffix?: string
		panelHidden?: boolean
		onTogglePanelHidden?: () => void
		prefetchingAssets?: boolean
		hasAiPending?: boolean
		onAcceptAllProposals?: () => void
		onRejectAllProposals?: () => void
		hoveredPaths?: string[]
		selectedRunPaths?: string[]
		activeRunnable?: { kind: 'script' | 'flow'; path: string } | undefined
		activeRunnableIds?: ReadonlySet<string>
		runStates?: ReadonlyMap<string, RunnableRunState>
		eventLogEvents?: PipelineEvent[]
		runsRefreshKey?: any
		runsPendingJobId?: string | undefined
		boundPick?: any
		validStartPaths?: Set<string>
		onStartBoundedRun?: (path: string) => void
		onPickEnd?: (id: string) => void
		panToNodeId?: string | undefined
		boundBar?: Snippet
		onCreateMissingTrigger?: (kind: NativeTriggerKind, scriptPath: string) => void
		onEditTrigger?: (...args: any[]) => void
		onDeleteTrigger?: (...args: any[]) => void
		onOpenWebhook?: (...args: any[]) => void
		onOpenDataUpload?: (...args: any[]) => void
		onSelect: (selection: AssetGraphSelection | undefined) => void
		onAddScriptForAsset?: (
			asset: { kind: AssetKind; path: string },
			language: ScriptLang,
			scriptPath: string,
			outputKind: PipelineOutputKind,
			aiPrompt?: string
		) => void
		onAddPipelineScript?: (
			language: ScriptLang,
			path: string,
			source: { kind: NativeTriggerKind; path: string | undefined },
			outputKind: PipelineOutputKind,
			aiPrompt?: string
		) => void
		onRunnableMenuRemove?: (...args: any[]) => void
		onRunProducer?: (producer: RunProducer) => Promise<string | undefined>
		idlePane?: Snippet
		onRequestEdit?: () => void
		canRunByPath?: boolean
		onRunByPath?: (path: string, args: Record<string, any>) => Promise<string | undefined>
		selectionProducers?: Array<{ kind: 'script' | 'flow'; path: string; unsaved?: boolean }>
		downstreamSubscribers?: number
		onStartBoundedRunForOpen?: (path: string) => void
		canBoundedRunOpenScript?: boolean
		onRunCompleted?: () => void
		onTestStateChange?: (running: boolean) => void
		requestRemoveSignal?: number
		requestRunSignal?: number
		requestRunCascadeSignal?: number
		focusUploadSignal?: number
		onDraftPathChange?: (oldPath: string, newPath: string) => boolean | string
		onClose?: () => void
		onDiscard?: () => void
		onDraftSaved?: (savedPath: string) => void
		onPersistedSaved?: (savedPath: string) => void
		onScriptRenamed?: (oldPath: string, newPath: string) => void
		onScriptRemoved?: (removedPath: string) => void
	} = $props()

	let leftPaneSize = $state(60)
	let rightPaneSize = $state(40)
	let storedRightPaneSize = $state(40)

	let effectiveSelection = $derived<AssetGraphSelection | undefined>(
		editor.activeDraftPath
			? { kind: 'runnable', runnable_kind: 'script', path: editor.activeDraftPath }
			: editor.selection
	)
	let activeDraft = $derived(editor.activeDraft)
	let detailsPaneOpen = $derived(
		!panelHidden &&
			(mode === 'edit' || editor.selection != undefined || editor.activeDraftPath != undefined)
	)
	let idleView = $derived(
		mode !== 'edit' && editor.selection == undefined && editor.activeDraftPath == undefined
	)

	$effect(() => {
		if (detailsPaneOpen) {
			const restore = storedRightPaneSize > 0 ? storedRightPaneSize : 40
			rightPaneSize = restore
			leftPaneSize = 100 - restore
		} else {
			if (rightPaneSize > 0) storedRightPaneSize = rightPaneSize
			leftPaneSize = 100
		}
	})
</script>

<Splitpanes class="!h-full">
	<Pane bind:size={leftPaneSize}>
		<div class="relative h-full">
			<AssetGraphCanvas
				graph={displayGraph}
				selection={effectiveSelection}
				{hoveredPaths}
				{selectedRunPaths}
				{activeRunnable}
				{activeRunnableIds}
				{runStates}
				{pathPrefix}
				{defaultPathSuffix}
				{onCreateMissingTrigger}
				{onEditTrigger}
				{onDeleteTrigger}
				{onOpenWebhook}
				{onOpenDataUpload}
				onselect={onSelect}
				{onAddScriptForAsset}
				{onAddPipelineScript}
				{onRunnableMenuRemove}
				{onRunProducer}
				{validStartPaths}
				{onStartBoundedRun}
				{boundPick}
				{onPickEnd}
				{panToNodeId}
			/>
			{#if boundBar}{@render boundBar()}{/if}
			{#if hasAiPending && onAcceptAllProposals && onRejectAllProposals}
				<GlobalReviewButtons
					onAcceptAll={onAcceptAllProposals}
					onRejectAll={onRejectAllProposals}
				/>
			{/if}
			{#if mode === 'edit'}
				<PipelineEventLog events={eventLogEvents} />
			{/if}
			{#if prefetchingAssets}
				<div
					class="absolute top-2 left-1/2 -translate-x-1/2 z-20 flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface/95 backdrop-blur-sm border border-gray-200 dark:border-gray-700 text-2xs text-secondary shadow-sm"
					title="Inferring assets for every script in this folder so the graph is complete"
				>
					<Loader2 size={11} class="animate-spin" />
					Parsing assets…
				</div>
			{/if}
			{#if onTogglePanelHidden && (mode !== 'edit' || editor.selection != undefined || editor.activeDraftPath != undefined)}
				<div
					class="absolute top-2 right-2 z-50 rounded-md bg-surface shadow-sm border border-gray-200 dark:border-gray-700"
				>
					<HideButton hidden={panelHidden} direction="right" on:click={onTogglePanelHidden} />
				</div>
			{/if}
		</div></Pane
	>
	{#if detailsPaneOpen && workspace}
		<Pane bind:size={rightPaneSize} minSize={25}>
			{#if idleView && idlePane}
				{@render idlePane()}
			{:else}
				<AssetGraphDetailsPane
					{mode}
					{onRequestEdit}
					{canRunByPath}
					{onRunByPath}
					selection={activeDraft ? undefined : editor.selection}
					selectionProducers={activeDraft ? [] : selectionProducers}
					{runsRefreshKey}
					{runsPendingJobId}
					{activeRunnable}
					{downstreamSubscribers}
					onStartBoundedRun={canBoundedRunOpenScript &&
					editor.openScriptPath &&
					onStartBoundedRunForOpen
						? () => onStartBoundedRunForOpen?.(editor.openScriptPath!)
						: undefined}
					{onRunCompleted}
					{onTestStateChange}
					{requestRemoveSignal}
					{requestRunSignal}
					{requestRunCascadeSignal}
					{focusUploadSignal}
					draftScript={activeDraft?.script}
					{pathPrefix}
					{onDraftPathChange}
					{workspace}
					onAnnotationsChange={editor.handleAnnotationsChange}
					onAssetsChange={editor.handleAssetsChange}
					onContentChange={editor.handleContentChange}
					onDraftPersist={editor.handleDraftPersist}
					onclose={onClose}
					onHide={onTogglePanelHidden}
					{onDiscard}
					{onDraftSaved}
					{onPersistedSaved}
					{onScriptRenamed}
					{onScriptRemoved}
				/>
			{/if}
		</Pane>
	{/if}
</Splitpanes>
