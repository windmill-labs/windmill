<script lang="ts">
	import autosize from '$lib/autosize'
	import { twMerge } from 'tailwind-merge'
	import AssistantMessage from './AssistantMessage.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { ChevronDown, HistoryIcon, Loader2, Plus, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import {
		ContextIconMap,
		type AIChatContext,
		type DisplayMessage,
		type ContextElement,
		type SelectedContext
	} from './core'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME,
		copilotInfo,
		copilotSessionModel
	} from '$lib/stores'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import { storeLocalSetting } from '$lib/utils'

	export let pastChats: { id: string; title: string }[]
	export let messages: DisplayMessage[]
	export let instructions: string
	export let selectedContext: SelectedContext[]
	export let availableContext: ContextElement[]

	const dispatch = createEventDispatcher<{
		sendRequest: null
		saveAndClear: null
		deletePastChat: { id: string }
		loadPastChat: { id: string }
	}>()

	const { loading, currentReply } = getContext<AIChatContext>('AIChatContext')

	export function enableAutomaticScroll() {
		automaticScroll = true
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

	$: providerModel = $copilotSessionModel ??
		$copilotInfo.defaultModel ??
		$copilotInfo.aiModels[0] ?? {
			model: 'No model',
			provider: 'No provider'
		}

	let showContextTooltip = false;
	let contextTooltipWord = '';
	let tooltipPosition = { x: 0, y: 0 };
	let textareaEl: HTMLTextAreaElement;

	function getHighlightedText(text: string) {
		return text.replace(/@[\w.-]+/g, (match) => {
			const contextElement = availableContext.find((c) => c.title === match.slice(1))
			if (contextElement) {
				return `<span class="bg-white text-black z-10">${match}</span>`
			}
			return match
		})
	}

	function addContextToSelection(contextElement: ContextElement) {
		if (!selectedContext.find((c) => c.type === contextElement.type)) {
			selectedContext = [
				...selectedContext,
				{
					type: contextElement.type,
					title: contextElement.title
				}
			]
		}
	}

	function updateInstructionsWithContext(contextElement: ContextElement) {
		const index = instructions.lastIndexOf("@")
		if (index !== -1) {
			instructions = instructions.substring(0, index) + `@${contextElement.title}`
		}
	}

	function handleContextSelection(contextElement: ContextElement) {
		addContextToSelection(contextElement)
		updateInstructionsWithContext(contextElement)
		showContextTooltip = false
	}

	function handleInput(e: Event) {
		const textarea = e.target as HTMLTextAreaElement;
		const words = instructions.split(/\s+/);
		const lastWord = words[words.length - 1];
		
		if (lastWord.startsWith('@')) {
			const rect = textarea.getBoundingClientRect();
			const cursorPosition = textarea.selectionStart;
			const textBeforeCursor = textarea.value.substring(0, cursorPosition);
			const lines = textBeforeCursor.split('\n');
			const currentLine = lines[lines.length - 1];
			const currentLineNumber = lines.length - 1;
			
			// Create a temporary span to measure text width
			const tempSpan = document.createElement('span');
			tempSpan.style.visibility = 'hidden';
			tempSpan.style.position = 'absolute';
			tempSpan.style.whiteSpace = 'pre-wrap';
			tempSpan.style.font = window.getComputedStyle(textarea).font;
			tempSpan.style.padding = window.getComputedStyle(textarea).padding;
			tempSpan.textContent = currentLine;
			document.body.appendChild(tempSpan);
			
			// Calculate cursor position
			const lineHeight = parseInt(window.getComputedStyle(textarea).lineHeight);
			const paddingTop = parseInt(window.getComputedStyle(textarea).paddingTop);
			const scrollTop = textarea.scrollTop;
			
			tooltipPosition = {
				x: rect.left + tempSpan.offsetWidth + parseInt(window.getComputedStyle(textarea).paddingLeft) - 100,
				y: rect.top + (currentLineNumber * lineHeight) + paddingTop - scrollTop + lineHeight + 5
			};
			
			document.body.removeChild(tempSpan);
			showContextTooltip = true;
			contextTooltipWord = lastWord.slice(1);
		} else {
			showContextTooltip = false;
			contextTooltipWord = '';
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
					<div class="flex flex-col gap-1 text-tertiary text-xs p-1 min-w-24">
						{#if availableContext.filter((c) => !selectedContext.find((sc) => sc.type === c.type)).length === 0}
							<div class="text-center text-tertiary text-xs">No available context</div>
						{:else}
							{#each availableContext as element}
								{#if !selectedContext.find((c) => c.type === element.type)}
									<button
										class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal"
										on:click={() => {
											handleContextSelection(element)
											close()
										}}
									>
										<svelte:component this={ContextIconMap[element.type]} size={16} />
										{element.title}
									</button>
								{/if}
							{/each}
						{/if}
					</div>
				</svelte:fragment>
			</Popover>
			{#each selectedContext as element}
				{@const contextElement = availableContext.find((c) => c.type === element.type)}
				{#if contextElement}
					<ContextElementBadge
						{contextElement}
						deletable
						on:delete={() => {
							selectedContext = selectedContext.filter((c) => c.type !== element.type)
						}}
					/>
				{/if}
			{/each}
		</div>
		<div class="relative w-full">
			<div
				class="absolute top-0 left-0 w-full h-full min-h-12 p-2 text-sm pt-1"
				style="line-height: 1.72"
			>
			<span class="break-words">
				{@html getHighlightedText(instructions)}
			</span>
			</div>
			<textarea
				bind:this={textareaEl}
			on:keypress={(e) => {
				if (e.key === 'Enter' && !e.shiftKey) {
					e.preventDefault()
					const contextElement = availableContext.find((c) => c.title.includes(contextTooltipWord))
					if (contextTooltipWord && contextElement) {
						handleContextSelection(contextElement)
					} else {
						dispatch('sendRequest')
					}
				}
			}}
			bind:value={instructions}
			use:autosize
			rows={3}
			on:input={handleInput}
			on:blur={() => {
				// Small delay to allow click events on the tooltip to fire first
				setTimeout(() => {
					showContextTooltip = false;
				}, 100);
			}}
			placeholder={messages.length > 0 ? 'Ask followup' : 'Ask anything'}
			class="resize-none bg-transparent absolute top-0 left-0 w-full h-full"
			style="{instructions.length > 0 ? 'color: transparent; -webkit-text-fill-color: transparent;' : ''}"
		/>
		</div>

		{#if showContextTooltip}
			<div
				class="absolute bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg p-2 z-50"
				style="left: {tooltipPosition.x}px; top: {tooltipPosition.y}px;"
			>
				<div class="flex flex-col gap-1 text-tertiary text-xs min-w-24">
					{#if availableContext.length === 0}
						<div class="text-center text-tertiary text-xs">No available context</div>
					{:else}
						{#each availableContext as element}
								<button
									class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal"
									on:click={() => handleContextSelection(element)}
								>
									<svelte:component this={ContextIconMap[element.type]} size={16} />
									{element.title}
								</button>
						{/each}
					{/if}
				</div>
			</div>
		{/if}

		<div class="flex flex-row justify-end items-center gap-2 px-0.5">
			<div class="min-w-0">
				<Popover disablePopup={$copilotInfo.aiModels.length <= 1}>
					<svelte:fragment slot="trigger">
						<div class="text-tertiary text-xs flex flex-row items-center gap-0.5 font-normal">
							{providerModel.model}
							{#if $copilotInfo.aiModels.length > 1}
								<ChevronDown size={16} />
							{/if}
						</div>
					</svelte:fragment>
					<svelte:fragment slot="content" let:close>
						<div class="flex flex-col gap-1 p-1 min-w-24">
							{#each $copilotInfo.aiModels.filter((m) => m.model !== providerModel.model) as providerModel}
								<button
									class="text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal"
									on:click={() => {
										$copilotSessionModel = providerModel
										storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, providerModel.model)
										storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, providerModel.provider)
										close()
									}}
								>
									{providerModel.model}
								</button>
							{/each}
						</div>
					</svelte:fragment>
				</Popover>
			</div>
		</div>
	</div>
</div>
