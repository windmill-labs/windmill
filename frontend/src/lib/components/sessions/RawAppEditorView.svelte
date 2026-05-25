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

	let {
		runtime,
		path,
		workspaceId,
		onNavigate
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
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
	$effect(() => {
		if (!workspaceId || !path) return
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
	// Same one-way-reactive discipline as ScriptEditorView / FlowEditorView:
	// inbound tracks only UserDraft, outbound tracks only rawApp.val; each
	// side's read of the other goes through untrack() to break the
	// keystroke-revert race.
	let lastInboundSig: string | undefined = $state(undefined)

	// Store → editor. Re-runs on UserDraft changes (chat write, other
	// session edit).
	$effect(() => {
		if (!workspaceId || !path) return
		const incoming = UserDraft.get<RawAppDraft>('raw_app', path, { workspace: workspaceId })
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
		defaultSidebarCollapsed
		sidebarStorageKey="raw-app-sidebar-collapsed-preview"
	/>
{/if}
