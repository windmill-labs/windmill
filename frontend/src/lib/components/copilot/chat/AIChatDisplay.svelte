<script lang="ts">
	import AIChatMessage from './AIChatMessage.svelte'
	import AppAvailableContextList from './AppAvailableContextList.svelte'
	import AvailableContextList from './AvailableContextList.svelte'
	import { type Snippet } from 'svelte'
	import {
		AlertTriangle,
		ArrowDown,
		AtSign,
		ChevronDown,
		ChevronsRight,
		CheckIcon,
		Hand,
		HistoryIcon,
		Hourglass,
		MousePointer2,
		Plus,
		TextSelect,
		X,
		XIcon
	} from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { fade } from 'svelte/transition'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { type DisplayMessage } from './shared'
	import type { ContextElement } from './context'
	import ChatQuickActions from './ChatQuickActions.svelte'
	import ProviderModelSelector from './ProviderModelSelector.svelte'
	import ChatMode from './ChatMode.svelte'
	import DatatableCreationPolicy from './DatatableCreationPolicy.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { twMerge } from 'tailwind-merge'
	import { AIAutonomyMode, AIMode } from './AIChatManager.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import ChatTypingIndicator from './ChatTypingIndicator.svelte'
	import AIChatInput from './AIChatInput.svelte'
	import { getModifierKey } from '$lib/utils'
	import type { SelectedContext } from './app/core'

	const MAX_YOLO_TOOLTIP_TOOLS = 8
	const aiChatManager = getAiChatManager()
	// `label` is shown in the dropdown; `shortLabel` (when set) is shown in the
	// compact trigger pill to save horizontal space.
	type AutonomyModeOption = { label: string; shortLabel?: string; mode: AIAutonomyMode }
	const autonomyModeOptions: AutonomyModeOption[] = [
		{ label: 'Ask permission', mode: AIAutonomyMode.DEFAULT },
		{ label: 'Auto-accept edits', mode: AIAutonomyMode.ACCEPT_EDIT },
		{ label: 'Yolo (bypass permissions)', shortLabel: 'Yolo', mode: AIAutonomyMode.YOLO }
	]
	const autonomyModeLabel = (mode: AIAutonomyMode) => {
		const option = autonomyModeOptions.find((o) => o.mode === mode) ?? autonomyModeOptions[0]
		return option.shortLabel ?? option.label
	}
	// "Auto-accept edits" only applies where script/flow edits can be accepted,
	// "Bypass permissions" only where tool confirmations exist; filter the picker
	// to the levels that actually do something in the current mode.
	const isAutonomyModeAvailable = (
		mode: AIAutonomyMode,
		autoAcceptEditsAvailable: boolean,
		autoAcceptToolConfirmationsAvailable: boolean
	) => {
		switch (mode) {
			case AIAutonomyMode.DEFAULT:
				return true
			case AIAutonomyMode.ACCEPT_EDIT:
				return autoAcceptEditsAvailable
			case AIAutonomyMode.YOLO:
				return autoAcceptToolConfirmationsAvailable
		}
		return false
	}
	// Ask-permission holds (raised hand); auto-accept/bypass fast-forward. Color
	// ramps from muted (ask) to accent (auto-accept) to red (bypass).
	const autonomyModeIcon = (mode: AIAutonomyMode) =>
		mode === AIAutonomyMode.DEFAULT ? Hand : ChevronsRight
	const autonomyModeIconColor = (mode: AIAutonomyMode) =>
		mode === AIAutonomyMode.YOLO
			? 'text-red-500'
			: mode === AIAutonomyMode.DEFAULT
				? 'text-secondary'
				: 'text-accent'

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
		askAi = () => {}, // todo: remove default,
		headerLeft,
		headerRight,
		disabled = false,
		disabledMessage = '',
		suggestions = [],
		hideHeader = false,
		hideModeSelector = false,
		wideLayout = false,
		emptyHint,
		inputPreface
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
		askAi?: (instructions: string, options?: { withCode?: boolean; withDiff?: boolean }) => void
		headerLeft?: Snippet
		headerRight?: Snippet
		disabled?: boolean
		disabledMessage?: string
		suggestions?: string[]
		hideHeader?: boolean
		hideModeSelector?: boolean
		// Center the messages + input columns inside a max-w-3xl px-8
		// inner container. The session pane uses this for breathing
		// room; the right-hand global chat panel is narrow enough that
		// the inner padding eats too much horizontal space, so it's
		// off there.
		wideLayout?: boolean
		emptyHint?: Snippet
		inputPreface?: Snippet
	} = $props()

	let aiChatInput: AIChatInput | undefined = $state()
	let editingMessageIndex = $state<number | null>(null)

	let scrollEl: HTMLDivElement | undefined = $state()
	// Programmatic-scroll guard. `scrollDown()` triggers an async `scroll`
	// event; if a token-append between the scrollTo and the dispatch makes
	// scrollHeight grow, the gap can briefly exceed STICK_TO_BOTTOM_PX and
	// disengage auto-scroll mid-stream. A short cooldown after our own
	// scroll swallows that spurious event without affecting genuine user
	// scrolls (wheel/touch/keyboard are reaction-time orders of magnitude
	// slower than the cooldown).
	const PROGRAMMATIC_SCROLL_COOLDOWN_MS = 120
	let programmaticScrollAt: number | undefined
	// Instant scroll — smooth would animate every token append, racing with
	// the next scrollDown and confusing the onscroll bottom-detection below.
	function scrollDown() {
		if (!scrollEl) return
		programmaticScrollAt = Date.now()
		scrollEl.scrollTo({ top: scrollEl.scrollHeight, behavior: 'auto' })
	}

	let height = $state(0)
	$effect(() => {
		if (aiChatManager.automaticScroll && height) {
			scrollDown()
		}
		// Recompute the scroll-to-latest visibility on every content-height
		// change. `onScroll` only fires for actual scroll events, so without
		// this the arrow can go stale when content grows past the threshold
		// while auto-scroll is disabled (user scrolled up mid-stream).
		if (scrollEl && height) {
			const distance = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight
			showScrollToLatest = distance > SCROLL_TO_LATEST_THRESHOLD_PX
		}
	})

	// Pixel distance from the bottom under which we treat the user as
	// "stuck to the bottom" and re-enable automatic scroll. 8px allows for
	// sub-pixel rounding from scrollTo + the occasional overscroll bounce.
	const STICK_TO_BOTTOM_PX = 8
	// Show the "scroll to latest" arrow only once the user has scrolled
	// meaningfully away from the tail — a couple of message-heights up. Avoids
	// flicker when the auto-scroll lags by a few px during streaming.
	const SCROLL_TO_LATEST_THRESHOLD_PX = 200
	let showScrollToLatest = $state(false)
	function onScroll() {
		if (!scrollEl) return
		const distance = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight
		// Always refresh the arrow visibility — even during the cooldown,
		// because clicking the arrow itself triggers a programmatic scroll
		// whose only event would otherwise be swallowed, leaving the arrow
		// stuck visible after we already reached the bottom.
		showScrollToLatest = distance > SCROLL_TO_LATEST_THRESHOLD_PX
		if (
			programmaticScrollAt !== undefined &&
			Date.now() - programmaticScrollAt < PROGRAMMATIC_SCROLL_COOLDOWN_MS
		) {
			return
		}
		if (distance <= STICK_TO_BOTTOM_PX) {
			aiChatManager.enableAutomaticScroll()
		} else {
			aiChatManager.disableAutomaticScroll()
		}
	}

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

	const showTypingIndicator = $derived(aiChatManager.loading)

	// `@` context picker is offered in modes that accept workspace/script/flow
	// references (SCRIPT, FLOW, GLOBAL → workspace items + code blocks) or in
	// APP mode (datatables, frontend files, etc.). Other modes (NAVIGATOR,
	// ASK, API) don't accept @-context.
	const showContextPicker = $derived(
		aiChatManager.mode === AIMode.SCRIPT ||
			aiChatManager.mode === AIMode.FLOW ||
			aiChatManager.mode === AIMode.GLOBAL ||
			aiChatManager.mode === AIMode.APP
	)
	const availableAutonomyModeOptions = $derived.by(() =>
		autonomyModeOptions.filter((option) =>
			isAutonomyModeAvailable(
				option.mode,
				aiChatManager.autoAcceptEditsAvailable,
				aiChatManager.autoAcceptToolConfirmationsAvailable
			)
		)
	)
	// Fall back to ask-permission when the persisted mode isn't applicable in the
	// current AI mode (e.g. auto-accept edits while in a mode without edits).
	const effectiveAutonomyMode = $derived(
		availableAutonomyModeOptions.some((option) => option.mode === aiChatManager.autonomyMode)
			? aiChatManager.autonomyMode
			: AIAutonomyMode.DEFAULT
	)
	const showAutonomyModeSelector = $derived(!disabled && availableAutonomyModeOptions.length > 1)
	const autonomyModeTooltip = $derived.by(() => {
		switch (effectiveAutonomyMode) {
			case AIAutonomyMode.ACCEPT_EDIT:
				return 'Automatically accepts script and flow edits. Tool calls still ask for confirmation.'
			case AIAutonomyMode.YOLO:
				if (!aiChatManager.autoAcceptEditsAvailable) {
					return 'Automatically accepts tool confirmations.'
				}
				return 'Automatically accepts script and flow edits plus tool confirmations.'
			default:
				if (!aiChatManager.autoAcceptEditsAvailable) {
					return 'Requires confirmation for tool calls.'
				}
				return 'Requires confirmation for edits and tool calls.'
		}
	})

	// "Waiting for user" detection — when the latest tool message is staged
	// for confirmation or has an unanswered askUserQuestion, the AI loop is
	// paused on the user, not on its own work. The typing-dots indicator
	// implies the AI is busy, which is misleading; surface a text pill
	// instead so users know to act on the tool above.
	const waitingForUserAction = $derived.by(() => {
		if (!aiChatManager.loading) return false
		const last = messages[messages.length - 1]
		if (!last || last.role !== 'tool') return false
		if (last.needsConfirmation && last.isLoading) return true
		if (
			last.userQuestion &&
			last.isLoading &&
			!last.error &&
			!last.userQuestion.selectedChoice &&
			!last.userQuestion.canceled
		) {
			return true
		}
		return false
	})

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
	const showFlowPendingActionControls = $derived(
		(aiChatManager.flowAiChatHelpers?.hasPendingChanges() ?? false) &&
			!aiChatManager.autoAcceptEditsActive
	)
	const showFooterLeftControls = $derived(
		!disabled &&
			(showContextPicker ||
				showAutonomyModeSelector ||
				(aiChatManager.mode === AIMode.SCRIPT && hasDiff))
	)
</script>

<div class="flex flex-col h-full">
	{#if !hideHeader}
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
	{/if}
	{#if messages.length === 0}
		{#if emptyHint}
			{@render emptyHint()}
		{:else}
			<span class="text-2xs text-gray-500 dark:text-gray-400 text-center px-2 my-2"
				>You can use {getModifierKey()}L to open or close this chat, and {getModifierKey()}K in the
				script editor to modify selected lines.</span
			>
		{/if}
	{/if}

	{#if messages.length > 0}
		<div class="flex-1 min-h-0 relative">
			<div class="absolute inset-0 overflow-y-scroll pt-2" bind:this={scrollEl} onscroll={onScroll}>
				<div
					class={wideLayout
						? 'w-full max-w-3xl mx-auto px-7 flex flex-col pb-2'
						: 'w-full max-w-2xl mx-auto px-3 flex flex-col pb-2'}
					bind:clientHeight={height}
				>
					{#each messages as message, messageIndex (messageIndex)}
						<AIChatMessage
							{message}
							{messageIndex}
							{availableContext}
							bind:selectedContext
							bind:editingMessageIndex
							isLast={messageIndex === messages.length - 1}
						/>
					{/each}
					{#if showTypingIndicator}
						<div
							class={twMerge(
								'sticky z-10 mt-0.5 ml-2 self-start pointer-events-none',
								showFlowPendingActionControls ? 'bottom-14' : 'bottom-2'
							)}
						>
							{#if waitingForUserAction}
								<span
									class="inline-flex items-center gap-1.5 text-2xs text-accent"
									aria-label="Waiting for your input"
								>
									<Hourglass class="w-3 h-3 hourglass-flip" />
									Waiting for your input
								</span>
							{:else}
								<ChatTypingIndicator loading={aiChatManager.loading} />
							{/if}
						</div>
					{/if}
				</div>
			</div>
			{#if showScrollToLatest}
				<div
					transition:fade={{ duration: 120 }}
					class={twMerge(
						'absolute left-1/2 -translate-x-1/2 z-10 rounded-md bg-surface shadow-md',
						showFlowPendingActionControls ? 'bottom-12' : 'bottom-2'
					)}
				>
					<Button
						variant="default"
						unifiedSize="sm"
						iconOnly
						title="Scroll to latest"
						aria-label="Scroll to latest message"
						startIcon={{ icon: ArrowDown }}
						on:click={() => {
							aiChatManager.enableAutomaticScroll()
							scrollDown()
						}}
					/>
				</div>
			{/if}
		</div>
	{/if}

	<div
		class={wideLayout
			? 'relative w-full max-w-3xl mx-auto px-6 pb-2'
			: 'relative w-full max-w-2xl mx-auto px-2 pb-2'}
	>
		{#if showFlowPendingActionControls}
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
				<div class="rounded bg-surface">
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
			</div>
		{/if}
		<div>
			{#if inputPreface}
				{@render inputPreface()}
			{/if}
			<AIChatInput
				bind:this={aiChatInput}
				bind:selectedContext
				{availableContext}
				{disabled}
				isFirstMessage={messages.length === 0}
			/>
			<div
				class="mt-1 flex flex-row flex-wrap items-center gap-x-1.5 gap-y-1"
				class:justify-between={showFooterLeftControls}
				class:justify-end={!showFooterLeftControls}
			>
				{#if showFooterLeftControls}
					<div class="flex flex-row items-center gap-x-1.5 min-w-0 flex-wrap">
						{#if showContextPicker && !disabled}
							<Popover>
								{#snippet trigger()}
									<Button
										nonCaptureEvent
										unifiedSize="2xs"
										variant="default"
										title="Add context"
										iconOnly
										startIcon={{ icon: AtSign }}
									/>
								{/snippet}
								{#snippet content({ close })}
									{#if aiChatManager.mode === AIMode.APP}
										<AppAvailableContextList
											{availableContext}
											{selectedContext}
											onSelect={(element) => {
												void aiChatInput?.addContextToSelection(element)
												close()
											}}
										/>
									{:else}
										<AvailableContextList
											{availableContext}
											{selectedContext}
											onSelect={(element) => {
												void aiChatInput?.addContextToSelection(element)
												close()
											}}
											onSelectWorkspaceItem={(element) => {
												void aiChatInput?.addContextToSelection(element)
												close()
											}}
										/>
									{/if}
								{/snippet}
							</Popover>
						{/if}
						{#if showAutonomyModeSelector}
							<DropdownV2
								items={() =>
									availableAutonomyModeOptions.map((option) => ({
										displayName: option.label,
										selected: effectiveAutonomyMode === option.mode,
										action: () => aiChatManager.setAutonomyMode(option.mode)
									}))}
								placement="bottom-start"
								fixedHeight={false}
								customWidth={240}
							>
								{#snippet buttonReplacement()}
									<Button
										nonCaptureEvent
										unifiedSize="2xs"
										variant="default"
										title={autonomyModeTooltip}
										startIcon={{
											icon: autonomyModeIcon(effectiveAutonomyMode),
											classes: autonomyModeIconColor(effectiveAutonomyMode)
										}}
										endIcon={{ icon: ChevronDown }}
									>
										{autonomyModeLabel(effectiveAutonomyMode)}
									</Button>
								{/snippet}
							</DropdownV2>
						{/if}
						{#if effectiveAutonomyMode === AIAutonomyMode.YOLO && aiChatManager.autoAcceptToolConfirmationsAvailable}
							<Tooltip small placement="top">
								<AlertTriangle class="w-3 h-3 text-red-500" />
								{#snippet text()}
									<div class="max-w-64 text-xs">
										<p class="font-semibold">
											{aiChatManager.autoAcceptEditsAvailable
												? 'Bypass permissions auto-accepts edits and tool usage.'
												: 'Bypass permissions auto-accepts tool usage.'}
										</p>
										<p class="mt-1">
											{aiChatManager.autoAcceptEditsAvailable
												? 'This can result in edits being applied or tools being called without user confirmation.'
												: 'This can result in tools being called without user confirmation.'}
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
						{#if !hideModeSelector}
							<ChatMode />
						{/if}
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
							variant="subtle"
							size="xs2"
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

<style>
	/* Hourglass flips every 4s with long rests at each upright position.
	   `:global` because the class is applied to a child component's root
	   (Lucide SVG) and Svelte scoped CSS otherwise wouldn't match it. */
	:global(.hourglass-flip) {
		animation: hourglass-flip 4s cubic-bezier(0.65, 0, 0.35, 1) infinite;
		transform-origin: center;
	}
	@keyframes hourglass-flip {
		0%,
		35% {
			transform: rotate(0deg);
		}
		50%,
		85% {
			transform: rotate(180deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
