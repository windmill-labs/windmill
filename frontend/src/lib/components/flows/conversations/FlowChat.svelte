<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createChat, type Chat, type ChatState } from 'windmill-chat'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { FlowModule } from '$lib/gen'
	import { chatFlowKey, FRAME_CLASS, type ChatFrame } from './flowChatProps'

	interface Props {
		/**
		 * Runs the flow for one turn and returns the job id: the deployed flow on the
		 * flow page, a preview run in the editor. The run must carry `memory_id` =
		 * `conversationId`, which is what ties the job to the conversation.
		 */
		onRunFlow: (
			userMessage: string,
			conversationId: string,
			additionalInputs?: Record<string, any>
		) => Promise<string | undefined>
		deploymentInProgress?: boolean
		/** The flow the chat runs and lists conversations for. Must be the path a run records,
		 *  or a conversation is stored under one path and looked for under another. */
		path: string
		/**
		 * What makes this a different chat, when that is not the path. An unsaved flow's path
		 * changes as its author types, and the chat follows the new path rather than being
		 * replaced, so the editor passes something that holds still for the flow it is editing.
		 */
		identity?: string
		hideSidebar?: boolean
		inputSchema?: Record<string, any>
		/** The flow's modules, which say which of a tool call's arguments the model supplied. */
		flowModules?: FlowModule[]
		/** The flow's description, shown under the empty transcript's prompt. */
		description?: string
		wideLayout?: boolean
		frame?: ChatFrame
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		path,
		identity = undefined,
		hideSidebar = false,
		inputSchema = undefined,
		flowModules = undefined,
		description = undefined,
		wideLayout = false,
		frame = 'top'
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	// The editor may act on a workspace other than the nav store's (AI-session live editor).
	const workspace = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	let chat = $state<Chat | undefined>(undefined)
	let chatState = $state<ChatState | undefined>(undefined)
	let sidebar = $state<FlowConversationsSidebar | undefined>(undefined)

	// The chat is built once per flow and workspace. Where the surface names the flow by
	// something steadier than its path, a rename keeps the chat and only repoints it.
	const flowKey = $derived(chatFlowKey({ path, identity }))
	const hasPath = $derived(path !== '')

	$effect(() => {
		const ws = workspace
		if (!ws || !hasPath || !flowKey) return
		const created = createChat({
			flowPath: untrack(() => path),
			workspace: ws,
			baseUrl: window.location.origin,
			history: 'server',
			// Only an enterprise server honours it; elsewhere it would just log a warning per
			// poll. The license loads asynchronously, so a cold load may create the chat twice.
			pollDelayMs: $enterpriseLicense ? 50 : undefined,
			run: async ({ user_message, ...inputs }, { conversationId }) => {
				const jobId = await onRunFlow(String(user_message), conversationId, inputs)
				if (!jobId) throw new Error('the flow did not start')
				// The server creates the conversation with the run, so the sidebar can list
				// it now, whatever becomes of the turn.
				sidebar?.conversationStarted(conversationId)
				return jobId
			},
			onError: (error) => sendUserToast('Failed to run flow: ' + error.message, true)
		})
		const unsubscribe = created.subscribe((s) => (chatState = s))
		chat = created
		return () => {
			unsubscribe()
			created.destroy()
		}
	})

	// Later runs and listings follow the path as it is typed; conversations already started
	// keep the path they were created under.
	$effect(() => {
		chat?.setFlowPath(path)
	})

	// Derive additional inputs schema (excluding user_message) for chat mode
	const additionalInputsSchema = $derived.by(() => {
		const props = inputSchema?.properties ?? {}
		const filtered = Object.fromEntries(Object.entries(props).filter(([k]) => k !== 'user_message'))
		if (Object.keys(filtered).length === 0) return undefined
		const required = inputSchema?.required
		const requiredArray: string[] = Array.isArray(required) ? required : []
		return {
			...inputSchema,
			properties: filtered,
			required: requiredArray.filter((k: string) => k !== 'user_message')
		}
	})
</script>

<div class="flex overflow-hidden flex-1 {FRAME_CLASS[frame]}">
	{#if chat && chatState}
		{#if !hideSidebar}
			<FlowConversationsSidebar bind:this={sidebar} {chat} {chatState} />
		{/if}
		<!-- pb-3 on the chat alone, not on the row: the transcript and composer stop short of
		     the panel edge the way the session chat does, while the sidebar and the border
		     dividing it from the chat still reach the bottom. -->
		<div class="flex flex-1 min-w-0 min-h-0 pb-3">
			<!-- The interface's host subscribes to the chat it was given, so a replaced chat
			     (another flow or workspace) mounts a fresh interface rather than a stale host. -->
			{#key chat}
				<FlowChatInterface
					{chat}
					{deploymentInProgress}
					{additionalInputsSchema}
					{path}
					{identity}
					{flowModules}
					{workspace}
					{description}
					{wideLayout}
				/>
			{/key}
		</div>
	{/if}
</div>
