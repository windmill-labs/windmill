<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import type { NewScript } from '$lib/gen'
	import { UserDraft } from '$lib/userDraft.svelte'
	import SessionItemNotFound from './SessionItemNotFound.svelte'

	let {
		runtime,
		path,
		workspaceId,
		onNavigate,
		initialTestPanelCollapsed = false
	}: {
		runtime: SessionRuntime
		path: string
		workspaceId: string
		onNavigate?: (item: WorkspaceItem) => void
		initialTestPanelCollapsed?: boolean
	} = $props()

	let diffDrawer: DiffDrawer | undefined = $state()

	$effect(() => {
		if (workspaceId && path) {
			untrack(() => runtime.loadScript(workspaceId, path))
		}
	})

	async function restoreFromCurrentTarget() {
		diffDrawer?.closeDrawer()
		await runtime.loadScript(workspaceId, path)
	}

	// Mark this editor as the live editor draft for the session's workspace
	// so the chat's `isLiveDraft` hint / `discard_local_draft` tool resolve
	// to this path — same registration the regular /scripts/edit page does.
	$effect(() => {
		if (!workspaceId || !path) return
		UserDraft.setLiveEditorDraft({
			workspace: workspaceId,
			itemKind: 'script',
			storagePath: path,
			effectivePath: runtime.scriptStore.val?.path ?? path
		})
		return () =>
			UserDraft.clearLiveEditorDraft('script', { workspace: workspaceId, storagePath: path })
	})

	// Bidirectional sync between this preview and `UserDraft<NewScript>`.
	// The same path under the same workspace is shared with the session's
	// chat (read_workspace_item / write_script / edit_script) and any other
	// open editor on the same workspace.
	//
	// One-way-reactive discipline: inbound tracks ONLY UserDraft.get
	// (and unwraps `script.content` via untrack); outbound tracks ONLY
	// `script.content` (and unwraps UserDraft via untrack). Without that
	// asymmetry, a user keystroke would re-fire the inbound effect with
	// the pre-keystroke stored value and revert the edit.
	let lastInboundContent: string | undefined = $state(undefined)

	// Store → editor. Re-runs on UserDraft changes (chat write, other
	// session edit, …). `script.content` is read inside untrack so user
	// keystrokes don't refire this effect.
	$effect(() => {
		if (!workspaceId || !path) return
		const draft = UserDraft.get<NewScript>('script', path, { workspace: workspaceId })
		if (!draft || typeof draft.content !== 'string') return
		const incoming = draft.content
		untrack(() => {
			if (runtime.loadedScriptPath !== path) return
			const script = runtime.scriptStore.val
			if (!script) return
			if (incoming === script.content) return
			lastInboundContent = incoming
			script.content = incoming
			if (draft.language) script.language = draft.language
			if (draft.summary !== undefined) script.summary = draft.summary
		})
	})

	// Editor → store. Re-runs on `script.content` mutation (user typing
	// or inbound write). UserDraft is read inside untrack so writing here
	// doesn't ping-pong the inbound effect.
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedScriptPath !== path) return
		const script = runtime.scriptStore.val
		if (!script) return
		const content = script.content
		if (content === lastInboundContent) return
		untrack(() => {
			const current = UserDraft.get<NewScript>('script', path, { workspace: workspaceId })
			if (current && current.content === content) return
			UserDraft.save<NewScript>(
				'script',
				path,
				{ ...(current ?? script), ...script },
				{
					workspace: workspaceId
				}
			)
		})
	})
</script>

{#if runtime.savedScript.val}
	<DiffDrawer
		bind:this={diffDrawer}
		restoreDeployed={restoreFromCurrentTarget}
		restoreDraft={restoreFromCurrentTarget}
	/>
{/if}
{#if runtime.loadingScript && !runtime.loadedScriptPath}
	<div class="p-4 text-secondary text-sm">Loading script {path}…</div>
{:else if runtime.notFoundScript && !runtime.loadedScriptPath}
	<SessionItemNotFound kind="script" {path} {onNavigate} />
{:else if runtime.scriptStore.val}
	<ScriptBuilder
		bind:script={runtime.scriptStore.val}
		bind:savedScript={runtime.savedScript.val}
		initialPath={path}
		fullyLoaded={!runtime.loadingScript}
		disableHistoryChange={true}
		{diffDrawer}
		{onNavigate}
		{initialTestPanelCollapsed}
	/>
{/if}
