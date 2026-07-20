<script lang="ts">
	import { Loader2, ChevronRight, XCircle, Play } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()
	import { isActiveUserQuestion, type ToolDisplayMessage } from './shared'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import ToolMessageActions from './ToolMessageActions.svelte'
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
			!message.error &&
			!message.needsConfirmation &&
			!message.isStreamingArguments
	)
	const autoCollapseDetails = $derived(message.autoCollapseDetails !== false)

	// An errored tool must be expandable even if it never opted into details,
	// otherwise the error set on its status would be invisible.
	const detailsAvailable = $derived(message.showDetails === true || message.error !== undefined)

	let isExpanded = $derived(
		(detailsAvailable && (!isSuccessful || !autoCollapseDetails)) ||
			(message.isStreamingArguments && hasParameters) ||
			(message.isLoading && message.needsConfirmation)
	)

	const visibleActions = $derived(
		message.actions && !message.isLoading && !message.error && !message.needsConfirmation
			? message.actions
			: []
	)

	const activeUserQuestion = $derived(
		isActiveUserQuestion(message) ? message.userQuestion : undefined
	)
</script>

{#if activeUserQuestion}
	<AskUserQuestionDisplay toolCallId={message.tool_call_id} userQuestion={activeUserQuestion} />
{:else}
	<div class="font-mono text-xs">
		<!-- Collapsible Header -->
		<button
			class={twMerge(
				'py-0.5 my-0.5 rounded-md hover:bg-surface-hover transition-colors inline-flex items-center text-left',
				message.needsConfirmation ? 'opacity-80' : ''
			)}
			onclick={() => (isExpanded = !isExpanded)}
			disabled={!detailsAvailable && !message.isStreamingArguments}
		>
			<div class="flex items-center gap-2">
				{#if message.isLoading && !message.needsConfirmation}
					<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
				{/if}
				<span class="text-primary font-medium text-2xs">
					{message.content}
				</span>

				{#if detailsAvailable || message.isStreamingArguments}
					<ChevronRight
						class={twMerge(
							'w-3 h-3 text-secondary transition-transform duration-150',
							isExpanded ? 'rotate-90' : ''
						)}
					/>
				{/if}
			</div>
		</button>

		<!-- Image a tool produced (e.g. take_screenshot) — shown inline, not gated on expand. -->
		{#if message.imageUrl}
			<div class="my-1">
				<ExpandableImage
					src={message.imageUrl}
					alt="App preview screenshot"
					class="max-h-48 max-w-full rounded border border-border-light"
				/>
			</div>
		{/if}

		<!-- Expanded Content -->
		{#if isExpanded}
			<div
				transition:slide={{ duration: 150 }}
				class="border border-border-light rounded-md bg-surface p-3 space-y-3"
			>
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
			</div>
		{/if}
	</div>
{/if}
