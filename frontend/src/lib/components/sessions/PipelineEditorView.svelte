<script lang="ts">
	import { resource } from 'runed'
	import { onDestroy, tick, untrack } from 'svelte'
	import { Loader2, Save, Workflow } from 'lucide-svelte'
	import PipelineGraphEditor from '$lib/components/assets/AssetGraph/PipelineGraphEditor.svelte'
	import PipelineTriggerEditors from '$lib/components/assets/AssetGraph/PipelineTriggerEditors.svelte'
	import { resolveGraph } from '$lib/components/assets/AssetGraph/resolveGraph'
	import { useActiveRunnableIds } from '$lib/components/assets/AssetGraph/activeRunnables.svelte'
	import type {
		AssetGraphResponse,
		AssetGraphSelection,
		NativeTriggerKind
	} from '$lib/components/assets/AssetGraph/types'
	import { AssetService, JobService } from '$lib/gen'
	import { DATA_ASSET_KINDS } from '$lib/components/assets/AssetGraph/cascadeRun'
	import { usePipelineAssetPrefetch } from '$lib/components/assets/AssetGraph/pipelineAssetPrefetch.svelte'
	import { sendUserToast } from '$lib/utils'
	import { createPipelineAiHelpers } from '$lib/components/assets/AssetGraph/pipelineAiHelpers'
	import { deployPipelineDrafts } from '$lib/components/assets/AssetGraph/pipelineDeploy.svelte'
	import {
		diffDeployedGraph,
		extractCascadeFacts,
		formatDrift
	} from '$lib/components/assets/AssetGraph/deployGraphDiff'
	import PipelineDeployErrors from '$lib/components/assets/AssetGraph/PipelineDeployErrors.svelte'
	import PipelineDeployTriggersModal from '$lib/components/assets/AssetGraph/PipelineDeployTriggersModal.svelte'
	import AutosaveIndicator from '$lib/components/AutosaveIndicator.svelte'
	import { Button } from '$lib/components/common'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'
	import { PIPELINE_DRAFT_KIND, pipelineBundlePath } from '$lib/pipelinePaths'
	import type { DeployResult } from '$lib/utils_workspace_deploy'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { maskKey } from './modifiedItemsMask'

	let {
		runtime,
		path,
		workspaceId,
		isActiveSession = true,
		active = true
	}: {
		/** Per-session runtime; owns the pipeline editor state so it survives the
		 * editor pane unmounting on hide/show. */
		runtime: SessionRuntime
		/** Folder name the pipeline graph is scoped to (not a workspace item path). */
		path: string
		workspaceId: string
		/** Only the visible session polls the live run badges. */
		isActiveSession?: boolean
		/** Whether this is the foreground preview tab. */
		active?: boolean
	} = $props()

	// This session's own chat manager — used directly rather than via
	// getAiChatManager()'s context lookup: PreviewTabHost mounts this view in the
	// preview panel, outside the SessionWrapper subtree that provides the scoped
	// manager context, so a lookup would fall back to the app-wide singleton and
	// register the pipeline tools (build_pipeline_node / edit_pipeline_node) on the
	// wrong chat — leaving this session's chat unable to build canvas nodes.
	const aiChatManager = runtime.manager

	// Only the foreground tab of the foreground session runs the live-badge poll.
	const engaged = $derived(isActiveSession && active)

	// This folder's editor state lives on the runtime, so its drafts survive a
	// remount. The host remounts this view when the tab moves to another folder, so
	// `path` is fixed for its lifetime.
	const pe = untrack(() => runtime.pipelineEditor(path))

	const EMPTY_GRAPH: AssetGraphResponse = { assets: [], runnables: [], edges: [], triggers: [] }

	// Data assets only, as on the pipeline page: variables and resources are
	// config most scripts reference, and would swamp the layout as hub nodes.
	const graphRes = resource(
		() => ({ workspace: workspaceId, folder: path }),
		async ({ workspace, folder }) =>
			workspace && folder
				? await AssetService.getAssetsGraph({
						workspace,
						folder,
						assetKinds: DATA_ASSET_KINDS.join(',')
					})
				: EMPTY_GRAPH
	)

	const assetPrefetch = usePipelineAssetPrefetch({
		getWorkspace: () => workspaceId,
		getGraph: () => graphRes.current as AssetGraphResponse | undefined,
		editor: pe
	})

	// Folder whose graph is actually rendered — `graphRes.current` is stale-
	// while-revalidate on a folder retarget, so the canvas's one-shot initial
	// fit is keyed on the folder captured when a graph lands, not on `path`
	// (same rationale as the pipeline route page).
	let viewportFitFolder = $state('')
	$effect(() => {
		if (graphRes.current) untrack(() => (viewportFitFolder = path))
	})

	// Deployed graph + the in-flight draft overlay (AI-built nodes render as plain
	// dashed unsaved drafts, same as manual drafts). (resolveGraph's base is the
	// pipeline runnables subset; the 'job' usage_kind of the wire type never appears.)
	let resolvedGraph = $derived.by<AssetGraphResponse>(() =>
		resolveGraph({
			base: (graphRes.current ?? EMPTY_GRAPH) as AssetGraphResponse,
			drafts: pe.drafts,
			liveBodyAssets: pe.liveBodyAssets,
			liveAnnotations: pe.liveAnnotations,
			inferredWritesByPath: assetPrefetch.inferredWritesByPath,
			inferredReadsByPath: assetPrefetch.inferredReadsByPath,
			annotatedNativeKindsByPath: assetPrefetch.annotatedNativeKindsByPath
		})
	)

	const helpers = createPipelineAiHelpers({
		getFolder: () => path,
		getWorkspace: () => workspaceId,
		getResolvedGraph: () => resolvedGraph,
		getDrafts: () => pe.drafts,
		setDrafts: (next) => (pe.drafts = next),
		newDraftLocalId: pe.newDraftLocalId,
		hasTriggerDrafts: () => pe.triggerDrafts.size > 0,
		onForgetPath: (p) => {
			pe.forgetPath(p)
			pe.discardTriggerDraftsFor(p)
		},
		// Open the staged node in the details pane so its code is visible.
		onProposeNode: (p) => {
			pe.activeDraftPath = p
			pe.selection = undefined
		},
		// An AI test_pipeline_node run lights up the same live badge as the run button.
		onRunStarted: (jobId, p) => {
			activeRunnables.arm(`script:${p}`)
			activeRunnable = { kind: 'script', path: p }
			activeRunnableJobId = jobId
		}
	})

	function handleCanvasSelect(s: AssetGraphSelection | undefined) {
		if (s && s.kind === 'runnable' && s.runnable_kind === 'script' && pe.drafts.has(s.path)) {
			pe.activeDraftPath = s.path
			pe.selection = undefined
		} else {
			pe.activeDraftPath = undefined
			pe.selection = s
		}
	}

	async function afterSaved(savedPath: string) {
		assetPrefetch.forget(savedPath)
		const next = new Map(pe.drafts)
		next.delete(savedPath)
		pe.drafts = next
		if (pe.activeDraftPath === savedPath) {
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path: savedPath }
			pe.activeDraftPath = undefined
		}
		await graphRes.refetch()
	}

	// Native trigger editor drawers — the shared <PipelineTriggerEditors> the route
	// page uses; the canvas drives them imperatively. The draft guards mirror the
	// route page: a trigger row stores a hard `script_path`, so it can only attach
	// to a deployed script.
	let triggerEditors: PipelineTriggerEditors | undefined = $state(undefined)
	let focusDataUploadSignal = $state(0)

	function openMissingTriggerDrawer(kind: NativeTriggerKind, scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — triggers can only be attached to deployed scripts.`,
				true
			)
			return
		}
		triggerEditors?.openNew(kind, scriptPath)
	}

	function openEditTriggerDrawer(kind: NativeTriggerKind, triggerPath: string, scriptPath: string) {
		triggerEditors?.openEdit(kind, triggerPath, scriptPath)
	}

	function deleteAttachedTrigger(kind: NativeTriggerKind, triggerPath: string) {
		triggerEditors?.requestDelete(kind, triggerPath)
	}

	function openWebhookDrawer(scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			sendUserToast(
				`Save the script "${scriptPath}" first — webhooks only trigger the deployed version.`,
				true
			)
			return
		}
		triggerEditors?.openWebhook(scriptPath)
	}

	// Data upload has no trigger row — open the target in the details pane and pulse
	// the signal so its auto-generated run form focuses the S3 input.
	function openDataUploadRun(scriptPath: string) {
		if (pe.drafts.has(scriptPath)) {
			pe.activeDraftPath = scriptPath
			pe.selection = undefined
		} else {
			pe.activeDraftPath = undefined
			pe.selection = { kind: 'runnable', runnable_kind: 'script', path: scriptPath }
		}
		void tick().then(() => focusDataUploadSignal++)
	}

	// ── Run dispatch + live run state ──────────────────────────────────────────
	// Reuses the shared folder-scoped job poll (node badges + event log) the route
	// page uses. The session runs ONE node at a time (preview for an unsaved draft,
	// the deployed version otherwise) — it skips the route page's cascade/deploy
	// queue, which the AI-session run UX doesn't need.
	const pathPrefix = $derived(`f/${path}/`)
	const activeRunnables = useActiveRunnableIds(
		() => workspaceId,
		() => pathPrefix
	)
	// Zero-latency "running" hint for a node run launched from the canvas (the poll
	// hasn't seen the job yet). Released when that exact job reaches a terminal
	// status in the poll, so the badge hands off to the poll's status with no flash.
	let activeRunnable = $state<{ kind: 'script' | 'flow'; path: string } | undefined>(undefined)
	let activeRunnableJobId = $state<string | undefined>(undefined)
	$effect(() => {
		pathPrefix // re-scope the poll when the folder changes
		// Only the engaged (foreground) tab needs live badges/event-log; a
		// background tab or a hidden session shouldn't poll.
		activeRunnables.setObserving(engaged)
		return () => activeRunnables.dispose()
	})
	$effect(() => {
		if (!activeRunnableJobId) return
		const ev = activeRunnables.events.find((e) => e.id === activeRunnableJobId)
		if (ev && (ev.status === 'success' || ev.status === 'failure')) {
			activeRunnable = undefined
			activeRunnableJobId = undefined
		}
	})

	function runProducer(producer: { kind: 'script' | 'flow'; path: string; cascade?: boolean }) {
		// Pipeline nodes are scripts; a flow producer can't be preview/by-path run.
		if (producer.kind !== 'script') return Promise.resolve(undefined)
		return runNode(producer.path, {}, producer.cascade ?? false)
	}

	async function runNode(
		nodePath: string,
		args: Record<string, any> = {},
		cascade = false
	): Promise<string | undefined> {
		const draft = pe.drafts.get(nodePath)
		activeRunnables.arm(`script:${nodePath}`)
		try {
			let jobId: string
			if (draft) {
				// Preview-run the draft content; a preview never dispatches downstream.
				jobId = await JobService.runScriptPreview({
					workspace: workspaceId,
					requestBody: {
						path: nodePath,
						content: draft.script.content,
						language: draft.script.language,
						args
					}
				})
			} else {
				// A single-node run must NOT fan out to downstream deployed subscribers
				// via the backend asset dispatcher unless the user explicitly chose
				// "run + downstream" (cascade) — otherwise clicking one node's Run would
				// fire side-effecting production scripts.
				jobId = await JobService.runScriptByPath({
					workspace: workspaceId,
					path: nodePath,
					requestBody: { ...args, ...(cascade ? {} : { _wmill_skip_asset_dispatch: true }) }
				})
			}
			activeRunnable = { kind: 'script', path: nodePath }
			activeRunnableJobId = jobId
			return jobId
		} catch (e: any) {
			sendUserToast(`Run failed: ${e?.body ?? e?.message ?? e}`, true)
			return undefined
		}
	}

	// Register this folder's pipeline tools on the session's own manager for as long
	// as the view is mounted — background tabs included, since each folder's tools
	// act on its own state — and release them on unmount.
	$effect(() => aiChatManager.setPipelineHelpers(helpers))

	// ── Deploy all ───────────────────────────────────────────────────────────
	// The same deploy as the pipeline page's "Save all", reached from this tab's
	// button and from the session's changes list (through the runtime).
	const pendingCount = $derived(pe.drafts.size + pe.triggerDrafts.size)
	let deploying = $state(false)
	let deployErrors = $state<Map<string, string>>(new Map())
	let deployErrorsOpen = $state(false)

	let triggerConfirmOpen = $state(false)
	let resolveTriggerConfirm: ((ok: boolean) => void) | undefined
	function confirmTriggers(): Promise<boolean> {
		triggerConfirmOpen = true
		return new Promise((resolve) => (resolveTriggerConfirm = resolve))
	}
	function answerTriggerConfirm(ok: boolean) {
		triggerConfirmOpen = false
		resolveTriggerConfirm?.(ok)
		resolveTriggerConfirm = undefined
	}

	// A deploy asked for from the changes list can arrive while the tab is still
	// loading this folder's drafts.
	async function hydrated(timeoutMs = 10000): Promise<boolean> {
		for (let waited = 0; !pe.hydratedFromDb && waited < timeoutMs; waited += 100) {
			await new Promise((resolve) => setTimeout(resolve, 100))
		}
		return pe.hydratedFromDb
	}

	async function deployAll(): Promise<DeployResult> {
		// Taken before any await: a second caller (the tab's button and the changes
		// list both reach here) would otherwise replace the pending confirmation and
		// leave the first one waiting forever.
		if (deploying) return { success: false, error: 'This pipeline is already deploying.' }
		deploying = true
		try {
			if (!(await hydrated())) return { success: false, error: 'The pipeline drafts did not load.' }
			if (pendingCount === 0) {
				// Nothing left to deploy, but an earlier deploy may have failed to save
				// that: the server would still hold the deployed drafts.
				const unsaved = await saveRemaining()
				return unsaved ? { success: false, error: unsaved } : { success: true }
			}
			if (pe.triggerDrafts.size > 0 && !(await confirmTriggers())) {
				return { success: false, error: 'Deploy cancelled.' }
			}
			// What the canvas promises for each draft, checked against what the
			// backend derived once deployed.
			const predicted = new Map(
				[...pe.drafts.keys()].map((p) => [p, extractCascadeFacts(resolvedGraph, p)])
			)
			const { savedPaths, savedTriggers, errors } = await deployPipelineDrafts(pe, workspaceId)
			assetPrefetch.forget(...savedPaths)
			const bundleKey = maskKey(PIPELINE_DRAFT_KIND, pipelineBundlePath(path))
			if (aiChatManager.modifiedItems?.has(bundleKey)) {
				await aiChatManager.recordDeployedItems('script', savedPaths)
			}
			if (savedPaths.length > 0 || savedTriggers.length > 0) {
				await graphRes.refetch()
				const deployed = new Map([...predicted].filter(([p]) => savedPaths.includes(p)))
				const drift = formatDrift(
					diffDeployedGraph(deployed, (graphRes.current ?? EMPTY_GRAPH) as AssetGraphResponse)
				)
				if (drift) sendUserToast(drift, true)
			}
			// The session's changes list re-reads the draft once this resolves, so
			// what is left of it has to be saved by then.
			const unsaved = await saveRemaining()
			deployErrors = errors
			if (unsaved) return { success: false, error: `Deployed, but ${unsaved}` }
			if (errors.size > 0) {
				deployErrorsOpen = true
				return {
					success: false,
					error: `${errors.size} of the pipeline's drafts failed to deploy, see its tab.`
				}
			}
			return { success: true }
		} finally {
			deploying = false
		}
	}

	// Saves what is left of the folder's draft and says why if that failed. A
	// flush resolves even when the save failed, and the server would then keep
	// drafts already deployed — deploying them again would recreate their
	// triggers. A failed save stays queued until one succeeds, so the flush
	// retries exactly what failed, an open script's unsaved edits included.
	async function saveRemaining(): Promise<string | undefined> {
		await tick()
		const draft = {
			workspace: workspaceId,
			itemKind: PIPELINE_DRAFT_KIND,
			path: pipelineBundlePath(path)
		}
		await UserDraftDbSyncer.flush(draft)
		if (UserDraftDbSyncer.getConflict(draft).conflict)
			return 'the remaining drafts conflict with a newer version.'
		const sync = UserDraftDbSyncer.getState(draft)
		if (sync.state === 'failed')
			return `the remaining drafts could not be saved: ${sync.failureMessage ?? 'unknown error'}.`
		return undefined
	}

	async function deployAllFromButton() {
		const count = pendingCount
		const res = await deployAll()
		if (res.success) sendUserToast(`Deployed ${count} draft${count === 1 ? '' : 's'}`)
	}

	$effect(() => runtime.registerPipelineView(path, { deployAll }))
	// A deploy waiting on the trigger confirmation must still settle when the tab goes.
	onDestroy(() => answerTriggerConfirm(false))
</script>

<div class="flex flex-col h-full w-full bg-surface">
	<div
		class="flex items-center gap-2 px-3 py-1.5 border-b border-light text-xs text-secondary shrink-0"
	>
		<Workflow size={14} />
		<span class="font-mono text-emphasis truncate">f/{path}</span>
		<span class="text-tertiary">· data pipeline</span>
		{#if workspaceId}
			<AutosaveIndicator
				workspace={workspaceId}
				itemKind={PIPELINE_DRAFT_KIND}
				path={pipelineBundlePath(path)}
				draftOnly
				loadedFromDraft={pe.loadedFromDbDraft}
			/>
		{/if}
		<div class="flex-1"></div>
		{#if deployErrors.size > 0}
			<PipelineDeployErrors errors={deployErrors} bind:open={deployErrorsOpen} />
		{/if}
		{#if pendingCount > 0}
			<Button
				variant="accent"
				unifiedSize="xs"
				startIcon={{ icon: deploying ? Loader2 : Save }}
				onclick={deployAllFromButton}
				disabled={deploying}
				title={deploying ? 'Deploying drafts…' : `Deploy all ${pendingCount} drafts`}
			>
				{deploying ? 'Deploying…' : `Deploy all (${pendingCount})`}
			</Button>
		{/if}
	</div>
	<div class="flex-1 min-h-0">
		<!-- Only block on the deployed-graph fetch when there's nothing to show yet.
		     When the runtime already holds drafts (e.g. returning to a session whose
		     editor pane was LRU-unmounted, so `graphRes` re-fetches from scratch),
		     render the editor immediately so the drafts stay visible — `resolveGraph`
		     overlays them on an empty base and the deployed nodes fill in when the
		     fetch resolves. -->
		{#if graphRes.loading && !graphRes.current && pe.drafts.size === 0}
			<div class="h-full flex items-center justify-center gap-2 text-tertiary">
				<Loader2 size={18} class="animate-spin" />
				<span>Loading pipeline…</span>
			</div>
		{:else if graphRes.error && pe.drafts.size === 0}
			<div class="h-full flex items-center justify-center text-red-500 text-sm px-4 text-center">
				Failed to load pipeline: {graphRes.error.message}
			</div>
		{:else}
			<!-- The session preview shares the route page's editor body and persists
			     drafts to the same per-folder DB draft, so AI drafts survive a reload /
			     session switch (and an LRU-evicted runtime). It wires the shared run +
			     trigger affordances (handlers above) but omits the route page's
			     cascade / bounded-run / add-script-from-asset callbacks, so those stay
			     hidden. -->
			<PipelineGraphEditor
				editor={pe}
				folder={path}
				viewportFitKey={viewportFitFolder}
				persistDrafts={true}
				modalPanel
				prefetchingAssets={assetPrefetch.prefetching}
				displayGraph={resolvedGraph}
				mode="edit"
				workspace={workspaceId}
				{pathPrefix}
				onCreateMissingTrigger={openMissingTriggerDrawer}
				onEditTrigger={openEditTriggerDrawer}
				onDeleteTrigger={deleteAttachedTrigger}
				onOpenWebhook={openWebhookDrawer}
				onOpenDataUpload={openDataUploadRun}
				focusUploadSignal={focusDataUploadSignal}
				{activeRunnable}
				activeRunnableIds={activeRunnables.ids}
				runStates={activeRunnables.states}
				eventLogEvents={activeRunnables.events}
				onRunProducer={runProducer}
				onRunByPath={(path, args) => runNode(path, args)}
				canRunByPath
				onTestStateChange={(running) => {
					const openPath = pe.openScriptPath
					if (running && openPath) {
						activeRunnable = { kind: 'script', path: openPath }
						activeRunnables.arm(`script:${openPath}`)
						activeRunnableJobId = undefined
					} else if (!running && activeRunnable?.path === openPath) {
						// Only clear the hint for the script the pane just finished — a
						// canvas per-node run of a different script keeps its own hint.
						activeRunnable = undefined
						activeRunnableJobId = undefined
					}
				}}
				onRunCompleted={() => {
					activeRunnable = undefined
					activeRunnableJobId = undefined
				}}
				onSelect={handleCanvasSelect}
				onDraftSaved={afterSaved}
				onPersistedSaved={afterSaved}
				onScriptRemoved={async (removedPath) => {
					pe.forgetPath(removedPath)
					await graphRes.refetch()
				}}
				onScriptRenamed={async (oldPath, newPath) => {
					assetPrefetch.forget(oldPath, newPath)
					// Repoint the selection so the canvas follows the renamed node instead
					// of staying on the now-gone old path until an unrelated refetch.
					if (pe.selection?.kind === 'runnable' && pe.selection.path === oldPath) {
						pe.selection = { ...pe.selection, path: newPath }
					}
					await graphRes.refetch()
				}}
				onDiscard={() => {
					if (pe.activeDraftPath) pe.discardDraft(pe.activeDraftPath)
				}}
				onClose={() => {
					pe.selection = undefined
					pe.activeDraftPath = undefined
					pe.clearLiveOverlays()
				}}
			/>
		{/if}
	</div>
</div>

<!-- Native trigger editor drawers (schedule/kafka/webhook/…), shared with the
     route page; opened imperatively from the canvas via the handlers above.
     `workspace={workspaceId}` scopes every trigger backend call to THIS
     session's (possibly forked) workspace — a session never switches the global
     `$workspaceStore` (SessionPicker), so without this the editors would write
     to the nav workspace. -->
<PipelineTriggerEditors
	bind:this={triggerEditors}
	mountTriggerEditors
	workspace={workspaceId}
	onUpdate={() => graphRes.refetch()}
/>

<PipelineDeployTriggersModal
	open={triggerConfirmOpen}
	triggerDrafts={pe.triggerDrafts}
	onConfirmed={() => answerTriggerConfirm(true)}
	onCanceled={() => answerTriggerConfirm(false)}
/>
