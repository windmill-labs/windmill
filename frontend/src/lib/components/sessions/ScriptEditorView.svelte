<script lang="ts">
	import ScriptBuilder from '$lib/components/ScriptBuilder.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import { untrack } from 'svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { globalDraftStore } from '$lib/components/copilot/chat/global/draftStore.svelte'

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

	// Bidirectional sync between this preview and the global AI chat's
	// in-memory draft store. The store is workspace-scoped and keyed by
	// (type, path), so any session looking at the same script — and the
	// AI's read_workspace_item / write_script / edit_script tools — all
	// converge on the same content.
	//
	// CRITICAL: each effect must be one-way reactive. The inbound tracks
	// ONLY the store (and unwraps its mutation through `script.content`
	// via untrack), and the outbound tracks ONLY `script.content` (and
	// unwraps the store via untrack). Tracking both sides in either effect
	// creates a race: a user keystroke updates `script.content` first,
	// inbound re-fires while the store still holds the pre-keystroke
	// value, and writes the stale store value back into the editor —
	// visibly "resetting" the user's typing.
	let lastInboundContent: string | undefined = $state(undefined)

	// Store → editor. Re-runs when globalDraftStore changes (AI write,
	// other session edit, etc.). The script.content read is untracked
	// so user keystrokes don't trigger this effect.
	$effect(() => {
		if (!workspaceId || !path) return
		const draft = globalDraftStore.getScriptDraft(workspaceId, path)
		if (!draft || typeof draft.value !== 'string') return
		const incoming = draft.value
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

	// Editor → store. Re-runs when script.content changes (user typing,
	// inbound mutation). The store read is untracked so writing to it
	// here doesn't ping-pong the inbound effect.
	$effect(() => {
		if (!workspaceId || !path) return
		if (runtime.loadedScriptPath !== path) return
		const script = runtime.scriptStore.val
		if (!script) return
		const content = script.content
		if (content === lastInboundContent) return
		untrack(() => {
			const current = globalDraftStore.getScriptDraft(workspaceId, path)
			if (typeof current?.value === 'string' && current.value === content) return
			globalDraftStore.setDraft(workspaceId, {
				type: 'script',
				path,
				language: script.language,
				summary: script.summary,
				value: content,
				isDraft: true
			})
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
	<div class="p-4 text-secondary text-sm">Script not found at path {path}</div>
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
