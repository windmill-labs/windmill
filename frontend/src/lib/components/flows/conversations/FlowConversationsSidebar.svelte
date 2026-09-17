<script lang="ts">
	import { Button } from '$lib/components/common'
	import {
		MessageCircle,
		Plus,
		Trash2,
		Pen,
		Filter,
		PanelLeftClose,
		PanelLeftOpen
	} from 'lucide-svelte'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
	import InfiniteList from '$lib/components/InfiniteList.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { Item } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { fade } from 'svelte/transition'
	import { tick, untrack } from 'svelte'
	import type { Chat, ChatState, Conversation, ConversationKind } from 'windmill-chat'

	interface Props {
		chat: Chat
		chatState: ChatState
		/**
		 * Which conversations the list holds at first. The editor shows its own test chats,
		 * since testing is what happens there; a deployed flow shows the chats its users
		 * started, so nobody's trial runs are mixed into them.
		 */
		defaultKind?: ConversationKind
		/**
		 * Whether the filter is offered. Only the editor does: a deployed flow has no test
		 * chats of its own to show, and offering to list someone's trial runs there would
		 * put editor scratch in front of the flow's users.
		 */
		canFilterKind?: boolean
	}

	let { chat, chatState, defaultKind = 'deployed', canFilterKind = false }: Props = $props()

	let expanded = $state(false)
	let list = $state<InfiniteList | undefined>(undefined)
	let items = $state<Conversation[]>([])
	let deletingId = $state<string | undefined>(undefined)
	// A conversation exists on the server only once its first turn ran, so "New chat"
	// shows a draft row until then.
	let draft = $state(false)
	// The prop seeds the filter; the filter is then the user's.
	let kind = $state<ConversationKind>(untrack(() => defaultKind))

	// The chat being renamed, and the text typed so far. One at a time: the input is the
	// row's own label, so a second one would have nowhere to go.
	let renamingId = $state<string | undefined>(undefined)
	let renameDraft = $state('')
	let renameInput = $state<TextInput | undefined>(undefined)

	const turnInFlight = $derived(
		chatState.status === 'submitted' || chatState.status === 'streaming'
	)

	$effect(() => {
		const l = list
		const c = chat
		if (!l) return
		untrack(() => {
			// Every load goes through here, the first one and infinite scroll included. A
			// response for a kind no longer selected keeps the rows shown: the load for the
			// selected kind brings its own, whichever of the two lands last.
			l.setLoader(async (page, perPage) => {
				const requested = kind
				const rows = await c.loadConversations({ page, perPage, kind: requested })
				return requested === kind ? rows : items
			})
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

	/**
	 * The container reports a started turn: a conversation's first one creates its server
	 * entry. A new conversation is of this surface's own kind, so a filter that would not
	 * list it goes back to that kind rather than hiding the chat that was just started.
	 */
	export async function conversationStarted(conversationId: string) {
		if (items.some((c) => c.id === conversationId)) return
		draft = false
		if (kind !== 'all' && kind !== defaultKind) kind = defaultKind
		await list?.loadData('forceRefresh')
	}

	const draftShown = $derived(draft && !items.some((c) => c.id === chatState.conversationId))

	function newChat() {
		chat.newConversation()
		draft = true
	}

	const KIND_LABELS: Record<ConversationKind, string> = {
		test: 'Test',
		deployed: 'Deployed',
		all: 'All'
	}

	/**
	 * Narrow the list to one kind of chat and reload it. The open conversation goes with it
	 * when it is not of the new kind: the composer sends into whatever is selected, and a
	 * conversation keeps the kind it was created with, so a turn sent into one the list no
	 * longer shows would be stored where nothing here lists it.
	 */
	async function setKind(next: ConversationKind) {
		// A turn writes into the open conversation, which a kind that excludes it would close.
		if (next === kind || turnInFlight) return
		kind = next
		const open = items.find((c) => c.id === chatState.conversationId)
		const stillListed = open === undefined || next === 'all' || (next === 'test') === open.isTest
		if (!stillListed) chat.newConversation()
		await list?.loadData('forceRefresh')
	}

	async function startRename(conversation: Conversation) {
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
		if (!id) return
		const title = renameDraft.trim()
		const current = items.find((c) => c.id === id)
		if (!current || title === '' || title === current.title) return
		try {
			await chat.renameConversation(id, title)
			// The list holds its own rows, loaded through the loader: patched rather than
			// reloaded, so the row keeps its place without a round trip. The title is read
			// back from the chat, which holds it as the server stored it (a long one is cut).
			const stored = chat.getState().conversations.find((c) => c.id === id)?.title ?? title
			items = items.map((c) => (c.id === id ? { ...c, title: stored } : c))
		} catch (error) {
			console.error('Failed to rename conversation:', error)
			sendUserToast('Failed to rename conversation', true)
		}
	}

	function rowActions(conversation: Conversation): Item[] {
		return [
			{ displayName: 'Rename', icon: Pen, action: () => startRename(conversation) },
			{
				displayName: 'Delete',
				icon: Trash2,
				type: 'delete',
				disabled: deletingId === conversation.id,
				action: () => list?.deleteItem(conversation.id)
			}
		]
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
			<!-- Side by side while there is width for both; stacked once collapsed, where the
			     rail fits one icon across. -->
			<div class={expanded ? 'flex flex-row gap-1 items-center' : 'flex flex-col gap-2'}>
				<Button
					unifiedSize="md"
					variant="subtle"
					startIcon={{ icon: Plus, classes: 'ml-[2px]' }}
					onClick={newChat}
					title="Start new conversation"
					iconOnly={!expanded}
					wrapperClasses={expanded ? 'grow min-w-0' : ''}
					btnClasses={'w-full justify-start transition-all duration-150 whitespace-nowrap'}
				>
					<div transition:fade={{ duration: 100 }}> New chat </div>
				</Button>
				{#if canFilterKind}
					<!-- No focus trap: opening a row's menu does not close this popover, and a
					     trapped popover pulls focus back from the rename field that menu opens. -->
					<Popover
						placement="bottom-start"
						closeButton={false}
						disableFocusTrap
						disabled={turnInFlight}
					>
						{#snippet trigger()}
							<!-- Icon-only next to the wider New chat: which kind is listed is named in
							     the title and by the group inside. -->
							<Button
								nonCaptureEvent
								unifiedSize="md"
								variant="subtle"
								startIcon={{ icon: Filter }}
								disabled={turnInFlight}
								title={turnInFlight
									? 'Wait for the current answer to change which chats are listed'
									: `Filter conversations · ${KIND_LABELS[kind]}`}
								iconOnly
							/>
						{/snippet}
						{#snippet content()}
							<div class="p-3">
								<ToggleButtonGroup
									selected={kind}
									onSelected={(next) => setKind(next as ConversationKind)}
									disabled={turnInFlight}
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
						{#if renamingId === conversation.id}
							<!-- While renaming, the field replaces the row rather than sitting inside its
							     button: a text input nested in a button is a nested interactive control,
							     and every keystroke would have to be kept from reaching the row. -->
							<div class="flex flex-row items-center h-8 px-2 rounded-md bg-surface-selected">
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
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div
									class={twMerge(
										'ml-2 transition-all duration-100 opacity-0 group-hover:opacity-100',
										deletingId === conversation.id ? 'opacity-100' : ''
									)}
									onclick={(e) => e.stopPropagation()}
								>
									<DropdownV2 items={() => rowActions(conversation)} size="xs" />
								</div>
							</Button>
						{/if}
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
