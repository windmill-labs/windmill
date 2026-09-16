<script lang="ts">
	import { Button } from '$lib/components/common'
	import { MessageCircle, Plus, Trash2, PanelLeftClose, PanelLeftOpen } from 'lucide-svelte'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import { sendUserToast } from '$lib/toast'
	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import { untrack } from 'svelte'
	import type { Chat, ChatState, Conversation } from 'windmill-chat'

	interface Props {
		chat: Chat
		chatState: ChatState
	}

	let { chat, chatState }: Props = $props()

	let expanded = $state(false)
	let list = $state<InfiniteList | undefined>(undefined)
	let items = $state<Conversation[]>([])
	let deletingId = $state<string | undefined>(undefined)
	// A conversation exists on the server only once its first turn ran, so "New chat"
	// shows a draft row until then.
	let draft = $state(false)

	$effect(() => {
		const l = list
		const c = chat
		if (!l) return
		untrack(() => {
			l.setLoader((page, perPage) => c.loadConversations({ page, perPage }))
			l.setDeleteItemFn(async (id: string) => {
				deletingId = id
				try {
					await c.deleteConversation(id)
					sendUserToast('Conversation deleted successfully')
				} catch (error) {
					console.error('Failed to delete conversation:', error)
					sendUserToast('Failed to delete conversation', true)
					throw error
				} finally {
					deletingId = undefined
				}
			})
		})
	})

	/** The container reports a started turn: a conversation's first one creates its server entry. */
	export async function conversationStarted(conversationId: string) {
		if (items.some((c) => c.id === conversationId)) return
		draft = false
		await list?.loadData('forceRefresh')
	}

	const draftShown = $derived(draft && !items.some((c) => c.id === chatState.conversationId))

	function newChat() {
		chat.newConversation()
		draft = true
	}

	function getConversationTitle(conversation: Conversation): string {
		return conversation.title || `Conversation ${conversation.createdAt.slice(0, 10)}`
	}
</script>

<div
	class="flex flex-col h-full bg-surface border-r transition-all duration-300 {expanded
		? 'w-60'
		: 'w-[44px]'}"
>
	<!-- Header -->
	<div class="flex-shrink-0 border-b">
		<div class="flex flex-col gap-2 p-1">
			<Button
				unifiedSize="md"
				variant="subtle"
				startIcon={{
					icon: expanded ? PanelLeftClose : PanelLeftOpen,
					classes: 'ml-[2px]'
				}}
				onClick={() => (expanded = !expanded)}
				iconOnly={!expanded}
				btnClasses={'justify-start transition-all duration-150'}
				title="Conversations"
			>
				<div transition:fade={{ duration: 100 }}> Conversations </div>
			</Button>
			<Button
				unifiedSize="md"
				variant="subtle"
				startIcon={{ icon: Plus, classes: 'ml-[2px]' }}
				onClick={newChat}
				title="Start new conversation"
				iconOnly={!expanded}
				btnClasses={'justify-start transition-all duration-150 whitespace-nowrap'}
			>
				<div transition:fade={{ duration: 100 }}> New chat </div>
			</Button>
		</div>
	</div>

	<!-- Conversations List -->
	{#if !expanded}
		<!-- Collapsed state - show single chat icon with badge -->
		<div class="p-1">
			<Button
				unifiedSize="md"
				startIcon={{ icon: MessageCircle }}
				onClick={() => (expanded = true)}
				title="{items.length} conversation{items.length !== 1 ? 's' : ''}"
				variant="subtle"
				btnClasses="w-fit px-2 relative"
			>
				<CountBadge count={items.length} small alwaysVisible={true} class="right-[3px] top-[3px]" />
			</Button>
		</div>
	{/if}

	<!-- Always mount InfiniteList, but hide it when collapsed -->
	<div class="flex-1 overflow-hidden transition-all duration-150 p-1" class:hidden={!expanded}>
		{#if draftShown && expanded}
			<div class="w-full pb-1" transition:fade={{ duration: 100, delay: 30 }}>
				<Button
					unifiedSize="md"
					variant="subtle"
					selected={true}
					btnClasses="transition-all duration-150 group"
				>
					<span class="flex-1 text-left truncate">New chat</span>
					<Button
						wrapperClasses="ml-2 transition-all duration-100 opacity-0 group-hover:opacity-100"
						onClick={(e) => {
							e?.stopPropagation()
							draft = false
							chat.newConversation()
						}}
						title="Discard draft"
						destructive
						unifiedSize="xs"
						variant="subtle"
						iconOnly
						startIcon={{ icon: Trash2 }}
					/>
				</Button>
			</div>
		{/if}
		<InfiniteList
			bind:this={list}
			bind:items
			selectedItemId={chatState.conversationId}
			noBorder={true}
			rounded={false}
			preventXOverflow={true}
		>
			{#snippet customRow({ item: conversation })}
				{#if expanded}
					<div class={twMerge('w-full pb-1')} transition:fade={{ duration: 100, delay: 30 }}>
						<Button
							unifiedSize="md"
							variant="subtle"
							onClick={() => {
								draft = false
								chat.selectConversation(conversation.id)
							}}
							selected={chatState.conversationId === conversation.id}
							btnClasses="transition-all duration-150 group"
						>
							<span class="flex-1 text-left truncate">
								{getConversationTitle(conversation)}
							</span>
							<Button
								wrapperClasses={twMerge(
									'ml-2 transition-all duration-100  opacity-0 group-hover:opacity-100',
									deletingId === conversation.id ? 'opacity-100' : ' '
								)}
								disabled={deletingId === conversation.id}
								onClick={(e) => {
									e?.stopPropagation()
									list?.deleteItem(conversation.id)
								}}
								title="Delete conversation"
								destructive
								unifiedSize="xs"
								variant="subtle"
								loading={deletingId === conversation.id}
								iconOnly
								startIcon={{ icon: Trash2 }}
							/>
						</Button>
					</div>
				{/if}
			{/snippet}

			{#snippet empty()}
				{#if !draftShown}
					<div class="p-4 text-center">
						<p class="text-sm text-secondary mb-2">No conversations yet</p>
					</div>
				{/if}
			{/snippet}
		</InfiniteList>
	</div>
</div>
