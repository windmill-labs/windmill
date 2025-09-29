<script lang="ts">
	import { Button } from '$lib/components/common'
	import { MessageCircle, Plus, Clock, Trash2, Menu } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { FlowConversationService, type FlowConversation } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		flowPath: string
		selectedConversationId?: string
		onNewConversation: () => void
		onSelectConversation: (conversationId: string) => void
		onDeleteConversation: (conversationId: string) => void
	}

	let {
		flowPath,
		selectedConversationId,
		onNewConversation,
		onSelectConversation,
		onDeleteConversation
	}: Props = $props()

	let conversations = $state<FlowConversation[]>([])
	let loading = $state(false)
	let isExpanded = $state(false)

	export async function refreshConversations() {
		return await loadConversations()
	}

	async function loadConversations() {
		if (!$workspaceStore || !flowPath) return

		loading = true
		try {
			const response = await FlowConversationService.listFlowConversations({
				workspace: $workspaceStore,
				flowPath: flowPath
			})
			conversations = response
			loading = false
			return conversations
		} catch (error) {
			console.error('Failed to load conversations:', error)
			sendUserToast('Failed to load conversations', true)
			conversations = []
			loading = false
			return []
		}
	}

	async function deleteConversation(conversationId: string, event: Event) {
		event.stopPropagation()

		try {
			await FlowConversationService.deleteFlowConversation({
				workspace: $workspaceStore!,
				conversationId
			})

			conversations = conversations.filter((c) => c.id !== conversationId)
			onDeleteConversation(conversationId)
			sendUserToast('Conversation deleted successfully')
		} catch (error) {
			console.error('Failed to delete conversation:', error)
			sendUserToast('Failed to delete conversation', true)
		}
	}

	function getConversationTitle(conversation: FlowConversation): string {
		return conversation.title || `Conversation ${conversation.created_at.slice(0, 10)}`
	}

	// Load conversations when component mounts or flowPath changes
	$effect(() => {
		if ($workspaceStore && flowPath) {
			loadConversations()
		}
	})
</script>

<div
	class="flex flex-col h-full bg-surface border-r border-gray-200 dark:border-gray-700 transition-all duration-300 {isExpanded
		? 'w-60'
		: 'w-16'}"
>
	<!-- Header -->
	<div class="flex-shrink-0 p-2 border-b border-gray-200 dark:border-gray-700">
		<div class="flex flex-col gap-2">
			<Button
				color="light"
				startIcon={{ icon: Menu }}
				onclick={() => (isExpanded = !isExpanded)}
				iconOnly={!isExpanded}
				btnClasses="!w-auto"
			>
				{#if isExpanded}Conversations{/if}
			</Button>
			<Button
				color="light"
				startIcon={{ icon: Plus }}
				onclick={onNewConversation}
				title="Start new conversation"
				iconOnly={!isExpanded}
				btnClasses="!w-auto"
			>
				{#if isExpanded}New chat{/if}
			</Button>
		</div>
	</div>

	<!-- Conversations List -->
	<div class="flex-1 overflow-y-auto">
		{#if !isExpanded}
			<!-- Collapsed state - show chat icons -->
			<div class="p-2 flex flex-col gap-2 items-center">
				{#each conversations as conversation (conversation.id)}
					<Button
						color="light"
						btnClasses={selectedConversationId === conversation.id
							? 'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700'
							: ''}
						onclick={() => onSelectConversation(conversation.id)}
						title={getConversationTitle(conversation)}
						startIcon={{ icon: MessageCircle }}
						iconOnly
					/>
				{/each}
			</div>
		{:else if loading}
			<div class="p-4 text-center">
				<div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500 mx-auto"></div>
				<p class="text-sm text-tertiary mt-2">Loading conversations...</p>
			</div>
		{:else if conversations.length === 0}
			<div class="p-4 text-center">
				<MessageCircle size={48} class="mx-auto mb-4 opacity-30 text-tertiary" />
				<p class="text-sm text-secondary mb-2">No conversations yet</p>
				<p class="text-xs text-tertiary">Start a new conversation to get started</p>
			</div>
		{:else}
			<div class="p-2">
				{#each conversations as conversation (conversation.id)}
					<div
						class="w-full p-3 rounded-md text-left hover:bg-surface-hover transition-colors mb-2 group
							{selectedConversationId === conversation.id
							? 'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700'
							: 'border border-transparent'}"
						onclick={() => onSelectConversation(conversation.id)}
						role="button"
						tabindex="0"
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') {
								onSelectConversation(conversation.id)
							}
						}}
					>
						<div class="flex items-start justify-between">
							<div class="flex-1 min-w-0">
								<p class="text-sm font-medium text-primary truncate">
									{getConversationTitle(conversation)}
								</p>
							</div>
							<button
								class="opacity-0 group-hover:opacity-100 ml-2 p-1 rounded hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500 transition-all"
								onclick={(e) => deleteConversation(conversation.id, e)}
								title="Delete conversation"
							>
								<Trash2 size={14} />
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Footer -->
	{#if isExpanded}
		<div class="flex-shrink-0 p-4 border-t border-gray-200 dark:border-gray-700">
			<p class="text-xs text-tertiary">
				{conversations.length} conversation{conversations.length !== 1 ? 's' : ''}
			</p>
		</div>
	{/if}
</div>

<style>
	/* Custom scrollbar for conversations list */
	.overflow-y-auto::-webkit-scrollbar {
		width: 6px;
	}
	.overflow-y-auto::-webkit-scrollbar-track {
		background: transparent;
	}
	.overflow-y-auto::-webkit-scrollbar-thumb {
		background: rgba(156, 163, 175, 0.5);
		border-radius: 3px;
	}
	.overflow-y-auto::-webkit-scrollbar-thumb:hover {
		background: rgba(156, 163, 175, 0.7);
	}
</style>
