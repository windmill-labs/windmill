<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		MessageCircle,
		Plus,
		Trash2,
		PanelLeftClose,
		PanelLeftOpen,
		Loader2
	} from 'lucide-svelte'
	import { type FlowConversation } from '$lib/gen'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import { twMerge } from 'tailwind-merge'
	import { FlowChatManager } from './FlowChatManager.svelte'

	interface Props {
		manager: FlowChatManager
	}

	let { manager }: Props = $props()

	function getConversationTitle(conversation: FlowConversation): string {
		return conversation.title || `Conversation ${conversation.created_at.slice(0, 10)}`
	}
</script>

<div
	class="flex flex-col h-full bg-surface border-r border-gray-200 dark:border-gray-700 transition-all duration-300 {manager.isSidebarExpanded
		? 'w-60'
		: 'w-16'}"
>
	<!-- Header -->
	<div class="flex-shrink-0 p-2 border-b border-gray-200 dark:border-gray-700">
		<div class="flex flex-col gap-2">
			<Button
				size="sm"
				color="light"
				startIcon={{ icon: manager.isSidebarExpanded ? PanelLeftClose : PanelLeftOpen }}
				onclick={() => (manager.isSidebarExpanded = !manager.isSidebarExpanded)}
				iconOnly={!manager.isSidebarExpanded}
				btnClasses={manager.isSidebarExpanded ? '!justify-start' : ''}
				label="Conversations"
			>
				Conversations
			</Button>
			<Button
				size="sm"
				color="light"
				startIcon={{ icon: Plus }}
				onclick={() => manager.createConversation({ clearMessages: true })}
				title="Start new conversation"
				iconOnly={!manager.isSidebarExpanded}
				btnClasses={manager.isSidebarExpanded ? '!justify-start' : ''}
				label="New chat"
			>
				New chat
			</Button>
		</div>
	</div>

	<!-- Conversations List -->
	{#if !manager.isSidebarExpanded}
		<!-- Collapsed state - show single chat icon with badge -->
		<div class="p-2 flex flex-col items-center mt-2">
			<button
				class="relative w-[23px] h-[23px] rounded-md center-center hover:bg-surface-hover transition-all duration-100 text-secondary hover:text-primary group"
				onclick={() => (manager.isSidebarExpanded = true)}
				title="{manager.conversations.length} conversation{manager.conversations.length !== 1
					? 's'
					: ''}"
			>
				<MessageCircle size={16} />
				<CountBadge count={manager.conversations.length} small={true} alwaysVisible={true} />
			</button>
		</div>
	{/if}

	<!-- Always mount InfiniteList, but hide it when collapsed -->
	<div class="flex-1 overflow-hidden" class:hidden={!manager.isSidebarExpanded}>
		<InfiniteList
			bind:this={manager.conversationListComponent}
			bind:items={manager.conversations}
			selectedItemId={manager.selectedConversationId}
			noBorder={true}
			rounded={false}
		>
			{#snippet children({ item: conversation, hover })}
				<div
					class={twMerge(
						'w-full p-1',
						manager.selectedConversationId === conversation.id
							? 'bg-blue-200/30 text-blue-500 dark:bg-blue-600/30 text-blue-400'
							: ''
					)}
				>
					<Button
						color="transparent"
						size="xs"
						onclick={() => manager.selectConversation(conversation.id, conversation.isDraft)}
					>
						<span class="flex-1 text-left text-sm font-medium text-primary truncate">
							{getConversationTitle(conversation)}
						</span>
						<button
							class="ml-2 p-1 rounded hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500 transition-all {hover ||
							manager.deletingConversationId === conversation.id
								? 'opacity-100'
								: 'opacity-0'}"
							disabled={manager.deletingConversationId === conversation.id}
							onclick={(e) => {
								e.stopPropagation()
								if (conversation.isDraft) {
									// just remove first conversation as it is the draft
									manager.conversations = [...manager.conversations.slice(1)]
								} else {
									manager.conversationListComponent?.deleteItem(conversation.id)
								}
							}}
							title="Delete conversation"
						>
							{#if manager.deletingConversationId === conversation.id}
								<Loader2 size={14} class="animate-spin" />
							{:else}
								<Trash2 size={14} />
							{/if}
						</button>
					</Button>
				</div>
			{/snippet}

			{#snippet empty()}
				<div class="p-4 text-center">
					<p class="text-sm text-secondary mb-2">No conversations yet</p>
				</div>
			{/snippet}
		</InfiniteList>
	</div>

	{#if manager.isSidebarExpanded}
		<!-- Footer -->
		<div class="flex-shrink-0 p-4 border-t border-gray-200 dark:border-gray-700">
			<p class="text-xs text-primary">
				{manager.conversations.length} conversation{manager.conversations.length !== 1 ? 's' : ''}
			</p>
		</div>
	{/if}
</div>
