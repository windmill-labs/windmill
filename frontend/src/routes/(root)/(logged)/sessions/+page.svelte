<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import { Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { goto } from '$lib/navigation'
	import SessionWrapper from '$lib/components/sessions/SessionWrapper.svelte'
	import {
		createSession,
		selectSession,
		sessionState,
		syncWorkspaceTo
	} from '$lib/components/sessions/sessionState.svelte'
	import {
		getOrCreateRuntime,
		getRuntime,
		listRuntimes,
		promoteEditorWarm
	} from '$lib/components/sessions/sessionRuntime.svelte'
	import { markSessionSeen } from '$lib/components/sessions/sessionUnread.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { userWorkspaces } from '$lib/stores'

	const globalEnabled = isGlobalAiEnabled()

	const sessionName = $derived(page.url.searchParams.get('session_name') ?? '')

	// Unfiltered resolution by name — used to drive workspace switching
	// when a deep-linked session lives outside the current workspace.
	const sessionByName = $derived(
		sessionName ? sessionState.sessions.find((s) => s.name === sessionName) : undefined
	)

	// If the deep-linked session committed to a workspace different from
	// the active one, switch globally so visibility resolves and the
	// editor loads against the right workspace. Skip the switch when the
	// target workspace is no longer in the user's list — pointing the
	// global workspace at a deleted id would break sidebar scope and the
	// editor; SessionWrapper handles the unavailable state separately.
	$effect(() => {
		const ws = sessionByName?.workspace_id
		if (!ws) return
		if (!$userWorkspaces.find((w) => w.id === ws)) return
		untrack(() => syncWorkspaceTo(ws))
	})

	// sessionState.sessions holds every local session for the user. Resolve by
	// name without applying the sidebar root filter so an open chat survives
	// workspace switches.
	const activeSession = $derived(sessionState.sessions.find((s) => s.name === sessionName))

	// Touch the runtime for the active session so it gets created on first visit
	// and the pane shows up. Subsequent renders find it via listRuntimes().
	//
	// Gate on session identity (id) rather than the full activeSession
	// derived — sessionState.sessions mutates on every persisted change
	// (including token-by-token last_message updates during AI streaming),
	// so a value-trigger would re-run dozens of times per turn. We only
	// want to react when the user actually arrives at a new session.
	let lastArrivedSessionId: string | undefined
	$effect(() => {
		const session = activeSession
		if (!session) {
			lastArrivedSessionId = undefined
			return
		}
		if (lastArrivedSessionId === session.id) return
		lastArrivedSessionId = session.id
		untrack(() => {
			// Keep currentSessionId in sync with the URL so consumers
			// (refresh hooks, picker selection) react to deep links the
			// same way they react to picker clicks.
			selectSession(session.id)
			getOrCreateRuntime(session)
		})
	})

	// Warm = sessions that currently have a live (module-scoped) runtime. The
	// picker eagerly creates runtimes for its visible sessions, so this tracks
	// whatever the picker shows — the current family, or every family when
	// "Show all workspaces" is on. Runtimes whose session record isn't loaded
	// resolve to undefined here and drop out.
	const warmSessions = $derived(
		listRuntimes()
			.map((r) => sessionState.sessions.find((s) => s.id === r.sessionId))
			.filter((s): s is NonNullable<typeof s> => s != null)
	)

	// Promote the active session in the LRU. Mutations untracked so the effect
	// only re-runs when activeSession changes, not on its own writes.
	$effect(() => {
		const id = activeSession?.id
		if (!id) return
		untrack(() => promoteEditorWarm(id))
	})

	// Mark the active session "seen" up to its current displayMessages
	// length. Watching messages.length here means: arrive at the page →
	// clear unread; AI streams a new message while you're on the page →
	// clear unread again so the badge never lights up for a session
	// you're actively looking at. The effect only depends on the
	// length, not the array contents, so token-by-token streams within
	// a single message don't fire it on every chunk.
	$effect(() => {
		const id = activeSession?.id
		if (!id) return
		const rt = getRuntime(id)
		if (!rt) return
		const count = rt.manager.displayMessages.length
		untrack(() => markSessionSeen(id, count))
	})

	async function startNewSession() {
		const fresh = createSession()
		await goto(`/sessions?session_name=${encodeURIComponent(fresh.name)}`)
	}
</script>

{#if !globalEnabled}
	<div class="p-8 text-secondary text-sm">
		Sessions are gated on the global-AI dev flag. Enable with
		<code class="text-2xs font-mono">localStorage.setItem('wm_dev_global_ai', '1')</code> and reload.
	</div>
{:else if !sessionName}
	<div class="p-8 text-secondary">No session selected — pick one in the sidebar.</div>
{:else if !sessionByName}
	<!-- A session_name is in the URL but no session by that name exists — e.g. a
	     deleted session or a link opened in a different browser. -->
	<div class="p-8 flex flex-col items-start gap-3 text-secondary text-sm">
		<div class="flex flex-col gap-1">
			<p class="text-primary font-medium">Session not found</p>
			<p>
				No session named <code class="font-mono text-2xs">{sessionName}</code> exists. It may have been
				deleted, or this link was created in a different browser.
			</p>
		</div>
		<Button size="xs" startIcon={{ icon: Plus }} onclick={startNewSession}>New session</Button>
	</div>
{:else}
	<div class="relative flex-1 min-h-0">
		{#each warmSessions as s (s.id)}
			<div
				class="absolute inset-0 flex flex-col {s.id === activeSession?.id
					? 'z-10 opacity-100 pointer-events-auto'
					: 'z-0 opacity-0 pointer-events-none'}"
				aria-hidden={s.id !== activeSession?.id}
			>
				<SessionWrapper sessionId={s.id} />
			</div>
		{/each}
	</div>
{/if}
