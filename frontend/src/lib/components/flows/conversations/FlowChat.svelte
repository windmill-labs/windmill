<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { createFlowChatManager, type ConversationKind } from './FlowChatManager.svelte'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { FlowModule } from '$lib/gen'

	interface Props {
		onRunFlow: (
			userMessage: string,
			conversationId: string,
			additionalInputs?: Record<string, any>
		) => Promise<string | undefined>
		useStreaming?: boolean
		deploymentInProgress?: boolean
		path: string
		hideSidebar?: boolean
		inputSchema?: Record<string, any>
		/** The flow's modules, used to find which inputs an AI agent step reads directly. */
		flowModules?: FlowModule[]
		/** Wider centered column, for the full-page chat. */
		wideLayout?: boolean
		/** Which chats the sidebar lists before the reader filters it themselves. */
		conversationKind?: ConversationKind
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		useStreaming = false,
		path,
		hideSidebar = false,
		inputSchema = undefined,
		flowModules = undefined,
		wideLayout = false,
		conversationKind = 'deployed'
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')

	const manager = createFlowChatManager()
	manager.operatingWorkspace = () => flowEditorContext?.opWorkspace?.()
	manager.conversationKind = conversationKind

	// Initialize manager when component mounts
	$effect(() => {
		if ($workspaceStore) {
			manager.initialize(onRunFlow, path, useStreaming)
			void manager.selectLatestConversation()
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
	// stores it as the conversation's message (execution.rs:644), so the name is a
	// contract rather than the author's choice.
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

<!-- border-t: the line the chat starts at, dividing it from whatever header sits above.
     The column's max width and side padding come from AIChatDisplay itself. -->
<div class="flex overflow-hidden flex-1 border-t">
	{#if !hideSidebar}
		<FlowConversationsSidebar {manager} />
	{/if}
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
			{wideLayout}
		/>
	</div>
</div>
