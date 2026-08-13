<script lang="ts">
	import { XCircle, Play } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()
	import { isActiveUserQuestion, type ToolDisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import ToolMessageActions from './ToolMessageActions.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import AskUserQuestionDisplay from './AskUserQuestionDisplay.svelte'
	import WebSearchSourcesDisplay from './WebSearchSourcesDisplay.svelte'
	import ExpandableImage from '$lib/components/common/image/ExpandableImage.svelte'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

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
			<div class="flex flex-row items-center justify-end gap-2">
				<Button
					variant="default"
					size="xs"
					on:click={() => {
						if (message.tool_call_id) {
							aiChatManager.handleToolConfirmation(message.tool_call_id, false)
						}
					}}
					startIcon={{ icon: XCircle }}
					destructive
				></Button>
				<Button
					variant="accent"
					size="xs"
					on:click={() => {
						if (message.tool_call_id) {
							aiChatManager.handleToolConfirmation(message.tool_call_id, true)
						}
					}}
					startIcon={{ icon: Play }}
				>
					Run
				</Button>
			</div>

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
