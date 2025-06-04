<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import AssistantMessage from './AssistantMessage.svelte'
	import { type Snippet } from 'svelte'
	import { HistoryIcon, Loader2, Plus, StopCircleIcon, X } from 'lucide-svelte'
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
	import { aiChatManager } from './AIChatManager.svelte'

	let {
		allowedModes,
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
		allowedModes: {
			script: boolean
			flow: boolean
			navigator: boolean
		}
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

	export function focusInput() {
		contextTextareaComponent?.focus()
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

	function findLastIndex<T>(array: T[], predicate: (item: T) => boolean): number {
		for (let i = array.length - 1; i >= 0; i--) {
			if (predicate(array[i])) {
				return i
			}
		}
		return -1
	}

	function submitSuggestion(suggestion: string) {
		aiChatManager.instructions = suggestion
		aiChatManager.sendRequest()
	}

	const lastUserMessageIndex = $derived(findLastIndex(messages, (m) => m.role === 'user'))
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
			class="h-full overflow-y-scroll pt-2"
			bind:this={scrollEl}
			onwheel={(e) => {
				aiChatManager.automaticScroll = false
			}}
		>
			<div class="flex flex-col" bind:clientHeight={height}>
				{#each messages as message, messageIndex}
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
								'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg',
							message.role === 'user'
								? aiChatManager.loading && lastUserMessageIndex === messageIndex
									? 'mb-1'
									: 'mb-2'
								: '',
							(message.role === 'assistant' || message.role === 'tool') && 'px-[1px]',
							message.role === 'tool' && 'text-tertiary'
						)}
					>
						{#if message.role === 'assistant'}
							<AssistantMessage {message} />
						{:else}
							{message.content}
						{/if}
					</div>
					{#if message.role === 'user' && aiChatManager.loading && lastUserMessageIndex === messageIndex}
						<div class="flex flex-row px-2 mb-2">
							<Button
								startIcon={{ icon: StopCircleIcon }}
								size="xs3"
								variant="border"
								btnClasses="px-1.5"
								color="light"
								on:click={() => {
									cancel()
								}}
							>
								Stop
							</Button>
						</div>
					{/if}
				{/each}
				{#if aiChatManager.loading && !aiChatManager.currentReply}
					<div class="mb-6 py-1 px-2">
						<Loader2 class="animate-spin" />
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<div class:border-t={messages.length > 0}>
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
				instructions={aiChatManager.instructions}
				{availableContext}
				{selectedContext}
				isFirstMessage={messages.length === 0}
				on:addContext={(e) => addContextToSelection(e.detail.contextElement)}
				on:sendRequest={() => {
					if (!aiChatManager.loading) {
						aiChatManager.sendRequest()
					}
				}}
				on:updateInstructions={(e) => (aiChatManager.instructions = e.detail.value)}
				{disabled}
			/>
		{:else}
			<div class="relative w-full px-2 scroll-pb-2 pt-2">
				<textarea
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
				<div class="text-tertiary text-xs mt-2 px-2">
					<Markdown md={disabledMessage} />
				</div>
			{:else}
				<div class="flex flex-row gap-2 min-w-0">
					<ChatMode {allowedModes} />
					<ProviderModelSelector />
				</div>
			{/if}
		</div>
		{#if aiChatManager.mode === 'navigator' && suggestions.length > 0 && messages.filter((m) => m.role === 'user').length === 0 && !disabled}
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
