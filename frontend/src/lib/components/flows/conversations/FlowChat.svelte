<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createFlowChatManager, type ConversationKind } from './FlowChatManager.svelte'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { FlowModule } from '$lib/gen'

	type ChatFrame = 'boxed' | 'top' | 'none'

	const FRAME_CLASS: Record<ChatFrame, string> = {
		boxed: 'border rounded-md',
		top: 'border-t',
		none: ''
	}

	interface Props {
		onRunFlow: (
			userMessage: string,
			conversationId: string,
			additionalInputs?: Record<string, any>
		) => Promise<string | undefined>
		useStreaming?: boolean
		deploymentInProgress?: boolean
		path: string
		/** The flow's own description, shown where the chat has room for it: the empty
		 * transcript, and the sidebar once a conversation has replaced it. */
		description?: string
		inputSchema?: Record<string, any>
		/** The flow's modules, used to find which inputs an AI agent step reads directly. */
		flowModules?: FlowModule[]
		/** Wider centered column, for the full-page chat. */
		wideLayout?: boolean
		/** What separates the chat from what sits above it. `boxed` is its own panel, corners
		 * clipped so the sidebar's edge follows them; `top` a dividing line under an enclosing
		 * header; `none` for a surface where the chat is the whole pane. */
		frame?: ChatFrame
		/** Which chats the sidebar lists before the reader filters it themselves. */
		conversationKind?: ConversationKind
		/** Whether a turn may run while another chat's is still going. Off where one run
		 * owns the surface — the editor's panel shows it on the graph. */
		parallelTurns?: boolean
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		useStreaming = false,
		path,
		description = undefined,
		inputSchema = undefined,
		flowModules = undefined,
		wideLayout = false,
		frame = 'top',
		conversationKind = 'deployed',
		parallelTurns = false
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')

	const manager = createFlowChatManager()
	manager.operatingWorkspace = () => flowEditorContext?.opWorkspace?.()
	manager.conversationKind = conversationKind
	manager.allowsParallelTurns = parallelTurns
	// The filter moves; what this surface runs does not.
	manager.surfaceKind = conversationKind === 'test' ? 'test' : 'deployed'
	// The editor is the only surface with both kinds in play, and it is the one that opens
	// on test chats. A deployed flow lists what its users started, with no way to ask for
	// anything else.
	manager.canFilterConversationKind = conversationKind !== 'deployed'

	// Initialize manager when component mounts
	$effect(() => {
		if ($workspaceStore) {
			manager.initialize(onRunFlow, path, useStreaming)
			// Reads the open conversation, and this effect tears down with `cleanup()`: tracked,
			// the first send of a fresh chat would select the conversation it just created and
			// so abort its own turn.
			untrack(() => manager.selectLatestConversation())
		}

		return () => {
			manager.cleanup()
		}
	})

	// Initialize InfiniteList when component mounts or flowPath changes
	$effect(() => {
		if ($workspaceStore && path && manager.conversationListComponent) {
			untrack(() => {
				manager.setupInfiniteList()
			})
		}
	})

	// Everything the chat asks for beyond the message itself. `user_message` is the
	// composer: the server requires that exact argument on a chat-enabled flow and
	// stores it as the conversation's message (`handle_chat_conversation_messages`), so
	// the name is a contract rather than the author's choice.
	const additionalInputsSchema = $derived.by(() => {
		const props = inputSchema?.properties ?? {}
		const messageInput = 'user_message'
		const filtered = Object.fromEntries(Object.entries(props).filter(([k]) => k !== messageInput))
		if (Object.keys(filtered).length === 0) return undefined
		const required = inputSchema?.required
		const requiredArray: string[] = Array.isArray(required) ? required : []
		return {
			...inputSchema,
			properties: filtered,
			required: requiredArray.filter((k: string) => k !== messageInput)
		}
	})
</script>

<!-- The column's max width and side padding come from AIChatDisplay itself. -->
<div class="flex overflow-hidden flex-1 {FRAME_CLASS[frame]}">
	<FlowConversationsSidebar {manager} {description} />
	<!-- pb-3 on the chat alone, not on the row: the transcript and composer stop short of
	     the panel edge the way the session chat does, while the sidebar and the border
	     dividing it from the chat still reach the bottom. -->
	<div class="flex flex-1 min-w-0 min-h-0 pb-3">
		<FlowChatInterface
			{manager}
			{deploymentInProgress}
			{additionalInputsSchema}
			{flowModules}
			{path}
			{description}
			{wideLayout}
		/>
	</div>
</div>
