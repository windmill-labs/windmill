<script lang="ts">
	import AIChatMessage from './AIChatMessage.svelte'
	import { type Snippet } from 'svelte'
	import {
		AlertTriangle,
		CheckIcon,
		HistoryIcon,
		Loader2,
		MousePointer2,
		Plus,
		Square,
		TextSelect,
		X,
		XIcon
	} from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { type DisplayMessage } from './shared'
	import type { ContextElement } from './context'
	import ChatQuickActions from './ChatQuickActions.svelte'
	import ProviderModelSelector from './ProviderModelSelector.svelte'
	import ChatMode from './ChatMode.svelte'
	import DatatableCreationPolicy from './DatatableCreationPolicy.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import AIChatInput from './AIChatInput.svelte'
	import { getModifierKey } from '$lib/utils'
	import type { SelectedContext } from './app/core'

	const MAX_YOLO_TOOLTIP_TOOLS = 8

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

	let aiChatInput: AIChatInput | undefined = $state()
	let editingMessageIndex = $state<number | null>(null)

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

	function submitSuggestion(suggestion: string) {
		aiChatManager.sendRequest({ instructions: suggestion })
	}

	export function focusInput() {
		aiChatInput?.focusInput()
	}

	$effect(() => {
		if (aiChatInput) {
			aiChatManager.setAiChatInput(aiChatInput)
		}

		return () => {
			aiChatManager.setAiChatInput(null)
		}
	})

	const isLastMessageTool = $derived(
		messages.length > 0 && messages[messages.length - 1].role === 'tool'
	)

	// Get app context for display when in APP mode
	const appContext = $derived.by((): SelectedContext | undefined => {
		if (aiChatManager.mode !== AIMode.APP || !aiChatManager.appAiChatHelpers) {
			return undefined
		}
		return aiChatManager.appAiChatHelpers.getSelectedContext()
	})

	const yoloBypassedTools = $derived.by(() => {
		return aiChatManager.tools
			.filter((tool) => tool.requiresConfirmation === true)
			.map((tool) => ({
				name: tool.def.function.name,
				label: tool.confirmationMessage ?? tool.def.function.name
			}))
	})
	const visibleYoloBypassedTools = $derived(yoloBypassedTools.slice(0, MAX_YOLO_TOOLTIP_TOOLS))
	const hiddenYoloBypassedToolCount = $derived(
		Math.max(0, yoloBypassedTools.length - visibleYoloBypassedTools.length)
	)
	const showFooterLeftControls = $derived(
		!disabled &&
			(aiChatManager.autoAcceptToolConfirmationsAvailable ||
				(aiChatManager.mode === AIMode.SCRIPT && hasDiff))
	)
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
				{#snippet trigger()}
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
				{/snippet}
				{#snippet content({ close })}
					<div class="p-1 overflow-y-auto max-h-[300px]">
						{#if pastChats.length === 0}
							<div class="text-center text-primary text-xs">No history</div>
						{:else}
							<div class="flex flex-col">
								{#each pastChats as chat (chat.id)}
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
											variant="default"
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
				{/snippet}
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
	{#if messages.length === 0}
		<span class="text-2xs text-gray-500 dark:text-gray-400 text-center px-2 my-2"
			>You can use {getModifierKey()}L to open or close this chat, and {getModifierKey()}K in the
			script editor to modify selected lines.</span
		>
	{/if}

	{#if messages.length > 0}
		<div
			class="h-full overflow-y-scroll pt-2 pb-12"
			bind:this={scrollEl}
			onwheel={() => {
				aiChatManager.disableAutomaticScroll()
			}}
		>
			<div class="flex flex-col" bind:clientHeight={height}>
				{#each messages as message, messageIndex (messageIndex)}
					<AIChatMessage
						{message}
						{messageIndex}
						{availableContext}
						bind:selectedContext
						bind:editingMessageIndex
					/>
				{/each}
				{#if aiChatManager.loading && !aiChatManager.currentReply && !isLastMessageTool}
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
					startIcon={{ icon: Square }}
					size="xs"
					variant="default"
					btnClasses="bg-surface hover:bg-surface-selected"
					on:click={() => {
						cancel()
					}}
				>
					Stop
				</Button>
			</div>
		{:else if aiChatManager.flowAiChatHelpers?.hasPendingChanges()}
			<div class="absolute -top-10 w-full flex flex-row justify-center gap-2">
				<Button
					startIcon={{ icon: CheckIcon }}
					size="xs"
					variant="default"
					btnClasses="bg-green-500 hover:bg-green-600 text-white hover:text-white"
					onclick={() => {
						aiChatManager.flowAiChatHelpers?.acceptAllModuleActions()
					}}
				>
					Accept all
				</Button>
				<Button
					startIcon={{ icon: XIcon }}
					size="xs"
					variant="default"
					btnClasses="dark:opacity-50 opacity-60 hover:opacity-100"
					onclick={() => {
						aiChatManager.flowAiChatHelpers?.rejectAllModuleActions()
					}}
				>
					Reject all
				</Button>
			</div>
		{/if}
		<div class="px-2">
			<AIChatInput
				bind:this={aiChatInput}
				bind:selectedContext
				{availableContext}
				{disabled}
				isFirstMessage={messages.length === 0}
			/>
			<div class="flex flex-row justify-between items-center gap-2">
				{#if showFooterLeftControls}
					<div class="flex flex-row gap-x-2 min-w-0 flex-wrap items-center">
						{#if aiChatManager.autoAcceptToolConfirmationsAvailable}
							<Toggle
								size="xs"
								color="red"
								checked={aiChatManager.autoAcceptToolConfirmations}
								options={{
									right: 'yolo',
									title: 'Auto-accept AI tool confirmations'
								}}
								on:change={(e) => aiChatManager.setAutoAcceptToolConfirmations(e.detail)}
							/>
							<Tooltip small placement="top">
								<AlertTriangle
									class={aiChatManager.autoAcceptToolConfirmations
										? 'w-3 h-3 text-red-500'
										: 'w-3 h-3 text-secondary'}
								/>
								{#snippet text()}
									<div class="max-w-64 text-xs">
										<p class="font-semibold">Yolo auto-accepts tool usage.</p>
										<p class="mt-1">
											This can result in unwanted tools being called without user confirmation.
										</p>
										{#if yoloBypassedTools.length > 0}
											<p class="mt-2 font-semibold">Bypassed in current mode:</p>
											<ul class="mt-1 list-disc pl-4 space-y-0.5">
												{#each visibleYoloBypassedTools as tool (tool.name)}
													<li class="break-words">{tool.label}</li>
												{/each}
											</ul>
											{#if hiddenYoloBypassedToolCount > 0}
												<p class="mt-1">+ {hiddenYoloBypassedToolCount} more</p>
											{/if}
										{:else}
											<p class="mt-2">No tools in the current mode require confirmation.</p>
										{/if}
									</div>
								{/snippet}
							</Tooltip>
						{/if}
						{#if aiChatManager.mode === AIMode.SCRIPT && hasDiff}
							<ChatQuickActions {askAi} {diffMode} />
						{/if}
					</div>
				{/if}
				{#if disabled}
					<div class="text-primary text-xs my-2 px-2">
						<Markdown md={disabledMessage} />
					</div>
				{:else}
					<div class="flex flex-row gap-x-1.5 min-w-0 flex-wrap items-center">
						<ChatMode />
						{#if aiChatManager.mode === AIMode.APP}
							<DatatableCreationPolicy />
						{/if}
						<ProviderModelSelector />

						{#if aiChatManager.mode === AIMode.APP && appContext && (appContext.inspectorElement || appContext.codeSelection)}
							{#if appContext.inspectorElement}
								<div
									class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 text-2xs"
									title={appContext.inspectorElement.path}
								>
									<MousePointer2 class="w-3 h-3" />
									<span class="truncate max-w-[60px]">
										{appContext.inspectorElement.tagName.toLowerCase()}{appContext.inspectorElement
											.id
											? `#${appContext.inspectorElement.id}`
											: ''}{appContext.inspectorElement.className
											? `.${appContext.inspectorElement.className.split(' ')[0]}`
											: ''}
									</span>
									<button
										class="hover:bg-purple-200 dark:hover:bg-purple-800/50 rounded p-0.5 -mr-0.5"
										onclick={() => appContext.clearInspector?.()}
										title="Clear element selection"
									>
										<X class="w-2.5 h-2.5" />
									</button>
								</div>
							{/if}
							{#if appContext.codeSelection}
								<div
									class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300 text-2xs"
									title={`${appContext.codeSelection.source}: lines ${appContext.codeSelection.startLine}-${appContext.codeSelection.endLine}`}
								>
									<TextSelect class="w-3 h-3" />
									<span class="truncate max-w-[80px]">
										L{appContext.codeSelection.startLine}-{appContext.codeSelection.endLine}
									</span>
									<button
										class="hover:bg-amber-200 dark:hover:bg-amber-800/50 rounded p-0.5 -mr-0.5"
										onclick={() => appContext.clearCodeSelection?.()}
										title="Clear code selection"
									>
										<X class="w-2.5 h-2.5" />
									</button>
								</div>
							{/if}
						{/if}
					</div>
				{/if}
			</div>
		</div>
		{#if (aiChatManager.mode === AIMode.NAVIGATOR || aiChatManager.mode === AIMode.ASK) && suggestions.length > 0 && messages.filter((m) => m.role === 'user').length === 0 && !disabled}
			<div class="px-2 mt-4">
				<div class="flex flex-col gap-2">
					{#each suggestions as suggestion (suggestion)}
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
