<script lang="ts">
	import { onMount, setContext, type Snippet } from 'svelte'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import type { DraftAttachments } from '$lib/components/copilot/chat/messageDraft.svelte'
	import type { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { StarterPrompts } from '$lib/components/copilot/chat/starter/starterPrompts.svelte'
	import { goto } from '$lib/navigation'
	import { getOrCreateRuntime } from './sessionRuntime.svelte'
	import { workspaceStore } from '$lib/stores'
	import { copilotWorkspaceRequested, loadCopilot } from '$lib/components/copilot/loadCopilot'
	import { copilotWorkspace } from '$lib/aiStore'
	import {
		getEffectiveWorkspaceId,
		getSessionDraftAttachments,
		getSessionDraftPrompt,
		homeStarterSession,
		setSessionDraftAttachments,
		selectSession,
		sessionInCurrentFamily,
		sessionPageHref,
		sessionState,
		setNewSessionWorkspace,
		setSessionDraftPrompt
	} from './sessionState.svelte'
	import SessionWorkspaceBar from './SessionWorkspaceBar.svelte'
	import { composerSentFromHome, takeComposerFocus } from './homeSessionsTransition.svelte'
	import { homeComposerContext } from './homeComposerContext.svelte'

	// Home's block is the empty session's own chat, run by that session's manager: the
	// composer, its controls and what a send does are the session's, not a copy of them.
	// Picked once; the parent remounts this on a workspace switch, which changes the family.
	// `placeholder` stands in while the chat waits on its config; `onSettled` reports the height
	// the chat first took, for the next placeholder to take.
	let {
		onShown,
		placeholder,
		onSettled
	}: { onShown?: () => void; placeholder?: Snippet; onSettled?: (height: number) => void } =
		$props()

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
	// can judge it at once on arrival instead of covering it while it asks. The layout also
	// loads Home's own workspace into the same shared config, and the last request wins, so
	// this asks again whenever a later request named another workspace.
	$effect(() => {
		if (actingWorkspaceId && $copilotWorkspaceRequested !== actingWorkspaceId)
			loadCopilot(actingWorkspaceId)
	})

	// The turn is already running on the session's manager; the page it belongs to takes over.
	function onSent() {
		composerSentFromHome()
		selectSession(sessionId)
		void goto(sessionPageHref(sessionId))
	}

	function onDraftChange(text: string) {
		setSessionDraftPrompt(sessionId, text)
		// A prompt typed here is what the AI Sessions switch should open; an empty box leaves
		// the switch resuming whatever session the user was last in.
		if (text.trim()) selectSession(sessionId)
	}

	function onAttachmentsChange(attachments: DraftAttachments) {
		setSessionDraftAttachments(sessionId, attachments)
		if (Object.values(attachments).some((lane) => lane.length > 0)) selectSession(sessionId)
	}

	let aiChat: AIChat | undefined = $state(undefined)
	let chatWrapper: HTMLDivElement | undefined = $state(undefined)
	// A seeded draft holds the composer open, which is not the shape it rests in once the
	// draft is sent, so only a mount with an empty draft is measured.
	const initialAttachments = getSessionDraftAttachments(sessionId)
	let settled =
		!!getSessionDraftPrompt(sessionId)?.trim() ||
		(!!initialAttachments && Object.values(initialAttachments).some((lane) => lane.length > 0))
	$effect(() => {
		if (!aiChat || !chatWrapper || settled) return
		settled = true
		const wrapper = chatWrapper
		requestAnimationFrame(() => onSettled?.(wrapper.offsetHeight))
	})
	// The draft acts on the workspace Home shows, on mount and after any switch, whichever picker
	// made it: a pick the session page left on another workspace of the family is replaced, and a
	// switch within the family keeps this mount. A switch to another family remounts with that
	// family's draft, so this one is left where it was. A staged fork of the shown workspace stays.
	onMount(() =>
		workspaceStore.subscribe((ws) => {
			const s = sessionState.sessions.find((x) => x.id === sessionId)
			if (!ws || !s || s.workspace_id || s.pending_workspace_id === ws) return
			if (sessionInCurrentFamily(s)) setNewSessionWorkspace(sessionId, ws)
		})
	)
	onMount(() => {
		onShown?.()
		if (takeComposerFocus()) setTimeout(() => aiChat?.focusInput(), 0)
		// The rows' "Ask AI" lands here. Before the chat has mounted (its config still
		// loading) there is nowhere to put the item, and the attach does nothing.
		return homeComposerContext.register(async (element) => {
			await aiChat?.attachContext(element)
		})
	})
</script>

{#snippet actingOn()}
	{#if session}
		<SessionWorkspaceBar {session} switchesWorkspace />
	{/if}
{/snippet}

<!-- Until a config has loaded the chat reads "AI not configured" and would flash its disabled
     state. Any loaded config will do, so a fork switch does not hide it while the fork's loads.
     The item count decides whether the composer rests collapsed, so it waits for that too
     rather than open and fold up a moment later. -->
{#if session && $copilotWorkspace !== undefined && starterPrompts.fewItems !== undefined}
	<div bind:this={chatWrapper}>
		<AIChat
			bind:this={aiChat}
			hideHeader
			hideModeSelector
			wideLayout
			initialInstructions={getSessionDraftPrompt(sessionId)}
			{onDraftChange}
			initialAttachments={getSessionDraftAttachments(sessionId)}
			{onAttachmentsChange}
			starter={{
				where: 'home',
				suggestions: starterPrompts.suggestions,
				fewItems: starterPrompts.fewItems
			}}
			viewTransitionNames
			{onSent}
			{actingOn}
		/>
	</div>
{:else}
	{@render placeholder?.()}
{/if}
