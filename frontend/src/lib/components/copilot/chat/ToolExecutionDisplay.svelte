<script lang="ts">
	import {
		Loader2,
		ChevronRight,
		XCircle,
		Play,
		ClipboardList,
		Check,
		CircleMinus,
		FileText,
		PanelRight,
		Lock
	} from 'lucide-svelte'
	import {
		EXIT_PLAN_MODE_TOOL,
		isPlanCardTool,
		planCardState,
		planVersionTarget,
		PLAN_CARD_COPY,
		PLAN_MODE_TEXT_COLOR
	} from './planMode'
	import { Button } from '$lib/components/common'
	import { markdownProse } from '$lib/components/markdownProse'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()
	import { isActiveUserQuestion, type ToolDisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import CodeDisplay from './script/CodeDisplay.svelte'
	import LinkRenderer from './LinkRenderer.svelte'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import ToolConfirmationFooter from './ToolConfirmationFooter.svelte'
	import ToolMessageActions from './ToolMessageActions.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import AskUserQuestionDisplay from './AskUserQuestionDisplay.svelte'
	import WebSearchSourcesDisplay from './WebSearchSourcesDisplay.svelte'
	import ExpandableImage from '$lib/components/common/image/ExpandableImage.svelte'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const isPlanReview = $derived(message.toolName === EXIT_PLAN_MODE_TOOL)
	const isPlanCard = $derived(isPlanCardTool(message.toolName))
	const planCopy = $derived(
		isPlanCardTool(message.toolName) ? PLAN_CARD_COPY[message.toolName] : undefined
	)
	// exit_plan_mode carries the plan, enter_plan_mode the one-line justification.
	const planBody = $derived(isPlanReview ? message.parameters?.summary : message.parameters?.reason)
	const planBodyText = $derived(typeof planBody === 'string' ? planBody : '')
	// Resolved once so the label and the icon cannot disagree about which state this is.
	// Undefined means this call does not read as a plan card at all; it renders as the
	// ordinary tool call below, where its error is the message.
	const planState = $derived(isPlanCard ? planCardState(message) : undefined)
	const planLabel = $derived((planState && planCopy?.[planState]) ?? '')
	const planDoc = $derived(
		message.planArtifactId
			? aiChatManager.artifacts.artifacts.find((a) => a.id === message.planArtifactId)
			: undefined
	)
	// The version this card wrote, not the document's current one, since later proposals move it on.
	const planCardVersion = $derived(planVersionTarget(planDoc, message.planVersion))
	// Keyed by call id: a bare flag would leak the expansion onto the next message reusing
	// this instance. The plan opens in the preview, so only enter's reason needs expanding.
	let planToggled = $state<{ id: string | undefined; open: boolean } | undefined>(undefined)
	const planExpanded = $derived(
		planToggled?.id === message.tool_call_id
			? planToggled.open
			: isPlanCard && !isPlanReview && Boolean(message.needsConfirmation)
	)

	// The preview pane's renderers, so a model-written link opens in a new tab either way.
	const planPlugins = [gfmPlugin(), { renderer: { pre: CodeDisplay, a: LinkRenderer } }]

	const hasParameters = $derived(
		message.parameters !== undefined && Object.keys(message.parameters).length > 0
	)

	const isSuccessful = $derived(
		!message.isLoading &&
			!message.isQueued &&
			!message.error &&
			!message.needsConfirmation &&
			!message.isStreamingArguments
	)
	const autoCollapseDetails = $derived(message.autoCollapseDetails !== false)

	// Executing right now, as opposed to queued behind another tool or waiting on
	// the user — the only state that gets the shimmer.
	const isRunning = $derived(Boolean(message.isLoading && !message.needsConfirmation))

	// An errored tool must be expandable even if it never opted into details,
	// otherwise the error set on its status would be invisible.
	const detailsAvailable = $derived(message.showDetails === true || message.error !== undefined)

	let isExpanded = $derived(
		Boolean(
			(detailsAvailable && (!isSuccessful || !autoCollapseDetails)) ||
				(message.isStreamingArguments && hasParameters) ||
				(message.isLoading && message.needsConfirmation)
		)
	)

	const visibleActions = $derived(
		message.actions && !message.isLoading && !message.error && !message.needsConfirmation
			? message.actions
			: []
	)

	const activeUserQuestion = $derived(
		isActiveUserQuestion(message) ? message.userQuestion : undefined
	)

	// The preview chip sits on the header row (to the right of the tool-call text);
	// shown once the tool settled, never while loading/erroring/awaiting confirmation.
	const showPreviewChip = $derived(
		Boolean(
			message.previewCard && !message.isLoading && !message.error && !message.needsConfirmation
		)
	)
</script>

{#if activeUserQuestion}
	<AskUserQuestionDisplay toolCallId={message.tool_call_id} userQuestion={activeUserQuestion} />
{:else if message.blockedByPlanMode}
	<!-- Not an error card: the call did what plan mode says it should. One flat row
	     naming the refused tool, so "why can't it edit" is answered where it is asked. -->
	<div class="font-mono text-xs flex items-center gap-2 py-0.5 my-0.5 min-w-0">
		<Lock class="w-3.5 h-3.5 text-tertiary shrink-0" />
		<span class="font-medium text-2xs text-tertiary shrink-0">
			{message.content}
		</span>
		{#if message.toolName}
			<span class="text-2xs text-tertiary truncate">{message.toolName}</span>
		{/if}
	</div>
{:else if planState}
	<!-- Same lean shape as a tool call below: a header row that collapses into the
	     transcript, with everything else in one box under it. -->
	<div
		class={twMerge(
			'font-mono text-xs',
			message.isQueued && !message.error ? 'opacity-60 hover:opacity-100 transition-opacity' : ''
		)}
	>
		<div class="flex items-center justify-between gap-2">
			<button
				class="min-w-0 py-0.5 my-0.5 rounded-md hover:bg-surface-hover transition-colors inline-flex items-center text-left"
				onclick={() => (planToggled = { id: message.tool_call_id, open: !planExpanded })}
				disabled={!planBodyText}
				aria-expanded={planExpanded}
			>
				<div class="flex items-center gap-2 min-w-0">
					{#if message.isLoading && !message.needsConfirmation}
						<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500 shrink-0" />
					{:else if planState === 'settled'}
						<Check class={twMerge('w-3.5 h-3.5 shrink-0', PLAN_MODE_TEXT_COLOR)} />
					{:else if planState === 'declined'}
						<!-- Muted and not red: declining a plan is an outcome, not a failure, and
						     this state also covers a card resolved by leaving plan mode. -->
						<CircleMinus class="w-3.5 h-3.5 text-tertiary shrink-0" />
					{:else}
						<ClipboardList class="w-3.5 h-3.5 text-secondary shrink-0" />
					{/if}
					<span class="text-primary font-medium text-2xs">{planLabel}</span>
					{#if planBodyText}
						<ChevronRight
							class={twMerge(
								'w-3 h-3 text-secondary transition-transform duration-150 shrink-0',
								planExpanded ? 'rotate-90' : ''
							)}
						/>
					{/if}
				</div>
			</button>
			{#if planDoc}
				<Button
					variant="default"
					unifiedSize="2xs"
					wrapperClasses="shrink-0"
					title="Open this plan in the side panel: {planDoc.name}"
					startIcon={{ icon: FileText, classes: PLAN_MODE_TEXT_COLOR }}
					endIcon={{ icon: PanelRight }}
					on:click={() => aiChatManager.openArtifact?.(planDoc.id, planDoc.name, planCardVersion)}
				>
					<span class="font-main">Plan</span>
				</Button>
			{/if}
		</div>

		{#snippet confirmFooter(extraClass: string)}
			<ToolConfirmationFooter
				toolCallId={message.tool_call_id}
				rejectLabel={planCopy?.reject}
				confirmLabel={planCopy?.confirm ?? ''}
				confirmIcon={isPlanReview ? undefined : ClipboardList}
				class={extraClass}
			/>
		{/snippet}

		{#if planExpanded && planBodyText}
			<div
				transition:slide={{ duration: 150 }}
				class="border border-border-light rounded-md bg-surface p-3 space-y-3 font-main"
			>
				{#if isPlanReview}
					<div class={markdownProse.sm}>
						<Markdown md={planBodyText} plugins={planPlugins} />
					</div>
				{:else}
					<div class="text-xs text-secondary leading-snug">{planBodyText}</div>
				{/if}
				{#if message.needsConfirmation}
					{@render confirmFooter('')}
				{/if}
			</div>
			<!-- Collapsed: the buttons stand on their own rather than boxing empty space. -->
		{:else if message.needsConfirmation}
			{@render confirmFooter('py-1 font-main')}
		{/if}
	</div>
{:else}
	<!-- Discrete preview chip for an item a tool created/updated, pinned to the
	     right of the header row. Rendered inline (not gated on expand) so it stays
	     visible after the tool collapses. -->
	{#snippet previewChip()}
		{#if message.previewCard}
			<ToolPreviewCard card={message.previewCard} />
		{/if}
	{/snippet}

	<!-- The shimmer is the only running indicator, so the states have to read off
	     weight alone: queued calls (waiting their turn behind the executing tool)
	     are faded, the running one sweeps, a settled one is plain. -->
	<ChatCollapsibleCard
		label={message.content}
		expanded={isExpanded}
		onToggle={() => (isExpanded = !isExpanded)}
		toggleable={detailsAvailable || message.isStreamingArguments === true}
		shimmer={isRunning}
		class={message.isQueued && !message.error
			? 'opacity-60 hover:opacity-100 transition-opacity'
			: ''}
		headerClass={message.needsConfirmation ? 'opacity-80' : ''}
		labelClass={showPreviewChip ? 'truncate' : ''}
		contentClass="space-y-3"
		headerRight={showPreviewChip ? previewChip : undefined}
	>
		<!-- Image a tool produced (e.g. take_screenshot) — shown inline, not gated on expand. -->
		{#snippet belowHeader()}
			{#if message.imageUrl}
				<div class="my-1">
					<ExpandableImage
						src={message.imageUrl}
						alt="App preview screenshot"
						class="max-h-48 max-w-full rounded border border-border-light"
					/>
				</div>
			{/if}
		{/snippet}

		<!-- Parameters Section - show if we have parameters, or if confirmation is needed (even with empty params) -->
		{#if hasParameters || message.needsConfirmation}
			<div class={message.needsConfirmation ? 'opacity-80' : ''}>
				<ToolContentDisplay
					title="Parameters"
					content={message.parameters}
					streaming={message.isStreamingArguments}
					toolName={message.toolName}
					showFade={message.showFade}
				/>
			</div>
		{/if}

		<!-- Confirmation Footer -->
		{#if message.needsConfirmation}
			<ToolConfirmationFooter
				toolCallId={message.tool_call_id}
				rejectIcon={XCircle}
				rejectDestructive
				confirmLabel="Run"
				confirmIcon={Play}
			/>

			<!-- Logs and Result - hide while streaming -->
		{:else if !message.isStreamingArguments}
			<ToolContentDisplay
				title="Logs"
				content={message.logs}
				loading={message.isLoading}
				showWhileLoading={false}
				showFade={message.showFade}
			/>

			{#if visibleActions.length > 0}
				<ToolMessageActions actions={visibleActions} />
			{:else if message.webSearchSources?.length && !message.error}
				<WebSearchSourcesDisplay sources={message.webSearchSources} />
			{:else}
				<ToolContentDisplay
					title="Result"
					content={message.result}
					error={message.error}
					loading={message.isLoading}
					showFade={message.showFade}
				/>
			{/if}
		{/if}
	</ChatCollapsibleCard>
{/if}
