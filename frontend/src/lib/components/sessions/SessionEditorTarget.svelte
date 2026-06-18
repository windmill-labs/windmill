<script lang="ts">
	import { untrack, type Snippet } from 'svelte'
	import { Loader2 } from 'lucide-svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { UserDraft } from '$lib/userDraft.svelte'
	import type { SessionRuntime, SessionTargetKind } from './sessionRuntime.svelte'
	import { useUserDraftSync, type DraftSyncCodec } from './useUserDraftSync.svelte'
	import { makeFlowCodec, makeScriptCodec, makeRawAppCodec } from './sessionDraftCodecs'
	import SessionItemNotFound from './SessionItemNotFound.svelte'

	let {
		runtime,
		kind,
		path,
		workspaceId,
		effectivePath,
		editor,
		onNavigate,
		isActiveSession = true
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
		/**
		 * Only the visible session claims the workspace's single live-editor slot
		 * (one per (workspace, kind)); a warm-but-hidden session must not, else
		 * chat actions on the visible session resolve to the hidden one's path.
		 */
		isActiveSession?: boolean
	} = $props()

	const slot = $derived(runtime.slot(kind))

	function triggerLoad(): Promise<void> {
		if (kind === 'flow') return runtime.loadFlow(workspaceId, path)
		if (kind === 'script') return runtime.loadScript(workspaceId, path)
		return runtime.loadRawApp(workspaceId, path)
	}

	function buildCodec(): DraftSyncCodec<any> {
		if (kind === 'flow') return makeFlowCodec(runtime)
		if (kind === 'script') return makeScriptCodec(runtime, () => path)
		return makeRawAppCodec(runtime)
	}

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => void triggerLoad())
		}
	})

	// Mark this editor as the live editor draft for the session's workspace so
	// the chat's `isLiveDraft` hint / `discard_local_draft` tool resolve to this
	// path — same registration the regular edit pages do. Gated on
	// `isActiveSession` (see prop doc).
	$effect(() => {
		if (!workspaceId || !path) return
		if (!isActiveSession) return
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
		codec: buildCodec()
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
