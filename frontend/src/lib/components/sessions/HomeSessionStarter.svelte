<script lang="ts">
	import { onMount, setContext } from 'svelte'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import type { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { StarterPrompts } from '$lib/components/copilot/chat/starter/starterPrompts.svelte'
	import { goto } from '$lib/navigation'
	import { getOrCreateRuntime } from './sessionRuntime.svelte'
	import { workspaceStore } from '$lib/stores'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import {
		getEffectiveWorkspaceId,
		getSessionDraftPrompt,
		homeStarterSession,
		selectSession,
		sessionPageHref,
		sessionState,
		setSessionDraftPrompt
	} from './sessionState.svelte'
	import SessionWorkspaceBar from './SessionWorkspaceBar.svelte'
	import { takeComposerFocus } from './homeSessionsTransition.svelte'

	// Home's block is the empty session's own chat, run by that session's manager: the
	// composer, its controls and what a send does are the session's, not a copy of them.
	// Picked once; the parent remounts this on a workspace switch, which changes the family.
	let { onShown }: { onShown?: () => void } = $props()

	const initialSession = homeStarterSession()
	const sessionId = initialSession.id
	const runtime = getOrCreateRuntime(initialSession)
	setContext<AIChatManager>('aiChatManager', runtime.manager)

	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId))
	const actingWorkspaceId = $derived(
		(session ? getEffectiveWorkspaceId(session) : undefined) ?? $workspaceStore ?? undefined
	)
	const starterPrompts = new StarterPrompts(() => actingWorkspaceId)

	// The AI config (models, whether AI is hidden) of the workspace the draft acts on, as the
	// session page loads it, so a fork picked here shows its own models and the session page
	// can judge it at once on arrival instead of covering it while it asks.
	$effect(() => {
		if (actingWorkspaceId) loadCopilot(actingWorkspaceId)
	})

	// The turn is already running on the session's manager; the page it belongs to takes over.
	function onSent() {
		selectSession(sessionId)
		void goto(sessionPageHref(sessionId))
	}

	function onDraftChange(text: string) {
		setSessionDraftPrompt(sessionId, text)
		// A prompt typed here is what the AI Sessions switch should open; an empty box leaves
		// the switch resuming whatever session the user was last in.
		if (text.trim()) selectSession(sessionId)
	}

	let aiChat: AIChat | undefined = $state(undefined)
	onMount(() => {
		onShown?.()
		if (takeComposerFocus()) setTimeout(() => aiChat?.focusInput(), 0)
	})
</script>

{#snippet inputPreface()}
	{#if session}
		<SessionWorkspaceBar {session} />
	{/if}
{/snippet}

{#if session}
	<AIChat
		bind:this={aiChat}
		hideHeader
		hideModeSelector
		wideLayout
		initialInstructions={getSessionDraftPrompt(sessionId)}
		{onDraftChange}
		starter={{ where: 'home', prompts: starterPrompts.list, chips: starterPrompts.chipsOffered }}
		viewTransitionNames
		{onSent}
		{inputPreface}
	/>
{/if}
