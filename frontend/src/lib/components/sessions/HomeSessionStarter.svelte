<script lang="ts">
	import { onMount, setContext, type Snippet } from 'svelte'
	import AIChat from '$lib/components/copilot/chat/AIChat.svelte'
	import type { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { StarterPrompts } from '$lib/components/copilot/chat/starter/starterPrompts.svelte'
	import { goto } from '$lib/navigation'
	import { getOrCreateRuntime } from './sessionRuntime.svelte'
	import { workspaceStore } from '$lib/stores'
	import { copilotWorkspace } from '$lib/aiStore'
	import {
		getSessionDraftAttachments,
		getSessionDraftPrompt,
		holdHomeDraft,
		homeStarterSession,
		setSessionDraftAttachments,
		selectSession,
		sessionPageHref,
		sessionState,
		setSessionDraftPrompt,
		takeHomeDraftFocus
	} from './sessionState.svelte'
	import { useWorkModeLock } from './workModeLock.svelte'
	import { AIWorkMode } from '$lib/components/copilot/chat/workMode'
	import { composerSentFromHome } from './homeSessionsTransition.svelte'
	import { homeComposerContext } from './homeComposerContext.svelte'

	// Home's block is the unsent session's own chat, run by that session's manager: the
	// composer, its controls and what a send does are the session's, not a copy of them.
	// Picked once; the parent remounts this on a workspace family switch and whenever
	// another draft is asked for (New session, a draft picked in the session list).
	// `placeholder` stands in while the chat waits on its config; `onSettled` reports the height
	// the chat first took, for the next placeholder to take. A workspace small enough that the
	// composer is the page's main way in keeps it open at rest.
	let {
		placeholder,
		onSettled
	}: {
		placeholder?: Snippet
		onSettled?: (height: number) => void
	} = $props()

	const initialSession = homeStarterSession()
	const sessionId = initialSession.id
	holdHomeDraft(sessionId)
	const runtime = getOrCreateRuntime(initialSession)
	setContext<AIChatManager>('aiChatManager', runtime.manager)

	const session = $derived(sessionState.sessions.find((s) => s.id === sessionId))
	// The draft acts on the workspace the page header names: holdHomeDraft keeps it there, so
	// the lock's way out to a dev workspace switches the page.
	const workModeLock = useWorkModeLock(() => session, true)
	const starterPrompts = new StarterPrompts(
		() => $workspaceStore ?? undefined,
		() => !workModeLock.current && runtime.manager.workMode !== AIWorkMode.USE
	)

	// The turn is already running on the session's manager; the page it belongs to takes over.
	function onSent() {
		composerSentFromHome()
		selectSession(sessionId)
		void goto(sessionPageHref(sessionId))
	}

	// Asked for with the draft, and taken once the chat is there: it mounts only after a config
	// has loaded, and this block can remount meanwhile (a workspace switch reloads both).
	let aiChat: AIChat | undefined = $state(undefined)
	function bindChat(chat: AIChat | undefined) {
		aiChat = chat
		if (chat && takeHomeDraftFocus()) setTimeout(() => chat.focusInput(), 0)
	}
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
	onMount(() => {
		// The rows' "Ask AI" lands here. Before the chat has mounted (its config still
		// loading) there is nowhere to put the item, and the attach does nothing.
		return homeComposerContext.register({
			attach: async (element) => {
				await aiChat?.attachContext(element)
			},
			focus: () => aiChat?.focusInput(),
			fold: () => aiChat?.foldComposer(),
			runtime
		})
	})
	onMount(() => {
		const manager = runtime.manager
		// What the tree's folder badges count. The send path loads these, and Home has not sent.
		if (!manager.loading && !manager.sendInFlight) {
			void manager.refreshGlobalIdentity()
			void manager.refreshGlobalFolderInstructions()
			void manager.refreshGlobalSkills()
			void manager.refreshMcpServers()
		}
	})
</script>

<!-- Until a config has loaded the chat reads "AI not configured" and would flash its disabled
     state. Any loaded config will do, so a fork switch does not hide it while the fork's loads.
     The item count decides whether the composer rests collapsed, so it waits for that too
     rather than open and fold up a moment later. -->
{#if session && $copilotWorkspace !== undefined && starterPrompts.fewItems !== undefined}
	<div bind:this={chatWrapper}>
		<AIChat
			bind:this={() => aiChat, bindChat}
			hideHeader
			hideModeSelector
			wideLayout
			initialInstructions={getSessionDraftPrompt(sessionId)}
			onDraftChange={(text) => setSessionDraftPrompt(sessionId, text)}
			initialAttachments={getSessionDraftAttachments(sessionId)}
			onAttachmentsChange={(a) => setSessionDraftAttachments(sessionId, a)}
			starter={{
				suggestions: starterPrompts.suggestions,
				restsOpen: starterPrompts.fewItems
			}}
			viewTransitionNames
			{onSent}
			workModeLock={workModeLock.current}
		/>
	</div>
{:else}
	{@render placeholder?.()}
{/if}
