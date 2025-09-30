<script lang="ts">
	import { Button } from '$lib/components/common'
	import { MessageCircle, Plus, Trash2, Menu, PanelLeftClose, PanelLeftOpen } from 'lucide-svelte'
	import { workspaceStore } from '$lib/stores'
	import { FlowConversationService, type FlowConversation } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import { untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		flowPath: string
		selectedConversationId?: string
		onNewConversation: () => void
		onSelectConversation: (conversationId: string, isDraft?: boolean) => void
		onDeleteConversation: (conversationId: string) => void
	}

	interface ConversationWithDraft extends FlowConversation {
		isDraft?: boolean
	}

	let {
		flowPath,
		selectedConversationId,
		onNewConversation,
		onSelectConversation,
		onDeleteConversation
	}: Props = $props()

	let isExpanded = $state(false)
	let infiniteList: InfiniteList | undefined = $state()
	let conversations = $state<ConversationWithDraft[]>([])

	export async function refreshConversations() {
		return await infiniteList?.loadData('forceRefresh')
	}

	export async function addNewConversation(conversationId: string, username: string) {
		// Check if there's already a draft conversation
		const existingDraft = conversations.find((c) => c.isDraft)
		if (existingDraft) {
			// Select the existing draft instead of creating a new one
			onSelectConversation(existingDraft.id, true)
			return existingDraft.id
		}

		// Create a new conversation object and add it to the top of the list
		const newConversation: ConversationWithDraft = {
			id: conversationId,
			workspace_id: $workspaceStore!,
			flow_path: flowPath,
			title: 'New chat',
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString(),
			created_by: username,
			isDraft: true
		}

		// Prepend to conversations list
		conversations = [newConversation, ...conversations]
		return conversationId
	}

	async function loadConversations(page: number, perPage: number) {
		if (!$workspaceStore || !flowPath) return []

		try {
			const response = await FlowConversationService.listFlowConversations({
				workspace: $workspaceStore,
				flowPath: flowPath,
				page: page,
				perPage: perPage
			})

			return response
		} catch (error) {
			console.error('Failed to load conversations:', error)
			sendUserToast('Failed to load conversations', true)
			return []
		}
	}

	async function deleteConversation(conversationId: string) {
		try {
			await FlowConversationService.deleteFlowConversation({
				workspace: $workspaceStore!,
				conversationId
			})

			onDeleteConversation(conversationId)
			sendUserToast('Conversation deleted successfully')
		} catch (error) {
			console.error('Failed to delete conversation:', error)
			sendUserToast('Failed to delete conversation', true)
			throw error
		}
	}

	function getConversationTitle(conversation: FlowConversation): string {
		return conversation.title || `Conversation ${conversation.created_at.slice(0, 10)}`
	}

	// Initialize InfiniteList when component mounts or flowPath changes
	$effect(() => {
		if ($workspaceStore && flowPath && infiniteList) {
			untrack(() => {
				infiniteList?.setLoader(loadConversations)
				infiniteList?.setDeleteItemFn(deleteConversation)
			})
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
				size="sm"
				color="light"
				startIcon={{ icon: isExpanded ? PanelLeftClose : PanelLeftOpen }}
				onclick={() => (isExpanded = !isExpanded)}
				iconOnly={!isExpanded}
				btnClasses="!justify-start"
				label="Conversations"
			>
				Conversations
			</Button>
			<Button
				size="sm"
				color="light"
				startIcon={{ icon: Plus }}
				onclick={onNewConversation}
				title="Start new conversation"
				iconOnly={!isExpanded}
				btnClasses="!justify-start"
				label="New chat"
			>
				New chat
			</Button>
		</div>
	</div>

	<!-- Conversations List -->
	{#if !isExpanded}
		<!-- Collapsed state - show single chat icon with badge -->
		<div class="p-2 flex flex-col items-center mt-2">
			<button
				class="relative w-[23px] h-[23px] rounded-md center-center hover:bg-surface-hover transition-all duration-100 text-secondary hover:text-primary group"
				onclick={() => (isExpanded = true)}
				title="{conversations.length} conversation{conversations.length !== 1 ? 's' : ''}"
			>
				<MessageCircle size={16} />
				<CountBadge count={conversations.length} small={true} alwaysVisible={true} />
			</button>
		</div>
	{:else}
		<div class="flex-1 overflow-hidden">
			<InfiniteList
				bind:this={infiniteList}
				bind:items={conversations}
				selectedItemId={selectedConversationId}
				noBorder={true}
				rounded={false}
			>
				{#snippet children({ item: conversation, hover })}
					<div
						class={twMerge(
							'w-full p-1',
							selectedConversationId === conversation.id
								? 'bg-blue-200/30 text-blue-500 dark:bg-blue-600/30 text-blue-400'
								: ''
						)}
					>
						<Button
							color="transparent"
							size="xs"
							onclick={() => onSelectConversation(conversation.id, conversation.isDraft)}
						>
							<span class="flex-1 text-left text-sm font-medium text-primary truncate">
								{getConversationTitle(conversation)}
							</span>
							<button
								class="ml-2 p-1 rounded hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500 transition-all {hover
									? 'opacity-100'
									: 'opacity-0'}"
								onclick={(e) => {
									e.stopPropagation()
									infiniteList?.deleteItem(conversation.id)
								}}
								title="Delete conversation"
							>
								<Trash2 size={14} />
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

		<!-- Footer -->
		<div class="flex-shrink-0 p-4 border-t border-gray-200 dark:border-gray-700">
			<p class="text-xs text-tertiary">
				{conversations.length} conversation{conversations.length !== 1 ? 's' : ''}
			</p>
		</div>
	{/if}
</div>
