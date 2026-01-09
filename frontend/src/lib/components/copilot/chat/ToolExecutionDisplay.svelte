<script lang="ts">
	import { Loader2, ChevronDown, ChevronRight, XCircle, Play } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { aiChatManager } from './AIChatManager.svelte'
	import type { ToolDisplayMessage } from './shared'
	import { twMerge } from 'tailwind-merge'
	import ToolContentDisplay from './ToolContentDisplay.svelte'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const hasParameters = $derived(
		message.parameters !== undefined && Object.keys(message.parameters).length > 0
	)

	let isExpanded = $derived(
		message.showDetails ||
			(message.isStreamingArguments && hasParameters) ||
			(message.isLoading && message.needsConfirmation)
	)
</script>

<div
	class="bg-surface border border-gray-200 dark:border-gray-700 rounded-md overflow-hidden font-mono text-xs"
>
	<!-- Collapsible Header -->
	<button
		class={twMerge(
			'w-full p-2 bg-surface-secondary hover:bg-surface-hover transition-colors flex items-center justify-between text-left border-b border-gray-200 dark:border-gray-700',
			message.needsConfirmation ? 'opacity-80' : ''
		)}
		onclick={() => (isExpanded = !isExpanded)}
		disabled={!message.showDetails && !message.isStreamingArguments}
	>
		<div class="flex items-center gap-2 flex-1">
			{#if message.showDetails || message.isStreamingArguments}
				{#if isExpanded}
					<ChevronDown class="w-3 h-3 text-secondary" />
				{:else}
					<ChevronRight class="w-3 h-3 text-secondary" />
				{/if}
			{/if}

			{#if message.isLoading && !message.needsConfirmation}
				<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
			{:else if message.error}
				<span class="text-red-500">✗</span>
			{:else if !message.isLoading && !message.error}
				<span class="text-green-500">✓</span>
			{/if}
			<span class="text-primary font-medium text-2xs">
				{message.content}
			</span>
		</div>
	</button>

	<!-- Expanded Content -->
	{#if isExpanded}
		<div class="p-2 bg-surface space-y-3">
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
				<div
					class={twMerge(
						'mt-3 pt-3 flex flex-row items-center justify-end gap-2',
						hasParameters ? 'border-t border-gray-200 dark:border-gray-700' : ''
					)}
				>
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
				/>

				<ToolContentDisplay
					title="Result"
					content={message.result}
					error={message.error}
					loading={message.isLoading}
				/>
			{/if}
		</div>
	{/if}
</div>
