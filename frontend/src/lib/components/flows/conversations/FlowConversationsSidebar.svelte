<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		MessageCircle,
		Plus,
		Trash2,
		PanelLeftClose,
		PanelLeftOpen,
		FlaskConical
	} from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Filter } from 'lucide-svelte'
	import { type FlowConversation } from '$lib/gen'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import { twMerge } from 'tailwind-merge'
	import { FlowChatManager } from './FlowChatManager.svelte'
	import { fade } from 'svelte/transition'

	interface Props {
		manager: FlowChatManager
	}

	let { manager }: Props = $props()

	function getConversationTitle(conversation: FlowConversation): string {
		return conversation.title || `Conversation ${conversation.created_at.slice(0, 10)}`
	}
</script>

<div
	class="flex flex-col h-full bg-surface border-r transition-all duration-300 {manager.isSidebarExpanded
		? 'w-60'
		: 'w-[44px]'}"
>
	<!-- Header -->
	<div class="flex-shrink-0">
		<div class="flex flex-col gap-2 p-1">
			<Button
				unifiedSize="md"
				variant="subtle"
				startIcon={{
					icon: manager.isSidebarExpanded ? PanelLeftClose : PanelLeftOpen,
					classes: 'ml-[2px]'
				}}
				onClick={() => (manager.isSidebarExpanded = !manager.isSidebarExpanded)}
				iconOnly={!manager.isSidebarExpanded}
				btnClasses={'justify-start transition-all duration-150'}
				title="Conversations"
			>
				<div transition:fade={{ duration: 100 }}> Conversations </div>
			</Button>
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
					title="Start new conversation"
					iconOnly={!manager.isSidebarExpanded}
					wrapperClasses={manager.isSidebarExpanded ? 'grow min-w-0' : ''}
					btnClasses={'w-full justify-start transition-all duration-150 whitespace-nowrap'}
				>
					<div transition:fade={{ duration: 100 }}> New chat </div>
				</Button>
				<Popover placement="bottom-start" closeButton={false}>
					{#snippet trigger()}
						<Button
							nonCaptureEvent
							unifiedSize="md"
							variant="subtle"
							startIcon={{ icon: Filter, classes: 'ml-[2px]' }}
							title="Filter conversations"
							iconOnly={!manager.isSidebarExpanded}
							btnClasses={'w-full justify-start transition-all duration-150 whitespace-nowrap'}
						>
							<div transition:fade={{ duration: 100 }}>
								Filter{manager.showTestChats ? ' · 1' : ''}
							</div>
						</Button>
					{/snippet}
					{#snippet content()}
						<div class="p-3">
							<Toggle
								size="xs"
								checked={manager.showTestChats}
								on:change={(e) => manager.setShowTestChats(e.detail)}
								options={{ right: 'Show test chats' }}
							/>
							<p class="text-2xs text-tertiary mt-1.5 max-w-[190px]">
								Chats run from the flow editor's test panel, kept apart from the flow's real
								conversations.
							</p>
						</div>
					{/snippet}
				</Popover>
			</div>
		</div>
	</div>

	<!-- Conversations List -->
	{#if !manager.isSidebarExpanded}
		<!-- Collapsed state - show single chat icon with badge -->
		<div class="p-1">
			<Button
				unifiedSize="md"
				startIcon={{ icon: MessageCircle }}
				onClick={() => (manager.isSidebarExpanded = true)}
				title="{manager.conversations.length} conversation{manager.conversations.length !== 1
					? 's'
					: ''}"
				variant="subtle"
				btnClasses="w-fit px-2 relative"
			>
				<CountBadge
					count={manager.conversations.length}
					small
					alwaysVisible={true}
					class="right-[3px] top-[3px]"
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
						<Button
							unifiedSize="md"
							variant="subtle"
							onClick={() => manager.selectConversation(conversation.id, conversation.isDraft)}
							selected={manager.selectedConversationId === conversation.id}
							btnClasses="transition-all duration-150 group"
						>
							{#if conversation.is_test}
								<!-- Both kinds share this list whenever the filter is on, so a test chat
								     has to be readable as one at a glance. -->
								<FlaskConical size={12} class="shrink-0 mr-1 text-tertiary" />
							{/if}
							<span class="flex-1 text-left truncate">
								{getConversationTitle(conversation)}
							</span>
							<Button
								wrapperClasses={twMerge(
									'ml-2 transition-all duration-100  opacity-0 group-hover:opacity-100',
									manager.deletingConversationId === conversation.id ? 'opacity-100' : ' '
								)}
								disabled={manager.deletingConversationId === conversation.id}
								onClick={(e) => {
									e?.stopPropagation()
									if (conversation.isDraft) {
										// just remove first conversation as it is the draft
										manager.conversations = [...manager.conversations.slice(1)]
									} else {
										manager.conversationListComponent?.deleteItem(conversation.id)
									}
								}}
								title="Delete conversation"
								destructive
								unifiedSize="xs"
								variant="subtle"
								loading={manager.deletingConversationId === conversation.id}
								iconOnly
								startIcon={{ icon: Trash2 }}
							/>
						</Button>
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
