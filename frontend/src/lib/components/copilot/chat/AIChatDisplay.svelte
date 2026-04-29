<script lang="ts">
	import AIChatMessage from './AIChatMessage.svelte'
	import {
		ArrowUp,
		CheckIcon,
		Code2,
		FileCode,
		Loader2,
		MousePointer2,
		Square,
		TextSelect,
		X,
		XIcon
	} from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { type DisplayMessage } from './shared'
	import type { ContextElement } from './context'
	import ChatQuickActions from './ChatQuickActions.svelte'
	import ProviderModelSelector from './ProviderModelSelector.svelte'
	import ChatMode from './ChatMode.svelte'
	import DatatableCreationPolicy from './DatatableCreationPolicy.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { aiChatManager, AIMode } from './AIChatManager.svelte'
	import AIChatInput from './AIChatInput.svelte'
	import { getModifierKey } from '$lib/utils'
	import type { SelectedContext } from './app/core'

	let {
		messages,
		hasDiff,
		diffMode = false, // todo: remove default
		selectedContext = $bindable([]), // todo: remove default
		availableContext = [], // todo: remove default
		cancel,
		askAi = () => {}, // todo: remove default,
		disabled = false,
		disabledMessage = '',
		suggestions = []
	}: {
		messages: DisplayMessage[]
		hasDiff?: boolean
		diffMode: boolean
		selectedContext: ContextElement[]
		availableContext: ContextElement[]
		cancel: () => void
		askAi?: (instructions: string, options?: { withCode?: boolean; withDiff?: boolean }) => void
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
</script>

<div class="flex flex-col h-full">
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
				{#each messages as message, messageIndex}
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

	<div class="relative px-2 pb-2 pt-1 w-full max-w-xl mx-auto">
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
		<div
			class="rounded-2xl border border-light bg-surface shadow-sm transition-all focus-within:shadow-md focus-within:border-gray-300 dark:focus-within:border-gray-600"
		>
			<div class="px-1 pt-0.5">
				<AIChatInput
					bind:this={aiChatInput}
					bind:selectedContext
					{availableContext}
					{disabled}
					isFirstMessage={messages.length === 0}
				/>
			</div>
			<div class="flex flex-row items-center justify-between gap-2 px-2 pb-1.5 pt-0.5">
				<div class="flex flex-row items-center gap-x-1.5 min-w-0 flex-wrap">
					{#if aiChatManager.mode === 'script' && hasDiff}
						<ChatQuickActions {askAi} {diffMode} />
					{/if}
					{#if disabled}
						<div class="text-primary text-xs px-1">
							<Markdown md={disabledMessage} />
						</div>
					{:else}
						<ChatMode />
						{#if aiChatManager.mode === AIMode.APP}
							<DatatableCreationPolicy />
						{/if}
						<ProviderModelSelector />

						{#if aiChatManager.mode === AIMode.APP && appContext && (appContext.type !== 'none' || appContext.inspectorElement || appContext.codeSelection)}
							{#if appContext.type === 'frontend' && appContext.frontendPath && !appContext.selectionExcluded}
								<div
									class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 text-2xs"
									title={appContext.frontendPath}
								>
									<FileCode class="w-3 h-3" />
									<span class="truncate max-w-[80px]">{appContext.frontendPath}</span>
									<button
										class="hover:bg-blue-200 dark:hover:bg-blue-800/50 rounded p-0.5 -mr-0.5"
										onclick={() => appContext.toggleSelectionExcluded?.()}
										title="Exclude from prompt"
									>
										<X class="w-2.5 h-2.5" />
									</button>
								</div>
							{:else if appContext.type === 'backend' && appContext.backendKey && !appContext.selectionExcluded}
								<div
									class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 text-2xs"
									title={appContext.backendKey}
								>
									<Code2 class="w-3 h-3" />
									<span class="truncate max-w-[80px]">{appContext.backendKey}</span>
									<button
										class="hover:bg-green-200 dark:hover:bg-green-800/50 rounded p-0.5 -mr-0.5"
										onclick={() => appContext.toggleSelectionExcluded?.()}
										title="Exclude from prompt"
									>
										<X class="w-2.5 h-2.5" />
									</button>
								</div>
							{/if}
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
					{/if}
				</div>

				{#if !disabled}
					<button
						type="button"
						aria-label={aiChatManager.loading ? 'Stop' : 'Send'}
						title={aiChatManager.loading ? 'Stop' : 'Send'}
						class="shrink-0 inline-flex items-center justify-center w-7 h-7 rounded-full bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900 hover:bg-gray-700 dark:hover:bg-gray-300 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
						disabled={!aiChatManager.loading && !aiChatInput?.hasContent()}
						onclick={() => {
							if (aiChatManager.loading) {
								cancel()
							} else {
								aiChatInput?.triggerSend()
							}
						}}
					>
						{#if aiChatManager.loading}
							<Square class="w-3 h-3" />
						{:else}
							<ArrowUp class="w-3.5 h-3.5" strokeWidth={2.5} />
						{/if}
					</button>
				{/if}
			</div>
		</div>
		{#if (aiChatManager.mode === AIMode.NAVIGATOR || aiChatManager.mode === AIMode.ASK) && suggestions.length > 0 && messages.filter((m) => m.role === 'user').length === 0 && !disabled}
			<div class="mt-4">
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
