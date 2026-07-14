<script lang="ts">
	import { setContext, untrack, type Snippet } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { UserDraft } from '$lib/userDraft.svelte'
	import type { SessionRuntime, SessionTargetKind } from './sessionRuntime.svelte'
	import { useUserDraftSync, type DraftSyncCodec } from './useUserDraftSync.svelte'
	import { makeFlowCodec, makeScriptCodec, makeRawAppCodec } from './sessionDraftCodecs'
	import { draftFriendlyLeaf } from './previewRouter'
	import SessionItemNotFound from './SessionItemNotFound.svelte'

	let {
		runtime,
		kind,
		path,
		workspaceId,
		effectivePath,
		editor,
		onNavigate,
		isActiveSession = true,
		isActiveTab = true
	}: {
		runtime: SessionRuntime
		kind: SessionTargetKind
		path: string
		workspaceId: string
		/**
		 * The path the live-editor draft should resolve to (the loaded item's own
		 * path, which may differ from the storage `path` when a draft renames it).
		 * A getter so the registration tracks it as the store settles.
		 */
		effectivePath: () => string
		/** The heavy editor for this kind; remounted on a data-ready target swap. */
		editor: Snippet
		onNavigate?: (item: WorkspaceItem) => void
		/** A warm-but-hidden session must not claim the live-editor slot. */
		isActiveSession?: boolean
		/**
		 * Only the visible editor tab claims the workspace's single live-editor slot
		 * (one per (workspace, kind)); with several editors open at once, a hidden
		 * tab must not, else chat actions resolve to the wrong item's path.
		 */
		isActiveTab?: boolean
	} = $props()

	// Mark this subtree as the session side panel: editors below detect the
	// context to hide their own AI entry points (an AI button here would nest
	// sessions), and chat-aware components resolve the session's scoped manager
	// instead of the app singleton. The value is captured at init — a reused
	// component instance keeps the first runtime's manager — so descendants may
	// rely on its presence, not its identity.
	setContext('aiChatManager', runtime.manager)

	// This tab's own editor cell (per (kind, path)); several tabs can be live at once.
	const cell = $derived(
		kind === 'flow'
			? runtime.flowCell(path)
			: kind === 'script'
				? runtime.scriptCell(path)
				: runtime.rawAppCell(path)
	)
	const slot = $derived(cell.slot)

	function triggerLoad(): Promise<void> {
		if (kind === 'flow') return runtime.loadFlow(workspaceId, path)
		if (kind === 'script') return runtime.loadScript(workspaceId, path)
		return runtime.loadRawApp(workspaceId, path)
	}

	function buildCodec(): DraftSyncCodec<any> {
		if (kind === 'flow')
			return makeFlowCodec(
				runtime.flowCell(path).store,
				runtime.flowCell(path).stateStore,
				workspaceId
			)
		if (kind === 'script') return makeScriptCodec(runtime.scriptCell(path).store, () => path)
		return makeRawAppCodec(runtime.rawAppCell(path).store)
	}
	// Rebuilds when `path` changes so the sync always binds this tab's current
	// cell: retargeting the tab (in-editor link / breadcrumb) swaps `path` without
	// remounting, and each codec closes over one cell's store.
	const codec = $derived(buildCodec())

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => void triggerLoad())
		}
	})

	// The runtime cell (store + `loadedPath`) outlives this component, so a draft
	// changed while unmounted (workspace edit, other device) would be masked by
	// `triggerLoad`'s early-return on the stale `loadedPath`. Invalidate it on
	// teardown so the next mount re-fetches as a clean first load.
	$effect(() => {
		const c = cell
		const p = path
		const w = workspaceId
		return () => {
			if (c.slot.loadedPath === p && c.slot.loadedWorkspace === w) c.slot.loadedPath = undefined
		}
	})

	// Mark this editor as the live editor draft for the session's workspace so
	// the chat's `isLiveDraft` hint / `discard_local_draft` tool resolve to this
	// path — same registration the regular edit pages do. Only the visible tab of
	// the active session claims the (workspace, kind) slot (see prop docs).
	$effect(() => {
		if (!workspaceId || !path) return
		if (!isActiveSession || !isActiveTab) return
		UserDraft.setLiveEditorDraft({
			workspace: workspaceId,
			itemKind: kind,
			storagePath: path,
			effectivePath: effectivePath()
		})
		return () => UserDraft.clearLiveEditorDraft(kind, { workspace: workspaceId, storagePath: path })
	})

	useUserDraftSync({
		path: () => path,
		workspace: () => workspaceId,
		ready: () => slot.loadedPath === path,
		codec: () => codec
	})

	// Stamp the tab's friendly label once this editor's cell knows the item's
	// typed/auto name. The page can't read the runtime cell reactively (it lives
	// outside the page's reactive root), but this editor — handed `runtime` as a
	// prop — can, so it mirrors the name onto the tab model the page does observe.
	// Only for a never-deployed item still parked at a `…/draft_<uuid>` storage
	// path; a deployed/real path keeps the plain location label.
	$effect(() => {
		const v = cell.store.val as { path?: string; draft_path?: string } | undefined
		const label = draftFriendlyLeaf(path, v?.draft_path ?? v?.path)
		runtime.previewTabs.setEditorFriendlyLabel({ kind, path }, label)
	})

	// Debounced loading affordance for a breadcrumb swap: while the loaded path
	// lags the requested `path` (data not landed), keep the old editor visible
	// for ~150ms, then dim it under a spinner. Cleared the moment the load
	// settles. The first-load / force-reload window (loadedPath === undefined)
	// uses the un-debounced branch in the markup instead.
	let showOverlay = $state(false)
	$effect(() => {
		const stale = slot.loadedPath !== undefined && slot.loadedPath !== path
		showOverlay = false
		if (!stale) return
		const t = setTimeout(() => (showOverlay = true), 150)
		return () => clearTimeout(t)
	})
</script>

{#snippet loadingOverlay(asOverlay: boolean)}
	<div
		class="flex items-center justify-center text-secondary {asOverlay
			? 'absolute inset-0 z-10 bg-surface/70'
			: 'h-full w-full bg-surface'}"
	>
		<Loader2 size={20} class="animate-spin" />
	</div>
{/snippet}

{#if slot.notFound && slot.loadedPath !== path}
	<!-- notFound-on-mismatch: the requested target 404'd. Covers first load and a
	     failed switch (where loadedPath still points at the previous target). -->
	<SessionItemNotFound {kind} {path} {onNavigate} />
{:else if slot.loadedPath === undefined}
	<!-- First load OR force-reload (loadedPath: B → undefined → B): no editor is
	     mounted, so the editor's deferred Monaco init can't race a teardown. -->
	{@render loadingOverlay(false)}
{:else}
	<!-- Breadcrumb swap (A → B): loadedPath stays A here, so editor A remains
	     visible until B lands; {#key} then remounts to B and its descendants
	     (e.g. Path.svelte's meta) re-derive from the new target. -->
	{#key slot.loadedPath}
		{@render editor()}
	{/key}
	{#if showOverlay}
		{@render loadingOverlay(true)}
	{/if}
{/if}
