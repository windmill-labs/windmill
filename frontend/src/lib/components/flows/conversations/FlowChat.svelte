<script lang="ts">
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import {
		createChat,
		turnRunningError,
		WindmillChatApi,
		type Chat,
		type ChatOptions,
		type ChatState
	} from 'windmill-chat'
	import FlowConversationsSidebar from './FlowConversationsSidebar.svelte'
	import FlowChatInterface from './FlowChatInterface.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { ApiError, type FlowModule } from '$lib/gen'
	import { FlowChatPool, type FlowChatPoolState } from './flowChatPool'
	import { FlowChatViewHost } from './flowChatViewHost.svelte'

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
		/** The flow's modules, read for the provider wiring of its AI agent steps. */
		flowModules?: FlowModule[]
		/** The flow's description, shown under the empty transcript's prompt. */
		description?: string
		wideLayout?: boolean
		/**
		 * What this surface's own runs are: the editor runs previews and lists its test
		 * chats, the flow page runs the deployed flow and lists only its users' chats.
		 * The sidebar offers the kind filter everywhere but on the deployed flow, whose
		 * users have no test chats to look at.
		 */
		conversationKind?: 'test' | 'deployed'
	}

	let {
		onRunFlow,
		deploymentInProgress = false,
		path,
		hideSidebar = false,
		inputSchema = undefined,
		flowModules = undefined,
		description = undefined,
		wideLayout = false,
		conversationKind = 'deployed'
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	// The editor may act on a workspace other than the nav store's (AI-session live editor).
	const workspace = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	// The sidebar lists, renames and deletes through `listChat`; each conversation runs its
	// turns on its own chat in the pool, so several can answer at once.
	let listChat = $state<Chat | undefined>(undefined)
	let listState = $state<ChatState | undefined>(undefined)
	let pool = $state<FlowChatPool<FlowChatViewHost> | undefined>(undefined)
	let poolState = $state<FlowChatPoolState | undefined>(undefined)
	let sidebar = $state<FlowConversationsSidebar | undefined>(undefined)

	$effect(() => {
		const ws = workspace
		const flowPath = path
		if (!ws || !flowPath) return
		const baseUrl = window.location.origin
		const options: ChatOptions = {
			flowPath,
			workspace: ws,
			baseUrl,
			history: 'server',
			// Only an enterprise server honours it; elsewhere it would just log a warning per
			// poll. The license loads asynchronously, so a cold load may create the chat twice.
			pollDelayMs: $enterpriseLicense ? 50 : undefined,
			run: async ({ user_message, ...inputs }, { conversationId }) => {
				let jobId: string | undefined
				try {
					jobId = await onRunFlow(String(user_message), conversationId, inputs)
				} catch (e) {
					// The conversation is still answering a message sent elsewhere; the chat
					// follows that turn instead of failing this one.
					if (e instanceof ApiError && e.status === 409) {
						throw turnRunningError(String(e.body)) ?? e
					}
					throw e
				}
				if (!jobId) throw new Error('the flow did not start')
				// The server creates the conversation with the run, so the sidebar can list
				// it now, whatever becomes of the turn.
				sidebar?.conversationStarted(conversationId)
				return jobId
			},
			onError: (error) => sendUserToast('Failed to run flow: ' + error.message, true)
		}
		const api = new WindmillChatApi({ baseUrl, workspace: ws })
		const createdList = createChat(options)
		const createdPool = new FlowChatPool<FlowChatViewHost>({
			createChat: () => createChat(options),
			createHost: (chat) => new FlowChatViewHost(chat),
			disposeHost: (host) => host.dispose(),
			hasQueued: (host) => host.queuedMessage !== '',
			resumeTurn: (host, turn) => host.resumeTurn(turn),
			isRunFinished: async (jobId) => (await api.getCompletedResult(jobId)).completed
		})
		const unsubscribeList = createdList.subscribe((s) => {
			listState = s
			createdPool.setListed(s.conversations)
		})
		const unsubscribePool = createdPool.subscribe((s) => (poolState = s))
		listChat = createdList
		pool = createdPool
		return () => {
			unsubscribeList()
			unsubscribePool()
			createdPool.destroy()
			createdList.destroy()
		}
	})

	// A new chat keeps its chat once its first turn names the conversation, so the panel is
	// only remounted when the reader moves to another one.
	const shown = $derived.by(() => {
		void poolState?.selectedId
		return pool?.selected
	})
	const shownIsTest = $derived(
		listState?.conversations.find((c) => c.id === poolState?.selectedId)?.isTest
	)

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
	{#if listChat && listState && pool && poolState && shown}
		{#if !hideSidebar}
			<FlowConversationsSidebar
				bind:this={sidebar}
				{listChat}
				{pool}
				{poolState}
				defaultKind={conversationKind}
				canFilterKind={conversationKind !== 'deployed'}
			/>
		{/if}
		{#key shown}
			<FlowChatInterface
				chat={shown.chat}
				chatHost={shown.host}
				isTest={shownIsTest}
				{deploymentInProgress}
				{additionalInputsSchema}
				{flowModules}
				{path}
				{workspace}
				{description}
				{wideLayout}
				{conversationKind}
			/>
		{/key}
	{/if}
</div>
