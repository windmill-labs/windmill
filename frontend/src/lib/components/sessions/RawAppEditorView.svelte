<script lang="ts">
	import { onDestroy, untrack } from 'svelte'
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import SessionEditorTarget from './SessionEditorTarget.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'
	import type {
		RawAppRuntimeLogRequester,
		RawAppRunsProvider
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

	// Only the ACTIVE preview tab owns the runtime's single DOM-requester slot, so
	// search_dom / read_dom always target the visible app. Hidden preview tabs stay
	// mounted, so without this an inactive tab (or a closing one) could leave the
	// slot pointing at the wrong app — unlike a plain register-on-mount.
	let domRequester = $state<RawAppDomRequester | undefined>(undefined)
	function registerDomRequester(requester: RawAppDomRequester | undefined) {
		domRequester = requester
	}
	$effect(() => {
		// Claim the slot when active (with our requester, or undefined until the
		// editor registers); an inactive tab never touches it, so switching tabs
		// can't be clobbered by a sibling's cleanup.
		if (active) runtime.setDomRequester(domRequester)
	})
	onDestroy(() => {
		// If the active tab unmounts, release the slot it owns.
		if (active) runtime.setDomRequester(undefined)
	})

	// An element picked in the preview inspector becomes a selector chip on the
	// session chat (the model reads it live via search_dom / read_dom). Target
	// this session's own manager (runtime.manager) — NOT the global singleton;
	// the session chat is driven by the per-session AIChatManager.
	function onInspectorSelect(info: InspectorElementInfo, additive: boolean) {
		const el = {
			selector: info.path,
			tagName: info.tagName,
			id: info.id,
			className: info.className
		}
		// Shift-click adds to the selection; a plain click replaces it (single-select).
		if (additive) runtime.manager.contextManager.addSelectedDomElement(el)
		else runtime.manager.contextManager.setSelectedDomElement(el)
	}

	// The chip list is the source of truth for the preview's highlights: push the
	// current selectors down so the preview renders one highlight per chip.
	const selectedDomSelectors = $derived(
		runtime.manager.contextManager
			.getSelectedContext()
			.filter((c) => c.type === 'app_dom_selector')
			.map((c) => c.selector)
	)

	// Removals originating in the preview (× on an overlay) or on rebuild.
	function onInspectorDeselect(selector: string) {
		runtime.manager.contextManager.removeSelectedDomElement(selector)
	}
	function onInspectorClearAll() {
		runtime.manager.contextManager.clearSelectedDomElements()
	}
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
	effectivePath={() => cell.store.val?.path ?? path}
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
			/>
		{/if}
	{/snippet}
</SessionEditorTarget>
