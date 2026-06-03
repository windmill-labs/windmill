<script lang="ts">
	import RawAppEditor from '$lib/components/raw_apps/RawAppEditor.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import type { RawAppDraft } from './appDraftCodec'
	import { applyDraftToRuntimeRawApp, runtimeRawAppToDraft } from './appDraftCodec'
	import SessionItemNotFound from './SessionItemNotFound.svelte'
	import { invalidateWorkspaceDrafts } from '$lib/workspaceDrafts.svelte'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		isActiveSession = true
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		/**
		 * Only the visible session should claim the workspace's live-editor
		 * slot — without this, a hidden warm-mounted session can overwrite the
		 * active session's UserDraft live-editor target (one slot per
		 * (workspace, kind)), so chat actions like discard / "the open editor"
		 * resolve to the wrong session.
		 */
		isActiveSession?: boolean
	} = $props()

	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadRawApp(workspaceId, path))
		}
	})

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadRawApp(workspaceId, path)
	}

	// Mark this editor as the live editor draft for the session's workspace
	// so the chat's `isLiveDraft` hint / `discard_local_draft` tool resolve
	// to this path — same registration the regular /apps_raw/edit page does.
	// Gated on `isActiveSession`: warm-but-hidden session editors must not
	// claim the workspace's single live-editor slot, else chat actions on the
	// visible session resolve to the hidden one's path.
	$effect(() => {
		if (!workspaceId || !path) return
		if (!isActiveSession) return
		UserDraft.setLiveEditorDraft({
			workspace: workspaceId,
			itemKind: 'raw_app',
			storagePath: path,
			effectivePath: runtime.rawApp.val?.path ?? path
		})
		return () =>
			UserDraft.clearLiveEditorDraft('raw_app', { workspace: workspaceId, storagePath: path })
	})

	// Bidirectional sync between this preview and `UserDraft<RawAppDraft>`.
	// We hold a *live* handle (useMany) rather than reading via the static
	// `UserDraft.get`: the handle materializes UserDraft's shared reactive
	// `$state` cell for (workspace, 'raw_app', path), and that cell is what
	// lets the chat's writes (UserDraft.save / setDraftAndMeta, from
	// write_app_file / patch_app_file / write_app_runnable) reach this preview.
	// Without a live entry those writes only touch localStorage and the inbound
	// effect below never re-fires. A reactive getter is used (not `use()`)
	// because switching open_preview to another app swaps `path` without
	// remounting this view, so the handle must re-acquire.
	//
	// Same one-way-reactive discipline as ScriptEditorView: inbound tracks only
	// the handle's draft, outbound tracks only rawApp.val; each side's read of
	// the other goes through untrack() to break the keystroke-revert race.
	const draftHandles = UserDraft.useMany<RawAppDraft>(() => [
		{ itemKind: 'raw_app', path, workspace: workspaceId }
	])
	let lastInboundSig: string | undefined = $state(undefined)

	// Store → editor. Re-runs when the handle's draft changes (chat write,
	// other session edit).
	$effect(() => {
		if (!workspaceId || !path) return
		const incoming = draftHandles[0]?.draft
		if (!incoming) return
		const sig = JSON.stringify(incoming)
		untrack(() => {
			if (runtime.loadedRawAppPath !== path) return
			if (sig === lastInboundSig) return
			const current = runtime.rawApp.val
			if (!current) return
			lastInboundSig = sig
			runtime.rawApp.val = applyDraftToRuntimeRawApp(current, incoming)
		})
	})

	// Editor → store. Debounced 150ms so a typing burst inside a frontend
	// file's Monaco editor coalesces into one store write.
	let outboundTimer: ReturnType<typeof setTimeout> | undefined
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedRawAppPath !== path) return
		const raw = runtime.rawApp.val
		if (!raw) return
		const draft = runtimeRawAppToDraft(raw)
		const sig = JSON.stringify(draft)
		if (sig === lastInboundSig) return
		if (outboundTimer) clearTimeout(outboundTimer)
		outboundTimer = setTimeout(() => {
			untrack(() => {
				const current = UserDraft.get<RawAppDraft>('raw_app', path, { workspace: workspaceId })
				if (current && JSON.stringify(current) === sig) return
				UserDraft.save('raw_app', path, draft, { workspace: workspaceId })
			})
		}, 150)
		return () => {
			if (outboundTimer) clearTimeout(outboundTimer)
		}
	})
</script>

{#if runtime.savedRawApp.val}
	<DiffDrawer
		bind:this={diffDrawer}
		restoreDeployed={restoreFromCurrentTarget}
		restoreDraft={restoreFromCurrentTarget}
	/>
{/if}
{#if runtime.loadingRawApp && !runtime.loadedRawAppPath}
	<div class="p-4 text-secondary text-sm">Loading raw app {path}…</div>
{:else if runtime.notFoundRawApp && !runtime.loadedRawAppPath}
	<SessionItemNotFound kind="raw_app" {path} {onNavigate} />
{:else if runtime.rawApp.val}
	<RawAppEditor
		bind:files={runtime.rawApp.val.files}
		bind:runnables={runtime.rawApp.val.runnables}
		bind:data={runtime.rawApp.val.data}
		bind:summary={runtime.rawApp.val.summary}
		newPath={runtime.rawApp.val.path}
		{path}
		policy={runtime.rawApp.val.policy}
		bind:savedApp={runtime.savedRawApp.val}
		newApp={!runtime.savedRawApp.val}
		{diffDrawer}
		{onNavigate}
		onDeploy={(e) => {
			// Sync the preview to deployed (raw apps deploy only from this editor).
			runtime.syncPreviewWithDeployed(workspaceId, 'raw_app', e.path)
			// Deploying clears the item's pending draft — refresh the Draft Count.
			invalidateWorkspaceDrafts(workspaceId)
		}}
		defaultSidebarCollapsed
		sidebarStorageKey="raw-app-sidebar-collapsed-preview"
		defaultSplitWithPreview={false}
	/>
{/if}
