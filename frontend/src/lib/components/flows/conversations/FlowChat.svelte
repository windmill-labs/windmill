<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
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
	import { getContext, untrack } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { ApiError, type FlowModule } from '$lib/gen'
	import { FlowChatPool, type FlowChatPoolState } from './flowChatPool'
	import { FlowChatViewHost, type ComposerAttachment } from './flowChatViewHost.svelte'
	import { FRAME_CLASS, loadFlowChatInputs, type ChatFrame } from './flowChatProps'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

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
		 * What the chat's stored inputs are filed under, when that is not the path. An unsaved
		 * flow's path changes as its author types, so the editor passes something that holds
		 * still for the flow it is editing.
		 */
		identity?: string
		hideSidebar?: boolean
		inputSchema?: Record<string, any>
		/** The flow's modules, read for the AI agent inputs the composer drives: the provider wiring
		 * and the attachments input. */
		flowModules?: FlowModule[]
		/** The flow's description, shown under the empty transcript's prompt. */
		description?: string
		wideLayout?: boolean
		frame?: ChatFrame
		/**
		 * What this surface's own runs are: the editor runs previews and lists its test
		 * chats, the flow page runs the deployed flow and lists only its users' chats.
		 * The sidebar offers the kind filter everywhere but on the deployed flow, whose
		 * users have no test chats to look at.
		 */
		conversationKind?: 'test' | 'deployed'
		/** What a message runs, as the chat names it. An agent has no deployed chats, so its
		 *  sidebar offers no filter between those and the test ones. */
		subject?: 'flow' | 'agent'
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
		frame = 'top',
		conversationKind = 'deployed',
		subject = 'flow'
	}: Props = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	// The editor may act on a workspace other than the nav store's (AI-session live editor).
	const workspace = $derived(flowEditorContext?.opWorkspace?.() ?? $operatingWorkspace)

	// The sidebar lists, renames and deletes through `listChat`; each conversation runs its
	// turns on its own chat in the pool, so several can answer at once.
	let listChat = $state<Chat | undefined>(undefined)
	let listState = $state<ChatState | undefined>(undefined)
	let pool = $state<FlowChatPool<FlowChatViewHost, ComposerAttachment> | undefined>(undefined)
	let poolState = $state<FlowChatPoolState | undefined>(undefined)
	let sidebar = $state<FlowConversationsSidebar | undefined>(undefined)
	/** Each conversation's kind as a listing gave it, kept past a filter that stops listing it. */
	const conversationKinds = new Map<string, boolean>()

	// What the reader chose for this flow — the model among them — held here rather than in
	// each panel, so every conversation's composer sends and shows the same values. Read again
	// wherever the chat is rebuilt below, since this component is reused from one flow to the
	// next and would otherwise send one flow's settings with another's messages.
	let inputValues = $state<Record<string, any>>({})

	$effect(() => {
		const ws = workspace
		const flowPath = path
		if (!ws || !flowPath) return
		// Untracked: the identity only names what the path already changed, and reading it here
		// would rebuild every chat of a flow whose identity merely arrived late.
		inputValues = loadFlowChatInputs({ path: flowPath, identity: untrack(() => identity) })
		conversationKinds.clear()
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
				if (!jobId) throw new Error(`the ${subject} did not start`)
				// The server creates the conversation with the run, so the sidebar can list
				// it now, whatever becomes of the turn.
				sidebar?.conversationStarted(conversationId)
				return jobId
			},
			onError: (error) => sendUserToast(`Failed to run ${subject}: ${error.message}`, true)
		}
		const api = new WindmillChatApi({ baseUrl, workspace: ws })
		const createdList = createChat(options)
		const createdPool = new FlowChatPool<FlowChatViewHost, ComposerAttachment>({
			createChat: () => createChat(options),
			createHost: (turns) => new FlowChatViewHost(turns),
			disposeHost: (host) => host.dispose(),
			holdsDraft: (host) => host.holdsDraft(),
			// Every kind: a conversation the reader follows keeps running whatever the filter lists.
			listRecent: async (page) =>
				(await api.listConversations(flowPath, { page, perPage: 50, kind: 'all' })).map((row) => ({
					id: row.id,
					runningTurn: row.running_turn
						? { jobId: row.running_turn.job_id, userSeq: row.running_turn.user_seq }
						: undefined
				}))
		})
		const unsubscribeList = createdList.subscribe((s) => {
			listState = s
			// The kind filter narrows what the listing holds, while a conversation of the other
			// kind keeps its panel mounted behind it. Remembered as each row is seen, so that
			// panel's composer keeps saying why it cannot be written to.
			for (const row of s.conversations) {
				if (row.isTest !== undefined) conversationKinds.set(row.id, row.isTest)
			}
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

	// The panel on screen. Every other panel the pool holds stays mounted behind it.
	const shownKey = $derived(poolState?.shownKey)

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
	{#if listChat && listState && pool && poolState}
		{#if !hideSidebar}
			<FlowConversationsSidebar
				bind:this={sidebar}
				{listChat}
				{pool}
				{poolState}
				defaultKind={conversationKind}
				canFilterKind={conversationKind !== 'deployed' && subject === 'flow'}
			/>
		{/if}
		<!-- pb-3 on the chat alone, not on the row: the transcript and composer stop short of
		     the panel edge the way the session chat does, while the sidebar and the border
		     dividing it from the chat still reach the bottom. -->
		<div class="relative flex flex-1 min-w-0 min-h-0">
			<!-- One panel per chat the pool holds, all mounted: the composer keeps what the
			     reader typed, and what is still being read finishes into it, so leaving a
			     conversation and coming back finds it as it was. The shown panel is in flow, so
			     the chat keeps a height of its own where the host gives none (the editor's
			     Test-flow panel); the others lie over it, invisible but laid out, which keeps
			     their transcript scrolled where it was. -->
			{#each poolState.mounted as key (key)}
				{@const panel = pool.get(key)}
				{#if panel}
					{@const shown = key === shownKey}
					<div
						class="flex min-w-0 min-h-0 pb-3 {shown ? 'relative flex-1' : 'absolute inset-0'}"
						class:invisible={!shown}
						class:pointer-events-none={!shown}
						aria-hidden={!shown}
						inert={!shown}
					>
						<FlowChatInterface
							chat={panel.chat}
							chatHost={panel.host}
							bind:inputValues
							isTestOf={(id) =>
								listState?.conversations.find((c) => c.id === id)?.isTest ??
								conversationKinds.get(id)}
							{deploymentInProgress}
							{additionalInputsSchema}
							{flowModules}
							{path}
							{identity}
							{workspace}
							{description}
							{wideLayout}
							{conversationKind}
							{subject}
						/>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
