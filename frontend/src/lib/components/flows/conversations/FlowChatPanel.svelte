<!--
	One chat, for one flow in one workspace. Mounted only by `FlowChat`, which keys it on
	that pair: the manager created here holds the conversations, the turn running in one of
	them and the rows that turn is writing, so pointing an existing panel at another flow
	would leave an upload, a launch or a poll landing in the chat that replaced it.
-->
<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createFlowChatManager } from './FlowChatManager.svelte'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { ChatFrame, FlowChatProps } from './flowChatProps'

	const FRAME_CLASS: Record<ChatFrame, string> = {
		boxed: 'border rounded-md',
		top: 'border-t',
		none: ''
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
	}: FlowChatProps = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')

	// The flow and the workspace this panel was built for. `FlowChat` keys on the same pair,
	// so neither can change under it: what they change is which panel exists.
	const workspace = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

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

	// The manager's inputs, kept current rather than set once: `useStreaming` follows the
	// flow's last step, which an edit can change while a turn is running.
	$effect(() => {
		manager.initialize(onRunFlow, path, useStreaming)
	})

	// Opens the chat on the conversation last spoken in. Untracked: it reads the open
	// conversation, and tracked, the first send of a fresh chat would select the conversation
	// it just created and so abort its own turn. Waits for a workspace, which an embedded
	// editor resolves from its session rather than having on the first run.
	$effect(() => {
		if (workspace) untrack(() => manager.selectLatestConversation())
	})

	// Ends whatever is still running when the panel goes away, which is the only thing that
	// should end it. Nothing is tracked here, so no input changing under the panel can.
	$effect(() => () => manager.cleanup())

	// Initialize InfiniteList when component mounts
	$effect(() => {
		if (workspace && path && manager.conversationListComponent) {
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
