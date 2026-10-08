<script lang="ts">
	import { untrack } from 'svelte'
	import { ExternalLink, Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import SessionWrapper from '$lib/components/sessions/SessionWrapper.svelte'
	import {
		createSession,
		findEmptyLandingSession,
		isTearingDownOpenSession,
		selectSession,
		sessionLastActivityAt,
		sessionPageHref,
		sessionState
	} from '$lib/components/sessions/sessionState.svelte'
	import { getOrCreateRuntime } from '$lib/components/sessions/sessionRuntime.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { connectBrowserBridge } from '$lib/components/copilot/chat/global/browserTools'
	import { extensionParentOrigin } from '$lib/components/copilot/chat/global/extensionFrame'
	import { goto } from '$lib/navigation'
	import { userStore } from '$lib/stores'
	import { base } from '$lib/base'

	// The chat-only view the Windmill browser extension frames in its side panel. A session
	// here runs as on /sessions, minus the preview pane and plus the browser tools. Anywhere
	// else it would offer neither, so it hands over to /sessions.
	const inExtension = !!extensionParentOrigin()
	if (inExtension) connectBrowserBridge()
	else goto('/sessions', { replaceState: true })

	let sessionId = $state<string | undefined>(undefined)
	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId))

	function open(id: string) {
		selectSession(id)
		sessionId = id
	}

	$effect(() => {
		// A delete awaiting its fork's removal opens the fresh session itself (onNewSession).
		if (!inExtension || !sessionState.hydrated || session || isTearingDownOpenSession()) return
		untrack(() => open((findEmptyLandingSession() ?? createSession()).id))
	})

	$effect(() => {
		if (!session) return
		untrack(() => getOrCreateRuntime(session).manager.enableBrowserTools())
	})

	const recentSessions = $derived(
		sessionState.sessions
			.filter((s) => !s.archived && (!s.transient || s.id === sessionId))
			.sort((a, b) => sessionLastActivityAt(b) - sessionLastActivityAt(a))
			.slice(0, 50)
			.map((s) => ({ label: s.summary || 'Untitled session', value: s.id }))
	)
</script>

<div class="h-screen flex flex-col bg-surface">
	{#if !isGlobalAiEnabled() || $userStore?.operator}
		<div class="p-4 text-sm text-secondary">AI sessions are not available for this account.</div>
	{:else}
		<div class="flex flex-row items-center gap-1 p-2 border-b">
			<Select
				class="flex-1 min-w-0"
				size="sm"
				items={recentSessions}
				bind:value={() => sessionId, (id) => id && open(id)}
			/>
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				startIcon={{ icon: Plus }}
				title="New session"
				aria-label="New session"
				onclick={() => open(createSession().id)}
			/>
			{#if sessionId}
				<Button
					variant="subtle"
					unifiedSize="sm"
					iconOnly
					startIcon={{ icon: ExternalLink }}
					title="Open in Windmill"
					aria-label="Open in Windmill"
					href="{base}{sessionPageHref(sessionId)}"
					target="_blank"
				/>
			{/if}
		</div>
		{#if sessionId}
			{#key sessionId}
				<SessionWrapper
					{sessionId}
					onNewSession={open}
					placeholder="Ask about this page, or run something on it"
				/>
			{/key}
		{/if}
	{/if}
</div>
