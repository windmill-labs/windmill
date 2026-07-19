<script lang="ts">
	import { untrack } from 'svelte'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import type {
		RawAppRuntimeLogRequester,
		RawAppRunsProvider,
		RawAppScreenshotRequester
	} from '$lib/components/raw_apps/utils'
	import type { RawAppDomRequester } from '$lib/components/raw_apps/rawAppDom'
	import type { InspectorElementInfo } from '$lib/components/copilot/chat/app/core'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		isActiveSession = true,
		active = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		/** Forwarded to SessionEditorTarget — only the visible session claims the
		 * workspace's single live-editor slot. */
		isActiveSession?: boolean
		/** Whether this is the visible preview tab (forwarded as isActiveTab). */
		active?: boolean
	} = $props()

	// This tab's own raw-app cell; each open app editor binds its own store.
	const cell = $derived(runtime.rawAppCell(path))
	let diffDrawer: DiffDrawer | undefined = $state()

	// Path typed in the editor header, surfaced when it differs from the stored
	// path. Mirror it into the runtime draft as `draft_path` so the rename
	// mutates this cell's store → the autosave sig changes → the draft is saved
	// (and the home/review/Drafts lists show the friendly name). Mirrors the
	// full-page /apps_raw/edit route.
	let pendingDraftPath = $state<string | undefined>(undefined)
	// The header collapses both "not yet bound" and "reverted to baseline" to
	// `undefined`. `surfacedDraftPath` tells them apart: the initial undefined
	// (before the header binds) must not clobber the `draft_path` seeded by
	// loadRawApp, but a revert/clear after a real typed path must drop the stale
	// friendly name — mirroring the script codec's `else delete draft_path`.
	let surfacedDraftPath = false
	$effect(() => {
		const dp = pendingDraftPath
		untrack(() => {
			const val = cell.store.val
			if (!val) return
			if (dp !== undefined) {
				surfacedDraftPath = true
				if (val.draft_path !== dp) val.draft_path = dp
			} else if (surfacedDraftPath && val.draft_path !== undefined) {
				val.draft_path = undefined
			}
		})
	})

	async function reloadDeployed() {
		await runtime.loadRawApp(workspaceId, path, true, true)
	}

	async function restoreDeployed() {
		diffDrawer?.closeDrawer()
		await runResetToDeployed({
			workspace: workspaceId,
			itemKind: 'raw_app',
			path,
			onResetToDeployed: reloadDeployed
		})
		invalidateWorkspaceDrafts(workspaceId)
	}

	function registerRuntimeLogRequester(requester: RawAppRuntimeLogRequester | undefined) {
		runtime.setRuntimeLogRequester(requester)
	}

	function registerRunsProvider(provider: RawAppRunsProvider | undefined) {
		runtime.setAppRunsProvider(provider)
	}

	// This tab's live DOM query requester, registered in the runtime's per-app-path
	// map below so search_dom / read_dom can reach it while it is mounted.
	let domRequester = $state<RawAppDomRequester | undefined>(undefined)
	function registerDomRequester(requester: RawAppDomRequester | undefined) {
		domRequester = requester
	}
	// Register this tab's requester keyed by its app path. ALL mounted preview
	// tabs register (hidden ones stay mounted), so a DOM-scoped turn reads its own
	// app's live DOM even when another tab is visible — search_dom / read_dom route
	// by the chip's app path.
	$effect(() => {
		const p = path
		const r = domRequester
		if (!r) return
		runtime.registerDomRequester(p, r)
		return () => runtime.unregisterDomRequester(p, r)
	})
	// The visible tab is the default target for a query that names no app path.
	const domSlotOwner = {}
	$effect(() => {
		if (!active) return
		runtime.setActiveDomApp(path, domSlotOwner)
		return () => runtime.releaseActiveDomApp(domSlotOwner)
	})

	// An element picked in the preview inspector becomes a selector chip on the
	// session chat (the model reads it live via search_dom / read_dom). Target
	// this session's own manager (runtime.manager) — NOT the global singleton;
	// the session chat is driven by the per-session AIChatManager.
	function onInspectorSelect(info: InspectorElementInfo, additive: boolean) {
		const el = {
			selector: info.path,
			// Scope the chip to THIS app: a selector only resolves against the app it
			// was picked from, and a session can have several raw-app preview tabs.
			appPath: path,
			tagName: info.tagName,
			id: info.id,
			className: info.className
		}
		// Shift-click adds to the selection; a plain click replaces it (single-select).
		if (additive) runtime.manager.contextManager.addSelectedDomElement(el)
		else runtime.manager.contextManager.setSelectedDomElement(el)
	}

	// The chip list is the source of truth for the preview's highlights: push the
	// current selectors down so the preview renders one highlight per chip. Only
	// this app's chips (a foreign chip would resolve against the wrong preview).
	const selectedDomSelectors = $derived(
		runtime.manager.contextManager
			.getSelectedContext()
			.filter((c) => c.type === 'app_dom_selector')
			.filter((c) => c.appPath === path)
			.map((c) => c.selector)
	)

	// Chips route per app (each carries its app path) and highlights are filtered
	// per app, so cross-app chips are technically safe. But the composer chip row
	// shows every chip without an app label, so chips from two apps would be
	// indistinguishable there. Keep the selection to a single app: when this tab
	// becomes active, drop any chips belonging to a different app.
	$effect(() => {
		if (!active) return
		const p = path
		untrack(() => {
			const foreign = runtime.manager.contextManager
				.getSelectedContext()
				.some((c) => c.type === 'app_dom_selector' && c.appPath !== p)
			if (foreign) runtime.manager.contextManager.clearSelectedDomElements()
		})
	})

	// Removals originating in the preview (× on an overlay) or on rebuild. Scope to
	// this app: another preview tab can hold a chip with the same selector string.
	function onInspectorDeselect(selector: string) {
		runtime.manager.contextManager.removeSelectedDomElement(selector, path)
	}
	function onInspectorClearAll() {
		runtime.manager.contextManager.clearSelectedDomElements()
	}

	// The inline mini-composer over a selected element sends a chat turn; the
	// element is already an app_dom_selector chip, so it rides along as context.
	// Mirror the composer: while a turn is streaming, queue it (a second concurrent
	// sendRequest would race the shared abortController / streaming buffers) — it
	// auto-sends when the current turn completes.
	function onInlinePrompt(selector: string, prompt: string) {
		// Snapshot the selection synchronously at submit time so a re-selection
		// during the async send preflight (immediate path) or before the queue
		// flushes (loading path) can't swap the context. Scope the DOM chips to the
		// anchored element: the inline prompt sits over ONE element, so with several
		// selected, drop the others (a multi-select would otherwise send every
		// selector and the model couldn't tell which one the prompt is about).
		// Non-DOM context is kept.
		const snapshot = runtime.manager.contextManager
			.getSelectedContext()
			.filter((c) => c.type !== 'app_dom_selector' || c.selector === selector)
		if (runtime.manager.loading) {
			runtime.manager.queueMessage(prompt, [], snapshot)
		} else {
			void runtime.manager.sendRequest({ instructions: prompt, contextOverride: snapshot })
		}
	}

	// The preview host keeps every opened tab mounted, so registering on mount would
	// leave a background tab owning the runtime's single screenshot slot and
	// take_screenshot would capture an app the user isn't looking at. Ownership
	// follows the visible tab instead, and only the owner may release it.
	let screenshotRequester = $state<RawAppScreenshotRequester | undefined>(undefined)
	function registerScreenshotRequester(requester: RawAppScreenshotRequester | undefined) {
		screenshotRequester = requester
	}
	$effect(() => {
		const requester = screenshotRequester
		if (!active || !requester) return
		runtime.setScreenshotRequester(requester)
		return () => runtime.clearScreenshotRequester(requester)
	})
</script>

{#if cell.saved.val}
	<DiffDrawer bind:this={diffDrawer} {restoreDeployed} />
{/if}
<SessionEditorTarget
	{runtime}
	kind="raw_app"
	{path}
	{workspaceId}
	{onNavigate}
	{isActiveSession}
	isActiveTab={active}
	effectivePath={() =>
		// A raw app's typed rename lives in `draft_path` (`val.path` is the storage
		// key), unlike scripts where `val.path` is the typed name — without it the
		// live-draft registration hides a staged rename from lists and pickers.
		(cell.store.val?.draft_path || cell.store.val?.path) ?? path}
>
	{#snippet editor()}
		{#if cell.store.val}
			<!-- newApp: a draft-only app (no_deployed=true) has a truthy synthesized
			     savedApp but no deployed row, so it must deploy via createApp — keying
			     on !savedApp alone would updateApp a never-deployed path and 404
			     "not found". -->
			<!-- These bind: targets live on this tab's editor cell (cell.store.val /
			     cell.saved.val), reactive state owned by the SessionRuntime class (via
			     rawAppCell), not by a component ancestor — so Svelte's ownership check
			     flags a false positive here. -->
			<!-- svelte-ignore ownership_invalid_binding -->
			<RawAppEditor
				bind:files={cell.store.val.files}
				bind:runnables={cell.store.val.runnables}
				bind:data={cell.store.val.data}
				bind:summary={cell.store.val.summary}
				bind:pendingDraftPath
				newPath={cell.store.val.draft_path ?? cell.store.val.path}
				{path}
				autosaveWorkspace={workspaceId}
				autosavePath={path}
				policy={cell.store.val.policy}
				bind:savedApp={cell.saved.val}
				newApp={!cell.saved.val || cell.saved.val.no_deployed === true}
				{diffDrawer}
				{onNavigate}
				condensedHeader={true}
				onResetToDeployed={reloadDeployed}
				onDeploy={(e) => {
					// Sync the preview to deployed (raw apps deploy only from this editor).
					runtime.syncPreviewWithDeployed(workspaceId, 'raw_app', e.path)
					// Deploying clears the item's pending draft — refresh the Draft Count.
					invalidateWorkspaceDrafts(workspaceId)
				}}
				defaultSidebarCollapsed
				sidebarStorageKey="raw-app-sidebar-collapsed-preview"
				defaultSplitWithPreview={false}
				onRuntimeLogRequester={registerRuntimeLogRequester}
				onRunsProvider={registerRunsProvider}
				onDomRequester={registerDomRequester}
				{onInspectorSelect}
				{selectedDomSelectors}
				{onInspectorDeselect}
				{onInspectorClearAll}
				{onInlinePrompt}
				onScreenshotRequester={registerScreenshotRequester}
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
