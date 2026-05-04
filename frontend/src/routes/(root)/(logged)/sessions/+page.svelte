<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import SessionWrapper from '$lib/components/sessions/SessionWrapper.svelte'
	import { sessionState } from '$lib/components/sessions/sessionState.svelte'
	import {
		editorWarmIds,
		getOrCreateRuntime,
		listRuntimes,
		promoteEditorWarm
	} from '$lib/components/sessions/sessionRuntime.svelte'
	import { visibleWorkspaceIds } from '$lib/components/sessions/sessionScope.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'

	const globalEnabled = isGlobalAiEnabled()

	const sessionName = $derived(page.url.searchParams.get('session_name') ?? '')
	// Only resolve the active session if it belongs to a workspace currently
	// in scope (active workspace + its forks). Out-of-scope sessions render
	// the "No session selected" placeholder.
	const activeSession = $derived(
		sessionState.sessions.find(
			(s) => s.name === sessionName && $visibleWorkspaceIds.has(s.workspace_id)
		)
	)

	// Touch the runtime for the active session so it gets created on first visit
	// and the pane shows up. Subsequent renders find it via listRuntimes().
	$effect(() => {
		if (activeSession) getOrCreateRuntime(activeSession)
	})

	// Warm = has a live runtime (module-scoped) AND its workspace is in scope.
	const warmSessions = $derived(
		listRuntimes()
			.map((r) => sessionState.sessions.find((s) => s.id === r.sessionId))
			.filter((s): s is NonNullable<typeof s> => s != null)
			.filter((s) => $visibleWorkspaceIds.has(s.workspace_id))
	)

	// Promote the active session in the LRU. Mutations untracked so the effect
	// only re-runs when activeSession changes, not on its own writes.
	$effect(() => {
		const id = activeSession?.id
		if (!id) return
		untrack(() => promoteEditorWarm(id))
	})
</script>

{#if !globalEnabled}
	<div class="p-8 text-secondary text-sm">
		Sessions are gated on the global-AI dev flag. Enable with
		<code class="text-2xs font-mono">localStorage.setItem('wm_dev_global_ai', '1')</code> and reload.
	</div>
{:else if !sessionName}
	<div class="p-8 text-secondary">No session selected — pick one in the sidebar.</div>
{:else}
	<div class="relative flex-1 min-h-0">
		{#each warmSessions as s (s.id)}
			<div
				class="absolute inset-0 flex flex-col {s.id === activeSession?.id
					? 'z-10 opacity-100 pointer-events-auto'
					: 'z-0 opacity-0 pointer-events-none'}"
				aria-hidden={s.id !== activeSession?.id}
			>
				<SessionWrapper sessionId={s.id} mountEditor={editorWarmIds.has(s.id)} />
			</div>
		{/each}
	</div>
{/if}
