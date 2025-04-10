<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import AssistantMessage from './AssistantMessage.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { HistoryIcon, Loader2, Plus, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { type AIChatContext, type DisplayMessage, type ContextElement } from './core'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import ContextTextarea from './ContextTextarea.svelte'
	import AvailableContextList from './AvailableContextList.svelte'
	import ChatQuickActions from './ChatQuickActions.svelte'
	import ProviderModelSelector from './ProviderModelSelector.svelte'

	export let pastChats: { id: string; title: string }[]
	export let messages: DisplayMessage[]
	export let instructions: string
	export let selectedContext: ContextElement[]
	export let availableContext: ContextElement[]
	export let hasDiff: boolean
	export let diffMode: boolean = false
	const dispatch = createEventDispatcher<{
		sendRequest: null
		saveAndClear: null
		deletePastChat: { id: string }
		loadPastChat: { id: string }
		analyzeChanges: null
		explainChanges: null
		suggestImprovements: null
	}>()

	const { loading, currentReply } = getContext<AIChatContext>('AIChatContext')

	export function enableAutomaticScroll() {
		automaticScroll = true
	}

	let contextTextareaComponent: ContextTextarea

	export function focusInput() {
		contextTextareaComponent?.focus()
	}

	let automaticScroll = true
	let scrollEl: HTMLDivElement
	async function scrollDown() {
		scrollEl?.scrollTo({
			top: scrollEl.scrollHeight,
			behavior: 'smooth'
		})
	}

	let height = 0
	$: automaticScroll && height && scrollDown()

	function addContextToSelection(contextElement: ContextElement) {
		if (
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
</script>

<div class="flex flex-col h-full">
	<div
		class="flex flex-row items-center justify-between gap-2 p-2 border-b border-gray-200 dark:border-gray-600"
	>
		<div class="flex flex-row items-center gap-2">
			<slot name="header-left" />
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
										on:click={() => {
											dispatch('loadPastChat', { id: chat.id })
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
												dispatch('deletePastChat', { id: chat.id })
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
					dispatch('saveAndClear')
				}}
				size="md"
				btnClasses="!p-1"
				startIcon={{ icon: Plus }}
				iconOnly
				variant="border"
				color="light"
			/>
			<slot name="header-right" />
		</div>
	</div>
	{#if messages.length > 0}
		<div
			class="h-full overflow-y-scroll pt-2"
			bind:this={scrollEl}
			on:wheel={(e) => {
				automaticScroll = false
			}}
		>
			<div class="flex flex-col" bind:clientHeight={height}>
				{#each messages as message}
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
								'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg mb-2',
							message.role === 'assistant' && 'px-[1px] mb-6'
						)}
					>
						{#if message.role === 'assistant'}
							<AssistantMessage {message} />
						{:else}
							{message.content}
						{/if}
					</div>
				{/each}
				{#if $loading && !$currentReply}
					<div class="mb-6 py-1 px-2">
						<Loader2 class="animate-spin" />
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<div class:border-t={messages.length > 0}>
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
						selectedContext = selectedContext.filter(
							(c) => c.type !== element.type || c.title !== element.title
						)
					}}
				/>
			{/each}
		</div>
		<ContextTextarea
			bind:this={contextTextareaComponent}
			{instructions}
			{availableContext}
			{selectedContext}
			isFirstMessage={messages.length === 0}
			on:addContext={(e) => addContextToSelection(e.detail.contextElement)}
			on:sendRequest={() => dispatch('sendRequest')}
			on:updateInstructions={(e) => (instructions = e.detail.value)}
		/>
		<div
			class={`flex flex-row ${
				hasDiff ? 'justify-between' : 'justify-end'
			} items-center gap-2 px-0.5`}
		>
			{#if hasDiff}
				<ChatQuickActions on:analyzeChanges on:explainChanges on:suggestImprovements {diffMode} />
			{/if}
			<ProviderModelSelector />
		</div>
	</div>
</div>
