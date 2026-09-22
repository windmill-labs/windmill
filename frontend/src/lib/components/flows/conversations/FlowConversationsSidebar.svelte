<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { tick } from 'svelte'
	import {
		MessageSquare,
		Pen,
		Plus,
		Trash2,
		PanelLeftClose,
		PanelLeftOpen,
		Info
	} from 'lucide-svelte'
	import SessionStatusDot from '$lib/components/sessions/SessionStatusDot.svelte'
	import ConversationRow from './ConversationRow.svelte'
	import type { SessionChatStatus } from '$lib/components/sessions/sessionRuntime.svelte'
	import UnreadCountBadge from '$lib/components/common/badge/UnreadCountBadge.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import GfmMarkdown from '$lib/components/GfmMarkdown.svelte'
	import { emptyString, type Item } from '$lib/utils'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Filter } from 'lucide-svelte'
	import { type FlowConversation } from '$lib/gen'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import { twMerge } from 'tailwind-merge'
	import {
		FlowChatManager,
		type ConversationKind,
		type ConversationWithDraft
	} from './FlowChatManager.svelte'
	import { fade } from 'svelte/transition'

	interface Props {
		manager: FlowChatManager
		/** The flow's description. The empty transcript shows it in full, but that is gone
		 * once a chat is under way — this keeps it reachable for the rest of the session. */
		description?: string
	}

	let { manager, description = undefined }: Props = $props()

	// The chat being renamed, and the text typed so far. One at a time: the input is the
	// row's own label, so a second one would have nowhere to go.
	let renamingId = $state<string | undefined>(undefined)
	let renameDraft = $state('')
	let renameInput = $state<TextInput | undefined>(undefined)

	async function startRename(conversation: FlowConversation) {
		renamingId = conversation.id
		renameDraft = getConversationTitle(conversation)
		// The field replaces the row, so it exists only after this render.
		await tick()
		renameInput?.focus()
		renameInput?.select()
	}

	async function commitRename() {
		const id = renamingId
		renamingId = undefined
		if (id) await manager.renameConversation(id, renameDraft)
	}

	function deleteConversation(conversation: ConversationWithDraft) {
		if (conversation.isDraft) {
			// The draft is the first row and exists only here; there is nothing to delete.
			manager.conversations = [...manager.conversations.slice(1)]
		} else {
			manager.conversationListComponent?.deleteItem(conversation.id)
		}
	}

	function rowActions(conversation: ConversationWithDraft): Item[] {
		return [
			{ displayName: 'Rename', icon: Pen, action: () => startRename(conversation) },
			{
				displayName: 'Delete',
				icon: Trash2,
				type: 'delete',
				disabled: manager.deletingConversationId === conversation.id,
				action: () => deleteConversation(conversation)
			}
		]
	}

	const KIND_LABELS: Record<ConversationKind, string> = {
		test: 'Test',
		deployed: 'Deployed',
		all: 'All'
	}

	/**
	 * The sessions sidebar's own vocabulary, so the two lists read alike: a running turn is
	 * its streaming signal and a failed one its error signal. A queued message is not a dot
	 * there either — it is the pencil beside the count.
	 */
	function dotStatus(conversationId: string): SessionChatStatus {
		const status = manager.conversationStatus(conversationId)
		return status === 'running' ? 'streaming' : status === 'error' ? 'error' : 'idle'
	}

	/** Why this row cannot be opened, when something stops it. */
	function rowLocked(conversation: ConversationWithDraft): string | undefined {
		return manager.lockedReason(conversation.id)
	}

	function getConversationTitle(conversation: FlowConversation): string {
		return conversation.title || `Conversation ${conversation.created_at.slice(0, 10)}`
	}
</script>

{#snippet statusDot(conversation: ConversationWithDraft)}
	<!-- The AI session sidebar's dot, with the resting mark this list needs: a session rests
	     as a workspace or a fork, a conversation as a test run or one of the deployed flow's. -->
	<SessionStatusDot
		status={dotStatus(conversation.id)}
		isFork={false}
		restingTitle={conversation.is_test
			? 'Test chat, run from the flow editor'
			: 'Chat on the deployed flow'}
	>
		{#snippet resting()}
			<span
				class="w-[6px] h-[6px] rounded-full {conversation.is_test
					? 'border border-gray-400 dark:border-gray-500'
					: 'bg-gray-300 dark:bg-gray-600'}"
			></span>
		{/snippet}
	</SessionStatusDot>
{/snippet}

<div
	class="flex flex-col h-full bg-surface border-r transition-all duration-300 {manager.isSidebarExpanded
		? 'w-60'
		: 'w-[44px]'}"
>
	<!-- Header -->
	<div class="flex-shrink-0">
		<div class="flex flex-col gap-2 p-1">
			<!-- Same shape as the New chat row below: the wide button takes the width and the
			     icon-only one sits at the end, stacking into the rail once collapsed. -->
			<div
				class={manager.isSidebarExpanded
					? 'flex flex-row gap-1 items-center'
					: 'flex flex-col gap-2'}
			>
				<Button
					unifiedSize="md"
					variant="subtle"
					startIcon={{
						icon: manager.isSidebarExpanded ? PanelLeftClose : PanelLeftOpen,
						classes: 'ml-[2px]'
					}}
					onClick={() => (manager.isSidebarExpanded = !manager.isSidebarExpanded)}
					iconOnly={!manager.isSidebarExpanded}
					wrapperClasses={manager.isSidebarExpanded ? 'grow min-w-0' : ''}
					btnClasses={'w-full justify-start transition-all duration-150'}
					title="Conversations"
				>
					<div transition:fade={{ duration: 100 }}> Conversations </div>
				</Button>
				{#if !emptyString(description)}
					<!-- The icon is passed as the trigger rather than left to Tooltip's own: without
					     children it renders an empty trigger span beside the icon, which takes a
					     button's worth of height in this column.
					     Anchored to the icon's top: a long description centred on it would grow up
					     over the header above the chat. -->
					<!-- Sized and inset like an icon-only Button's own icon (px-2 plus the ml-[2px]
					     every icon in this column carries), so it lands on their line in the rail. -->
					<Tooltip
						placement="right-start"
						class="inline-flex items-center size-8 shrink-0 pl-[10px] text-secondary hover:text-primary"
					>
						<Info size={14} />
						{#snippet text()}
							<!-- A flow description is markdown, and is rendered as such everywhere else it
							     is shown. TooltipInner brings the width cap and the scroll. -->
							<GfmMarkdown md={description ?? ''} noPadding prose="sm" />
						{/snippet}
					</Tooltip>
				{/if}
			</div>
			<!-- Side by side while there is width for both labels; stacked once collapsed,
			     where the rail fits one icon across. -->
			<div
				class={manager.isSidebarExpanded
					? 'flex flex-row gap-1 items-center'
					: 'flex flex-col gap-2'}
			>
				<Button
					unifiedSize="md"
					variant="subtle"
					startIcon={{ icon: Plus, classes: 'ml-[2px]' }}
					onClick={() => manager.createConversation({ clearMessages: true })}
					disabled={!!manager.newChatReason}
					title={manager.newChatReason ?? 'Start new conversation'}
					iconOnly={!manager.isSidebarExpanded}
					wrapperClasses={manager.isSidebarExpanded ? 'grow min-w-0' : ''}
					btnClasses={'w-full justify-start transition-all duration-150 whitespace-nowrap'}
				>
					<div transition:fade={{ duration: 100 }}> New chat </div>
				</Button>
				{#if manager.canFilterConversationKind}
					<Popover placement="bottom-start" closeButton={false}>
						{#snippet trigger()}
							<!-- Icon-only next to the wider New chat: which kind is listed is named in
							     the title and by the group inside. -->
							<Button
								nonCaptureEvent
								unifiedSize="md"
								variant="subtle"
								startIcon={{ icon: Filter }}
								disabled={manager.isTurnInFlight}
								title={manager.isTurnInFlight
									? 'Wait for the current answer to change which chats are listed'
									: `Filter conversations · ${KIND_LABELS[manager.conversationKind]}`}
								iconOnly
							/>
						{/snippet}
						{#snippet content()}
							<div class="p-3">
								<ToggleButtonGroup
									selected={manager.conversationKind}
									onSelected={(kind) => manager.setConversationKind(kind as ConversationKind)}
									noWFull
								>
									{#snippet children({ item })}
										<ToggleButton size="sm" value="test" label={KIND_LABELS.test} {item} />
										<ToggleButton size="sm" value="deployed" label={KIND_LABELS.deployed} {item} />
										<ToggleButton size="sm" value="all" label={KIND_LABELS.all} {item} />
									{/snippet}
								</ToggleButtonGroup>
								<p class="text-2xs text-tertiary mt-1.5 max-w-[190px]">
									Test chats are the ones run from the flow editor's test panel, kept apart from the
									conversations the deployed flow's users started.
								</p>
							</div>
						{/snippet}
					</Popover>
				{/if}
			</div>
		</div>
	</div>

	<!-- Conversations List -->
	{#if !manager.isSidebarExpanded}
		<!-- Collapsed state - show single chat icon with badge -->
		<div class="p-1">
			<Button
				unifiedSize="md"
				startIcon={{ icon: MessageSquare, classes: 'ml-[2px]' }}
				onClick={() => (manager.isSidebarExpanded = true)}
				title="{manager.conversations.length} conversation{manager.conversations.length !== 1
					? 's'
					: ''}{manager.totalUnread > 0 ? `, ${manager.totalUnread} unread` : ''}"
				variant="subtle"
				btnClasses="w-fit px-2 relative"
			>
				<!-- The same badge the rows carry, over the one icon that stands for all of them:
				     collapsed, what is worth a number is what arrived, not how many chats exist. -->
				<UnreadCountBadge
					count={manager.totalUnread}
					small
					class="absolute right-[3px] top-[3px] pointer-events-none"
				/>
			</Button>
		</div>
	{/if}

	<!-- Always mount InfiniteList, but hide it when collapsed -->
	<div
		class="flex-1 overflow-hidden transition-all duration-150 p-1"
		class:hidden={!manager.isSidebarExpanded}
	>
		<InfiniteList
			bind:this={manager.conversationListComponent}
			bind:items={manager.conversations}
			selectedItemId={manager.selectedConversationId}
			noBorder={true}
			rounded={false}
			preventXOverflow={true}
		>
			{#snippet customRow({ item: conversation, hover })}
				{#if manager.isSidebarExpanded}
					<div class={twMerge('w-full pb-1')} transition:fade={{ duration: 100, delay: 30 }}>
						{#if renamingId === conversation.id}
							<!-- While renaming, the field replaces the row rather than sitting inside its
							     button: a text input nested in a button is a nested interactive control,
							     and every keystroke would have to be kept from reaching the row. -->
							<div class="flex flex-row items-center gap-1 h-8 px-2 rounded-md bg-surface-selected">
								{@render statusDot(conversation)}
								<TextInput
									bind:this={renameInput}
									bind:value={renameDraft}
									class="min-w-0 flex-1"
									size="sm"
									inputProps={{
										'aria-label': 'Chat name',
										onblur: commitRename,
										onkeydown: (e: KeyboardEvent) => {
											if (e.key === 'Enter') {
												e.preventDefault()
												commitRename()
											} else if (e.key === 'Escape') {
												e.preventDefault()
												renamingId = undefined
											}
										}
									}}
								/>
							</div>
						{:else}
							<!-- The dot sits in the slot New chat's icon occupies above, so the column lines
							     up. -->
							<ConversationRow
								title={getConversationTitle(conversation)}
								status={dotStatus(conversation.id)}
								isTest={conversation.is_test}
								selected={manager.selectedConversationId === conversation.id}
								lockedReason={rowLocked(conversation)}
								unread={manager.unreadCount(conversation.id)}
								queued={manager.conversationStatus(conversation.id) === 'queued'}
								busy={manager.deletingConversationId === conversation.id}
								onSelect={() => manager.selectConversation(conversation.id, conversation.isDraft)}
								actions={() => rowActions(conversation)}
							/>
						{/if}
					</div>
				{/if}
			{/snippet}

			{#snippet empty()}
				<div class="p-4 text-center">
					<p class="text-sm text-secondary mb-2">No conversations yet</p>
				</div>
			{/snippet}
		</InfiniteList>
	</div>
</div>
