<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import AssistantMessage from './AssistantMessage.svelte'
	import { type Snippet } from 'svelte'
	import {
		CheckIcon,
		EditIcon,
		HistoryIcon,
		Loader2,
		Plus,
		RefreshCwIcon,
		StopCircleIcon,
		Undo2Icon,
		X,
		XIcon
	} from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { type DisplayMessage } from './shared'
	import type { ContextElement } from './context'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import ContextTextarea from './ContextTextarea.svelte'
	import AvailableContextList from './AvailableContextList.svelte'
	import ChatQuickActions from './ChatQuickActions.svelte'
	import ProviderModelSelector from './ProviderModelSelector.svelte'
	import ChatMode from './ChatMode.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'

	let {
		messages,
		pastChats,
		hasDiff,
		diffMode = false, // todo: remove default
		selectedContext = $bindable([]), // todo: remove default
		availableContext = [], // todo: remove default
		loadPastChat,
		deletePastChat,
		saveAndClear,
		cancel,
		askAi = () => {}, // todo: remove default,
		headerLeft,
		headerRight,
		disabled = false,
		disabledMessage = '',
		suggestions = []
	}: {
		messages: DisplayMessage[]
		pastChats: { id: string; title: string }[]
		hasDiff?: boolean
		diffMode: boolean
		selectedContext: ContextElement[]
		availableContext: ContextElement[]
		loadPastChat: (id: string) => void
		deletePastChat: (id: string) => void
		saveAndClear: () => void
		cancel: () => void
		askAi?: (instructions: string, options?: { withCode?: boolean; withDiff?: boolean }) => void
		headerLeft?: Snippet
		headerRight?: Snippet
		disabled?: boolean
		disabledMessage?: string
		suggestions?: string[]
	} = $props()

	let contextTextareaComponent: ContextTextarea | undefined = $state()
	let instructionsTextarea: HTMLTextAreaElement | undefined = $state()
	let editingMessageIndex = $state<number | null>(null)
	let editingMessageContent = $state<string>('')

	export function focusInput() {
		if (aiChatManager.mode === 'script') {
			contextTextareaComponent?.focus()
		} else {
			instructionsTextarea?.focus()
		}
	}

	let scrollEl: HTMLDivElement | undefined = $state()
	async function scrollDown() {
		scrollEl?.scrollTo({
			top: scrollEl.scrollHeight,
			behavior: 'smooth'
		})
	}

	let height = $state(0)
	$effect(() => {
		aiChatManager.automaticScroll && height && scrollDown()
	})

	function addContextToSelection(contextElement: ContextElement) {
		if (
			selectedContext &&
			availableContext &&
			!selectedContext.find(
				(c) => c.type === contextElement.type && c.title === contextElement.title
			) &&
			availableContext.find(
				(c) => c.type === contextElement.type && c.title === contextElement.title
			)
		) {
			selectedContext = [...selectedContext, contextElement]
		}
	}

	function submitSuggestion(suggestion: string) {
		aiChatManager.instructions = suggestion
		aiChatManager.sendRequest()
	}

	function isLastUserMessage(messageIndex: number): boolean {
		// Find the last user message index
		for (let i = messages.length - 1; i >= 0; i--) {
			if (messages[i].role === 'user') {
				return i === messageIndex
			}
		}
		return false
	}

	function restartGeneration(messageIndex: number) {
		aiChatManager.restartLastGeneration(messageIndex)
	}

	function startEditMessage(messageIndex: number) {
		editingMessageIndex = messageIndex
		editingMessageContent = messages[messageIndex].content
	}

	function cancelEditMessage() {
		editingMessageIndex = null
		editingMessageContent = ''
	}

	function saveEditMessage() {
		if (editingMessageIndex !== null && editingMessageContent.trim()) {
			aiChatManager.editMessage(editingMessageIndex, editingMessageContent.trim())
			editingMessageIndex = null
			editingMessageContent = ''
		}
	}
</script>

<div class="flex flex-col h-full">
	<div
		class="flex flex-row items-center justify-between gap-2 p-2 border-b border-gray-200 dark:border-gray-600"
	>
		<div class="flex flex-row items-center gap-2">
			{@render headerLeft?.()}
			<p class="text-sm font-semibold">Chat</p>
		</div>
		<div class="flex flex-row items-center gap-2">
			<Popover>
				<svelte:fragment slot="trigger">
					<Button
						on:click={() => {}}
						title="History"
						size="md"
						btnClasses="!p-1"
						startIcon={{ icon: HistoryIcon }}
						iconOnly
						variant="border"
						color="light"
						propagateEvent
					/>
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<div class="p-1 overflow-y-auto max-h-[300px]">
						{#if pastChats.length === 0}
							<div class="text-center text-tertiary text-xs">No history</div>
						{:else}
							<div class="flex flex-col">
								{#each pastChats as chat}
									<button
										class="text-left flex flex-row items-center gap-2 justify-between hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md p-1"
										onclick={() => {
											loadPastChat(chat.id)
											close()
										}}
									>
										<div
											class="text-xs font-medium w-48 text-ellipsis overflow-hidden whitespace-nowrap flex-1"
											title={chat.title}
										>
											{chat.title}
										</div>
										<Button
											iconOnly
											size="xs2"
											btnClasses="!p-1"
											color="light"
											variant="border"
											startIcon={{ icon: X }}
											on:click={() => {
												deletePastChat(chat.id)
											}}
										/>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				</svelte:fragment>
			</Popover>
			<Button
				title="New chat"
				on:click={() => {
					saveAndClear()
				}}
				size="md"
				btnClasses="!p-1"
				startIcon={{ icon: Plus }}
				iconOnly
				variant="border"
				color="light"
			/>
			{@render headerRight?.()}
		</div>
	</div>
	{#if messages.length > 0}
		<div
			class="h-full overflow-y-scroll pt-2 pb-12"
			bind:this={scrollEl}
			onwheel={() => {
				aiChatManager.disableAutomaticScroll()
			}}
		>
			<div class="flex flex-col" bind:clientHeight={height}>
				{#each messages as message, messageIndex}
					<div class={twMerge(message.role === 'user' && messageIndex > 0 && 'mt-6', 'mb-2')}>
						{#if message.role === 'user' && message.contextElements}
							<div class="flex flex-row gap-1 mb-1 overflow-scroll no-scrollbar px-2">
								{#each message.contextElements as element}
									<ContextElementBadge contextElement={element} />
								{/each}
							</div>
						{/if}
						<div
							class={twMerge(
								'text-sm py-1 mx-2',
								message.role === 'user' &&
									'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg relative group',
								(message.role === 'assistant' || message.role === 'tool') && 'px-[1px]',
								message.role === 'tool' && 'text-tertiary'
							)}
						>
							{#if message.role === 'assistant'}
								<AssistantMessage {message} />
							{:else if message.role === 'user' && editingMessageIndex === messageIndex}
								<textarea
									bind:value={editingMessageContent}
									use:autosize
									onkeydown={(e) => {
										if (e.key === 'Enter' && !e.shiftKey) {
											e.preventDefault()
											saveEditMessage()
										} else if (e.key === 'Escape') {
											e.preventDefault()
											cancelEditMessage()
										}
									}}
									class="w-full resize-none bg-transparent border-none outline-none"
									placeholder="Edit message..."
									rows={3}
								></textarea>
								<div class="flex justify-end gap-1 mt-1">
									<Button
										size="xs2"
										variant="border"
										color="light"
										on:click={cancelEditMessage}
										title="Cancel editing"
									>
										Cancel
									</Button>
									<Button
										size="xs2"
										variant="border"
										color="blue"
										on:click={saveEditMessage}
										title="Save and send"
									>
										Send
									</Button>
								</div>
							{:else}
								{message.content}
							{/if}

							{#if message.role === 'user' && editingMessageIndex !== messageIndex && !aiChatManager.loading}
								<div
									class="absolute top-1 right-1 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1"
								>
									<Button
										size="xs2"
										variant="border"
										color="light"
										iconOnly
										title="Edit message"
										startIcon={{ icon: EditIcon }}
										btnClasses="!p-1 !h-6 !w-6"
										on:click={() => startEditMessage(messageIndex)}
									/>
									{#if isLastUserMessage(messageIndex)}
										<Button
											size="xs2"
											variant="border"
											color="light"
											iconOnly
											title="Restart generation"
											startIcon={{ icon: RefreshCwIcon }}
											btnClasses="!p-1 !h-6 !w-6"
											on:click={() => restartGeneration(messageIndex)}
										/>
									{/if}
								</div>
							{/if}
						</div>
						{#if message.role === 'user' && message.snapshot}
							<div
								class="mx-2 text-sm text-tertiary flex flex-row items-center justify-between gap-2 mt-2"
							>
								Saved a flow snapshot
								<Button
									size="xs2"
									variant="border"
									color="light"
									on:click={() => {
										if (message.snapshot) {
											aiChatManager.flowAiChatHelpers?.revertToSnapshot(message.snapshot)
										}
									}}
									title="Revert to snapshot"
									startIcon={{ icon: Undo2Icon }}
								>
									Revert
								</Button>
							</div>
						{/if}
					</div>
				{/each}
				{#if aiChatManager.loading && !aiChatManager.currentReply}
					<div class="mb-6 py-1 px-2">
						<Loader2 class="animate-spin" />
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<div class:border-t={messages.length > 0} class="relative">
		{#if aiChatManager.loading}
			<div class="absolute -top-10 w-full flex flex-row justify-center">
				<Button
					startIcon={{ icon: StopCircleIcon }}
					size="xs"
					variant="border"
					color="light"
					on:click={() => {
						cancel()
					}}
				>
					Stop
				</Button>
			</div>
		{:else if aiChatManager.flowAiChatHelpers?.hasDiff()}
			<div class="absolute -top-10 w-full flex flex-row justify-center gap-2">
				<Button
					startIcon={{ icon: CheckIcon }}
					size="xs"
					variant="border"
					color="light"
					btnClasses="bg-green-500 hover:bg-green-600 text-white hover:text-white"
					on:click={() => {
						aiChatManager.flowAiChatHelpers?.acceptAllModuleActions()
					}}
				>
					Accept all
				</Button>
				<Button
					startIcon={{ icon: XIcon }}
					size="xs"
					variant="border"
					color="light"
					btnClasses="dark:opacity-50 opacity-60 hover:opacity-100"
					on:click={() => {
						aiChatManager.flowAiChatHelpers?.rejectAllModuleActions()
					}}
				>
					Reject all
				</Button>
			</div>
		{/if}
		{#if aiChatManager.mode === 'script'}
			<div class="flex flex-row gap-1 mb-1 overflow-scroll pt-2 px-2 no-scrollbar">
				<Popover>
					<svelte:fragment slot="trigger">
						<div
							class="border rounded-md px-1 py-0.5 font-normal text-tertiary text-xs hover:bg-surface-hover"
							>@</div
						>
					</svelte:fragment>
					<svelte:fragment slot="content" let:close>
						<AvailableContextList
							{availableContext}
							{selectedContext}
							onSelect={(element) => {
								addContextToSelection(element)
								close()
							}}
						/>
					</svelte:fragment>
				</Popover>
				{#each selectedContext as element}
					<ContextElementBadge
						contextElement={element}
						deletable
						on:delete={() => {
							selectedContext = selectedContext?.filter(
								(c) => c.type !== element.type || c.title !== element.title
							)
						}}
					/>
				{/each}
			</div>
			<ContextTextarea
				bind:this={contextTextareaComponent}
				{availableContext}
				{selectedContext}
				isFirstMessage={messages.length === 0}
				onAddContext={(contextElement) => addContextToSelection(contextElement)}
				onSendRequest={() => {
					if (!aiChatManager.loading) {
						aiChatManager.sendRequest()
					}
				}}
				onUpdateInstructions={(value) => (aiChatManager.instructions = value)}
				{disabled}
			/>
		{:else}
			<div class="relative w-full px-2 scroll-pb-2 pt-2">
				<textarea
					bind:this={instructionsTextarea}
					bind:value={aiChatManager.instructions}
					use:autosize
					onkeydown={(e) => {
						if (e.key === 'Enter' && !e.shiftKey && !aiChatManager.loading) {
							e.preventDefault()
							aiChatManager.sendRequest()
						}
					}}
					rows={3}
					placeholder={messages.length === 0 ? 'Ask anything' : 'Ask followup'}
					class="resize-none"
					{disabled}
				></textarea>
			</div>
		{/if}
		<div
			class={`flex flex-row ${
				aiChatManager.mode === 'script' && hasDiff ? 'justify-between' : 'justify-end'
			} items-center px-0.5`}
		>
			{#if aiChatManager.mode === 'script' && hasDiff}
				<ChatQuickActions {askAi} {diffMode} />
			{/if}
			{#if disabled}
				<div class="text-tertiary text-xs my-2 px-2">
					<Markdown md={disabledMessage} />
				</div>
			{:else}
				<div class="flex flex-row gap-2 min-w-0">
					<ChatMode />
					<ProviderModelSelector />
				</div>
			{/if}
		</div>
		{#if (aiChatManager.mode === AIMode.NAVIGATOR || aiChatManager.mode === AIMode.ASK) && suggestions.length > 0 && messages.filter((m) => m.role === 'user').length === 0 && !disabled}
			<div class="px-2 mt-4">
				<div class="flex flex-col gap-2">
					{#each suggestions as suggestion}
						<Button
							on:click={() => submitSuggestion(suggestion)}
							size="xs2"
							color="light"
							btnClasses="whitespace-normal text-center font-normal"
						>
							{suggestion}
						</Button>
					{/each}
				</div>
			</div>
		{/if}
	</div>
</div>
