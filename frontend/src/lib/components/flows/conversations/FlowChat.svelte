<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { createChat, type Chat, type ChatState } from 'windmill-chat'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

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
		path: string
		hideSidebar?: boolean
		inputSchema?: Record<string, any>
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		path,
		hideSidebar = false,
		inputSchema = undefined
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	// The editor may act on a workspace other than the nav store's (AI-session live editor).
	const workspace = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	let chat = $state<Chat | undefined>(undefined)
	let chatState = $state<ChatState | undefined>(undefined)
	let sidebar = $state<FlowConversationsSidebar | undefined>(undefined)

	$effect(() => {
		const ws = workspace
		const flowPath = path
		if (!ws || !flowPath) return
		const created = createChat({
			flowPath,
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

<div class="flex border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden flex-1">
	{#if chat && chatState}
		{#if !hideSidebar}
			<FlowConversationsSidebar bind:this={sidebar} {chat} {chatState} />
		{/if}
		<FlowChatInterface
			{chat}
			{chatState}
			{deploymentInProgress}
			{additionalInputsSchema}
			{path}
			{workspace}
		/>
	{/if}
</div>
