<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { tick } from 'svelte'
	import {
		MessageCircle,
		Pen,
		Plus,
		Trash2,
		PanelLeftClose,
		PanelLeftOpen,
		FlaskConical
	} from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import type { Item } from '$lib/utils'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Filter } from 'lucide-svelte'
	import { type FlowConversation } from '$lib/gen'
	import CountBadge from '$lib/components/common/badge/CountBadge.svelte'
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
	}

	let { manager }: Props = $props()

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

	/** A turn is running and this is not the chat it is running in. */
	function rowLocked(conversation: ConversationWithDraft): boolean {
		return manager.isTurnInFlight && manager.selectedConversationId !== conversation.id
	}

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
					disabled={manager.isTurnInFlight}
					title={manager.isTurnInFlight
						? 'Wait for the current answer to start a new chat'
						: 'Start new conversation'}
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
						{#if renamingId === conversation.id}
							<!-- While renaming, the field replaces the row rather than sitting inside its
							     button: a text input nested in a button is a nested interactive control,
							     and every keystroke would have to be kept from reaching the row. -->
							<div class="flex flex-row items-center gap-1 h-8 px-2 rounded-md bg-surface-selected">
								{#if conversation.is_test}
									<FlaskConical size={12} class="shrink-0 text-tertiary" />
								{/if}
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
								onClick={() => manager.selectConversation(conversation.id, conversation.isDraft)}
								selected={manager.selectedConversationId === conversation.id}
								disabled={rowLocked(conversation)}
								title={rowLocked(conversation)
									? 'Wait for the current answer to switch conversation'
									: undefined}
								btnClasses="transition-all duration-150 group"
							>
								{#if conversation.is_test}
									<!-- The list holds both kinds under the "All" filter, so a test chat has to
									     be readable as one at a glance. -->
									<FlaskConical size={12} class="shrink-0 mr-1 text-tertiary" />
								{/if}
								<span class="flex-1 text-left truncate">
									{getConversationTitle(conversation)}
								</span>
								<!-- Hidden while the row is disabled: it sits inside the row's button, and a
								     disabled button swallows every click in its subtree, so a visible menu
								     here would be an affordance that does nothing. -->
								{#if !rowLocked(conversation)}
									<!-- svelte-ignore a11y_click_events_have_key_events -->
									<!-- svelte-ignore a11y_no_static_element_interactions -->
									<div
										class={twMerge(
											'ml-2 transition-all duration-100 opacity-0 group-hover:opacity-100',
											manager.deletingConversationId === conversation.id ? 'opacity-100' : ''
										)}
										onclick={(e) => e.stopPropagation()}
									>
										<DropdownV2 items={() => rowActions(conversation)} size="xs" />
									</div>
								{/if}
							</Button>
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
