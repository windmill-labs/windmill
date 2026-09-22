<script lang="ts">
	import { resource } from 'runed'
	import { Bot, ChevronDown, ExternalLink, Pen, Plus, Trash2 } from 'lucide-svelte'
	import { base } from '$lib/base'
	import { Button, Skeleton } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import FlowChat from '$lib/components/flows/conversations/FlowChat.svelte'
	import ConversationRow from '$lib/components/flows/conversations/ConversationRow.svelte'
	import { runFlowPreview } from '$lib/components/flows/utils.svelte'
	import { agentArgsToTransforms } from '$lib/components/flows/linkedAgentDrafts'
	import {
		AGENT_CHAT_BLOCKED_REASON,
		agentArgsChatReady,
		agentChatFlow,
		agentChatReady,
		agentChatStreams,
		type AgentStepShape
	} from '$lib/components/flows/agentChatFlow'
	import type { AIAgentConfig } from '$lib/components/flows/agentResourceUtils'
	import type { AgentTool } from '$lib/components/flows/agentToolUtils'
	import {
		FlowConversationsService,
		ResourceService,
		type FlowConversation,
		type ListableResource
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import type { Item } from '$lib/utils'

	/**
	 * Every conversation with a saved agent, listed by agent the way the sidebar lists AI sessions
	 * by workspace, with the chat itself laid out as a session is: a title row, and the agent it
	 * talks to shown above the composer until the first message fixes it, then beside the title.
	 * The chat runs the deployed agent, as the agent's own page does; there is no form here.
	 */
	let ws = $derived($workspaceStore)

	type AgentGroup = { agent: ListableResource; conversations: FlowConversation[] }

	// Conversations are stored by flow path, and an agent's chats run under its resource path,
	// so grouping is a join on that path. Every agent is kept for the picker; only those someone
	// has talked to get a section.
	let catalog = resource(
		() => ws,
		async (
			ws
		): Promise<
			{ agents: ListableResource[]; groups: AgentGroup[]; chatReady: Set<string> } | undefined
		> => {
			if (!ws) return undefined
			const [agents, conversations] = await Promise.all([
				ResourceService.listResource({ workspace: ws, resourceType: 'ai_agent' }),
				FlowConversationsService.listFlowConversations({ workspace: ws, kind: 'all', perPage: 100 })
			])
			// The list leaves a deployed row's value out, and whether an agent can chat is in it.
			// Read per agent; a draft-only row carries its value and has no deployed one to read.
			const values = await Promise.all(
				agents.map((a) =>
					a.value != null
						? Promise.resolve(a.value)
						: ResourceService.getResourceValue({ workspace: ws, path: a.path }).catch(
								() => undefined
							)
				)
			)
			const chatReady = new Set(
				agents
					.filter((_, i) => agentArgsChatReady(values[i] as Record<string, any> | undefined))
					.map((a) => a.path)
			)
			const byPath = new Map<string, FlowConversation[]>()
			for (const c of conversations) {
				byPath.set(c.flow_path, [...(byPath.get(c.flow_path) ?? []), c])
			}
			const sorted = [...agents].sort((a, b) => a.path.localeCompare(b.path))
			return {
				agents: sorted,
				chatReady,
				groups: sorted
					.filter((agent) => byPath.has(agent.path))
					.map((agent) => ({
						agent,
						conversations: (byPath.get(agent.path) ?? []).sort((a, b) =>
							b.updated_at.localeCompare(a.updated_at)
						)
					}))
			}
		}
	)
	let agents = $derived(catalog.current?.agents ?? [])
	/** The agents a new chat can talk to: their memory is `auto`, which the chat needs. */
	let chatAgents = $derived(agents.filter((a) => catalog.current?.chatReady.has(a.path)))
	let groups = $derived(catalog.current?.groups ?? [])

	let selectedAgent = $state<string | undefined>(undefined)
	let selectedConversation = $state<string | undefined>(undefined)
	/** A chat being started. Its agent is picked above the composer, as a session's workspace
	 *  is, and fixed by the first message. */
	let newChat = $state(false)
	let pickedAgent = $state<string | undefined>(undefined)
	let chatStarted = $derived(newChat && selectedConversation !== undefined)

	let currentAgent = $derived(newChat ? pickedAgent : (selectedAgent ?? groups[0]?.agent.path))
	let currentResource = $derived(agents.find((a) => a.path === currentAgent))
	let currentGroup = $derived(groups.find((g) => g.agent.path === currentAgent))
	// With nothing picked, the agent's latest chat: what the panel opens on its own, named here
	// so the title and the highlighted row agree with it.
	let currentConversationId = $derived(
		newChat ? selectedConversation : (selectedConversation ?? currentGroup?.conversations[0]?.id)
	)
	let currentConversation = $derived(
		groups.flatMap((g) => g.conversations).find((c) => c.id === currentConversationId)
	)
	let title = $derived(
		newChat && !chatStarted ? 'Untitled chat' : (currentConversation?.title ?? 'Untitled chat')
	)

	function open(agentPath: string, conversationId?: string) {
		newChat = false
		selectedAgent = agentPath
		selectedConversation = conversationId
	}

	// Starts on the agent last looked at, as a session starts on the active workspace.
	function startNewChat() {
		// Read before the switch: once in a new chat, the current agent is the picked one.
		const lastLookedAt = currentAgent
		newChat = true
		pickedAgent = chatAgents.find((a) => a.path === lastLookedAt)?.path ?? chatAgents[0]?.path
		selectedConversation = undefined
	}

	// The deployed config of the agent being talked to, in the shape a flow step carries it.
	let agentResource = resource(
		() => ({ ws, path: currentAgent }),
		async ({ ws, path }) => {
			if (!ws || !path) return undefined
			return await ResourceService.getResource({ workspace: ws, path })
		}
	)
	let config = $derived((agentResource.current?.value ?? {}) as AIAgentConfig)
	let chatStep = $derived<AgentStepShape>({
		input_transforms: agentArgsToTransforms(config) as AgentStepShape['input_transforms'],
		tools: (Array.isArray(config.tools) ? config.tools : []) as AgentTool[]
	})
	let chatReady = $derived(agentResource.current !== undefined && agentChatReady(chatStep))
	let useStreaming = $derived(agentChatStreams(chatStep))
	let chatFlowShape = $derived(agentChatFlow(chatStep))

	async function runChatTurn(
		userMessage: string,
		conversationId: string,
		additionalInputs?: Record<string, any>
	): Promise<string | undefined> {
		const path = currentAgent
		if (!path) return undefined
		const jobId = await runFlowPreview(
			{ user_message: userMessage, ...(additionalInputs ?? {}) },
			agentChatFlow(chatStep),
			path,
			undefined,
			conversationId,
			undefined,
			ws
		)
		// A first turn creates the conversation this list does not have yet. The chat keeps its
		// own state; only the list's highlight follows it, so nothing reloads under the turn.
		selectedAgent = path
		selectedConversation = conversationId
		void catalog.refetch()
		return jobId
	}

	function conversationTitle(conversation: FlowConversation): string {
		return conversation.title || `Conversation ${conversation.created_at.slice(0, 10)}`
	}

	let deletingId = $state<string | undefined>(undefined)

	async function renameConversation(conversation: FlowConversation) {
		const title = window.prompt('Chat name', conversationTitle(conversation))
		if (!ws || title == null) return
		await FlowConversationsService.updateFlowConversation({
			workspace: ws,
			conversationId: conversation.id,
			requestBody: { title }
		})
		void catalog.refetch()
	}

	async function deleteConversation(conversation: FlowConversation) {
		if (!ws) return
		deletingId = conversation.id
		try {
			await FlowConversationsService.deleteFlowConversation({
				workspace: ws,
				conversationId: conversation.id
			})
			if (selectedConversation === conversation.id) selectedConversation = undefined
			await catalog.refetch()
		} catch (err) {
			sendUserToast(`Could not delete the chat: ${err}`, true)
		} finally {
			deletingId = undefined
		}
	}

	function rowActions(conversation: FlowConversation): Item[] {
		return [
			{ displayName: 'Rename', icon: Pen, action: () => renameConversation(conversation) },
			{
				displayName: 'Delete',
				icon: Trash2,
				type: 'delete',
				disabled: deletingId === conversation.id,
				action: () => deleteConversation(conversation)
			}
		]
	}

	let agentMenu = $derived(
		chatAgents.map((a) => ({
			displayName: a.description || a.path,
			// The summary alone does not tell two agents apart ("AI judge" is common); the path does.
			subtitle: a.description ? a.path : undefined,
			action: () => (pickedAgent = a.path)
		}))
	)
</script>

<!-- The chip naming the agent, as the workspace chip names a session's workspace: a picker
     before the first message, inert once the chat has one. -->
{#snippet agentChip(interactive: boolean)}
	<Button
		variant="subtle"
		unifiedSize="xs"
		title={currentResource?.path}
		tabindex={interactive ? undefined : -1}
		endIcon={interactive ? { icon: ChevronDown } : undefined}
		btnClasses="min-w-0 rounded-md text-2xs justify-start bg-surface-secondary {interactive
			? ''
			: 'cursor-default active:opacity-100 hover:bg-surface-secondary'}"
		wrapperClasses="max-w-[16rem]"
	>
		<span class="truncate min-w-0 flex-1 text-left">
			{currentResource?.description || currentResource?.path || 'Pick an agent'}
		</span>
	</Button>
{/snippet}

<!-- Nothing where the flow chat would prompt to start: the composer says it, as a session's. -->
{#snippet noHint()}{/snippet}

{#snippet talkingTo()}
	<div class="flex flex-row items-center gap-1 py-0.5 px-1 text-2xs text-secondary">
		<span class="shrink-0">Talking to</span>
		<DropdownV2 fixedHeight={false} placement="bottom-start" enableFlyTransition items={agentMenu}>
			{#snippet buttonReplacement()}
				{@render agentChip(true)}
			{/snippet}
		</DropdownV2>
	</div>
{/snippet}

<main class="h-screen w-full flex min-h-0">
	<!-- Laid out as the sidebar's session picker is: the new-item button on top, then a plain
	     section title per agent with its chats as text rows, groups spaced apart. -->
	<aside class="w-64 shrink-0 border-r flex flex-col min-h-0">
		<div class="flex flex-col gap-1 px-2 pt-3 pb-2 shrink-0">
			<MenuButton
				stopPropagationOnClick={true}
				on:click={startNewChat}
				isCollapsed={false}
				icon={Plus}
				label="New chat"
				class="!text-xs"
			/>
		</div>
		<div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-0.5 px-2 pt-2 pb-3">
			{#if catalog.loading && groups.length === 0}
				<div class="p-2"><Skeleton layout={[[1], 0.5, [1], 0.5, [1]]} /></div>
			{:else if groups.length === 0}
				<div class="p-2 text-xs text-tertiary">No conversation yet.</div>
			{:else}
				{#each groups as group, groupIdx (group.agent.path)}
					<!-- The session picker's group header, as a link to the agent: it looks the same at
					     rest and only answers the pointer. -->
					<div
						class="flex flex-row items-center gap-1 pl-1 pr-0.5 pt-2 pb-1 min-w-0 {groupIdx > 0
							? 'mt-4'
							: ''}"
					>
						<a
							href="{base}/agents/get/{group.agent.path}"
							title="Open {group.agent.path}"
							class="group/title flex items-center gap-1.5 min-w-0 text-secondary text-xs hover:text-primary"
						>
							<Bot size={14} class="shrink-0" />
							<span class="truncate group-hover/title:underline">
								{group.agent.description || group.agent.path}
							</span>
						</a>
					</div>
					{#each group.conversations as conversation (conversation.id)}
						<div class="pl-1">
							<ConversationRow
								title={conversationTitle(conversation)}
								isTest={conversation.is_test}
								selected={currentConversationId === conversation.id}
								busy={deletingId === conversation.id}
								onSelect={() => open(group.agent.path, conversation.id)}
								actions={() => rowActions(conversation)}
							/>
						</div>
					{/each}
				{/each}
			{/if}
		</div>
	</aside>

	<section class="flex-1 min-w-0 flex flex-col min-h-0 pb-2">
		{#if currentResource}
			<header class="flex flex-row items-center gap-1 pl-4 pr-4 py-2 shrink-0">
				<span class="text-sm font-semibold text-primary truncate">{title}</span>
				{#if !newChat || chatStarted}
					<!-- Fixed by the first message, so it moves up beside the title as a session's
					     workspace does. -->
					<div class="flex items-center gap-1 min-w-0 ml-2 text-2xs text-tertiary">
						<span class="shrink-0">Talking to</span>
						{@render agentChip(false)}
					</div>
				{/if}
				<div class="grow"></div>
				<Button
					variant="subtle"
					unifiedSize="xs"
					iconOnly
					startIcon={{ icon: ExternalLink }}
					title="Open agent"
					href="{base}/agents/get/{currentResource.path}"
				/>
			</header>
			<div class="flex-1 min-h-0 w-full flex flex-col {newChat && !chatStarted ? 'pt-8' : ''}">
				{#if chatReady}
					<!-- Before the first message the chat takes only the height its composer needs, so
					     it sits at the top as a new session's does, with the agent's description under
					     it rather than a prompt to start. -->
					{#key `${newChat}:${currentResource.path}`}
						<div
							class={newChat && !chatStarted
								? 'shrink-0 flex flex-col'
								: 'flex-1 min-h-0 flex flex-col'}
						>
							<FlowChat
								{useStreaming}
								onRunFlow={runChatTurn}
								conversationKind="test"
								frame="none"
								wideLayout
								path={currentResource.path}
								identity={currentResource.path}
								description={currentResource.description}
								inputSchema={chatFlowShape.schema}
								flowModules={chatFlowShape.value.modules}
								conversationId={newChat ? null : currentConversationId}
								inputPreface={newChat && !chatStarted ? talkingTo : undefined}
								emptyHint={noHint}
								hideSidebar
							/>
						</div>
					{/key}
					{#if newChat && !chatStarted && currentResource.description}
						<div class="w-full max-w-3xl mx-auto px-7 pt-4 text-xs text-tertiary">
							<GfmMarkdown md={currentResource.description} noPadding prose="sm" />
						</div>
					{/if}
				{:else if agentResource.current !== undefined}
					<div class="flex-1 flex flex-col items-center justify-center gap-3 p-6">
						{#if newChat && !chatStarted}
							{@render talkingTo()}
						{/if}
						<span class="text-xs text-tertiary">{AGENT_CHAT_BLOCKED_REASON}</span>
					</div>
				{/if}
			</div>
		{:else if !catalog.loading && (newChat || agents.length === 0)}
			<div class="flex-1 flex items-center justify-center text-xs text-tertiary">
				{agents.length === 0
					? 'No saved agent in this workspace. Create one from the home page to chat with it.'
					: 'No agent can chat yet: set its memory to auto from its editor.'}
			</div>
		{/if}
	</section>
</main>
