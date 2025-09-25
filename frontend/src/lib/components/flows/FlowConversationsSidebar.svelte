<script lang="ts">
	import { Button } from '$lib/components/common'
	import { MessageCircle, Plus, Trash2, Clock } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import TimeAgo from '$lib/components/TimeAgo.svelte'

	interface FlowConversation {
		id: string
		workspace_id: string
		flow_path: string
		title?: string
		created_at: string
		updated_at: string
		created_by: string
	}

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

	async function loadConversations() {
		if (!$workspaceStore || !flowPath) return

		loading = true
		try {
			// TODO: Replace with actual API call once backend is implemented
			// const response = await ConversationService.listConversations({
			//   workspace: $workspaceStore,
			//   flowPath
			// })
			// conversations = response

			// For now, show empty state
			conversations = []
		} catch (error) {
			console.error('Failed to load conversations:', error)
			conversations = []
		} finally {
			loading = false
		}
	}

	async function deleteConversation(conversationId: string, event: Event) {
		event.stopPropagation()

		try {
			// TODO: Replace with actual API call once backend is implemented
			// await ConversationService.deleteConversation({
			//   workspace: $workspaceStore!,
			//   conversationId
			// })

			conversations = conversations.filter((c) => c.id !== conversationId)
			onDeleteConversation(conversationId)
		} catch (error) {
			console.error('Failed to delete conversation:', error)
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

<div class="flex flex-col h-full bg-surface border-r border-gray-200 dark:border-gray-700">
	<!-- Header -->
	<div class="flex-shrink-0 p-4 border-b border-gray-200 dark:border-gray-700">
		<div class="flex items-center justify-between mb-4">
			<div class="flex items-center gap-2">
				<MessageCircle size={20} class="text-tertiary" />
				<h3 class="text-sm font-medium text-primary">Conversations</h3>
			</div>
			<Button
				size="xs"
				color="light"
				startIcon={{ icon: Plus }}
				onclick={onNewConversation}
				title="Start new conversation"
			/>
		</div>
	</div>

	<!-- Conversations List -->
	<div class="flex-1 overflow-y-auto">
		{#if loading}
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
					<button
						class="w-full p-3 rounded-md text-left hover:bg-surface-hover transition-colors mb-2 group
							{selectedConversationId === conversation.id
							? 'bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700'
							: 'border border-transparent'}"
						onclick={() => onSelectConversation(conversation.id)}
					>
						<div class="flex items-start justify-between">
							<div class="flex-1 min-w-0">
								<p class="text-sm font-medium text-primary truncate">
									{getConversationTitle(conversation)}
								</p>
								<div class="flex items-center gap-1 mt-1">
									<Clock size={12} class="text-tertiary" />
									<span class="text-xs text-tertiary">
										<TimeAgo date={conversation.updated_at} />
									</span>
								</div>
							</div>
							<!-- <button
								class="opacity-0 group-hover:opacity-100 ml-2 p-1 rounded hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500 transition-all"
								onclick={(e) => deleteConversation(conversation.id, e)}
								title="Delete conversation"
							>
								<Trash2 size={14} />
							</button> -->
						</div>
					</button>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Footer -->
	<div class="flex-shrink-0 p-4 border-t border-gray-200 dark:border-gray-700">
		<p class="text-xs text-tertiary">
			{conversations.length} conversation{conversations.length !== 1 ? 's' : ''}
		</p>
	</div>
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
