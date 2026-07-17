<script lang="ts">
	import { Loader2, ChevronRight, XCircle, Play, ClipboardList, Check } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { getAiChatManager } from './aiChatManagerContext'

	const aiChatManager = getAiChatManager()
	import { isActiveUserQuestion, type ToolDisplayMessage } from './shared'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import ToolContentDisplay from './ToolContentDisplay.svelte'
	import ToolMessageActions from './ToolMessageActions.svelte'
	import AskUserQuestionDisplay from './AskUserQuestionDisplay.svelte'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const isPlanReview = $derived(message.toolName === 'exit_plan_mode')
	const planText = $derived(
		typeof message.parameters?.summary === 'string' ? message.parameters.summary : ''
	)
	const planApproved = $derived(
		isPlanReview &&
			!message.needsConfirmation &&
			!message.error &&
			!message.isLoading &&
			!message.isStreamingArguments
	)
	const planDoc = $derived(
		message.planArtifactId
			? aiChatManager.artifacts.artifacts.find((a) => a.id === message.planArtifactId)
			: undefined
	)
	// A bare flag would leak this expansion onto the next message reusing this instance.
	let expandedCardId = $state<string | undefined>(undefined)
	const planExpanded = $derived(expandedCardId === message.tool_call_id)

	const isPlanEnter = $derived(message.toolName === 'enter_plan_mode')
	const planReason = $derived(
		typeof message.parameters?.reason === 'string' ? message.parameters.reason : ''
	)
	const planEnterStarted = $derived(
		isPlanEnter &&
			!message.needsConfirmation &&
			!message.error &&
			!message.isLoading &&
			!message.isStreamingArguments
	)

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
{:else if isPlanReview}
	<div class="my-1 rounded-md border border-border-light bg-surface overflow-hidden">
		<div
			class="flex items-center justify-between gap-2 px-3 py-2 border-b border-border-light bg-surface-secondary/30"
		>
			<button
				class="flex items-center gap-2 min-w-0 text-left"
				onclick={() => (expandedCardId = planExpanded ? undefined : message.tool_call_id)}
				disabled={!planText}
				aria-expanded={planExpanded}
			>
				{#if message.isLoading && !message.needsConfirmation}
					<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
				{:else if planApproved}
					<Check class="w-3.5 h-3.5 text-green-600" />
				{:else}
					<ClipboardList class="w-3.5 h-3.5 text-secondary" />
				{/if}
				<span class="text-primary font-medium text-xs">
					{planApproved ? 'Plan approved' : message.error ? 'Kept planning' : 'Proposed plan'}
				</span>
				{#if planText}
					<ChevronRight
						class={twMerge(
							'w-3 h-3 text-secondary transition-transform duration-150',
							planExpanded ? 'rotate-90' : ''
						)}
					/>
				{/if}
			</button>
			{#if planDoc}
				<Button
					variant="subtle"
					size="xs"
					on:click={() => aiChatManager.openArtifact?.(planDoc.id, planDoc.name)}
				>
					Open plan document
				</Button>
			{/if}
		</div>
		{#if planText && planExpanded}
			<div
				transition:slide={{ duration: 150 }}
				class="px-3 py-2 prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6
					prose-p:text-xs prose-li:text-xs prose-code:text-xs prose-pre:text-xs
					prose-code:break-words prose-a:break-words
					prose-headings:font-medium prose-headings:text-emphasis prose-headings:mt-3 prose-headings:mb-1
					prose-h1:text-sm prose-h2:text-xs prose-h3:text-xs prose-h4:text-xs prose-h5:text-xs prose-h6:text-xs"
			>
				<Markdown md={planText} plugins={[gfmPlugin()]} />
			</div>
		{/if}
		{#if message.needsConfirmation}
			<div
				class="flex flex-row items-center justify-end gap-2 px-3 py-2 border-t border-border-light"
			>
				<Button
					variant="default"
					size="xs"
					on:click={() => {
						if (message.tool_call_id) {
							aiChatManager.handleToolConfirmation(message.tool_call_id, false)
						}
					}}
				>
					Keep planning
				</Button>
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
					Approve & run
				</Button>
			</div>
		{/if}
	</div>
{:else if isPlanEnter}
	<div class="my-1 rounded-md border border-border-light bg-surface overflow-hidden">
		<div
			class="flex items-center gap-2 px-3 py-2 border-b border-border-light bg-surface-secondary/30"
		>
			{#if message.isLoading && !message.needsConfirmation}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
			{:else if planEnterStarted}
				<Check class="w-3.5 h-3.5 text-green-600" />
			{:else}
				<ClipboardList class="w-3.5 h-3.5 text-secondary" />
			{/if}
			<span class="text-primary font-medium text-xs">
				{planEnterStarted
					? 'Planning started'
					: message.error
						? 'Continuing without planning'
						: 'Start planning?'}
			</span>
		</div>
		{#if planReason}
			<div class="px-3 py-2 text-xs text-secondary leading-snug">
				{planReason}
			</div>
		{/if}
		{#if message.needsConfirmation}
			<div
				class="flex flex-row items-center justify-end gap-2 px-3 py-2 border-t border-border-light"
			>
				<Button
					variant="default"
					size="xs"
					on:click={() => {
						if (message.tool_call_id) {
							aiChatManager.handleToolConfirmation(message.tool_call_id, false)
						}
					}}
				>
					Not now
				</Button>
				<Button
					variant="accent"
					size="xs"
					on:click={() => {
						if (message.tool_call_id) {
							aiChatManager.handleToolConfirmation(message.tool_call_id, true)
						}
					}}
					startIcon={{ icon: ClipboardList }}
				>
					Start planning
				</Button>
			</div>
		{/if}
	</div>
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
