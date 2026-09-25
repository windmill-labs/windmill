<script lang="ts">
	import type { ToolDisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolCodeDiffView from './ToolCodeDiffView.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import {
		argumentsCarryWholeFile,
		diffLineCounts,
		toolCodeDiff,
		toolDiffLineCounts,
		toolDiffLines
	} from './toolCodeDiff'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const diff = $derived(toolCodeDiff(message))
	const isStreaming = $derived(Boolean(message.isStreamingArguments))
	const hasDiff = $derived(Boolean(diff && (diff.before !== '' || diff.after !== '')))
	// The card body is a bordered box: opened with nothing in it, it draws an empty line.
	const hasBody = $derived(hasDiff || Boolean(message.error))

	// Keyed by call id: a bare flag would carry the expansion onto the next message that
	// reuses this instance. An active or failed call opens by itself, and remains open
	// when it settles so the new diff does not disappear under the user. A call whose arguments
	// are a whole file (`write_script`, full-code `edit_code`) opens only on failure. The key
	// also changes on failure, so a collapse made while running cannot hide the error.
	const toggleKey = $derived(`${message.tool_call_id}:${message.error ? 'error' : ''}`)
	let toggled = $state<{ id: string; open: boolean } | undefined>(undefined)
	const opensByDefault = $derived(
		Boolean(
			message.error ||
				(hasDiff &&
					!argumentsCarryWholeFile(message) &&
					(message.isLoading || message.isQueued || message.isStreamingArguments))
		)
	)
	$effect(() => {
		if (opensByDefault && toggled?.id !== toggleKey) {
			toggled = { id: toggleKey, open: true }
		}
	})
	const expanded = $derived(
		hasBody && (toggled?.id === toggleKey ? toggled.open : opensByDefault)
	)
	const lines = $derived(diff && expanded ? toolDiffLines(diff, isStreaming) : undefined)
	const counts = $derived(
		diff && expanded
			? isStreaming
				? diffLineCounts(diff, true)
				: toolDiffLineCounts(lines ?? [])
			: undefined
	)

	const isRunning = $derived(Boolean(message.isLoading && !message.needsConfirmation))
	const showPreviewChip = $derived(
		Boolean(message.previewCard && !message.isLoading && !message.error)
	)
</script>

{#snippet headerRight()}
	<div class="flex items-center gap-2 shrink-0">
		{#if counts && (counts.added > 0 || counts.removed > 0)}
			<span class="font-mono text-2xs">
				<span class="text-green-600 dark:text-green-400">+{counts.added}</span>
				<span class="text-red-600 dark:text-red-400">−{counts.removed}</span>
			</span>
		{/if}
		{#if showPreviewChip && message.previewCard}
			<ToolPreviewCard card={message.previewCard} />
		{/if}
	</div>
{/snippet}

<ChatCollapsibleCard
	label={message.content}
	{expanded}
	onToggle={() => (toggled = { id: toggleKey, open: !expanded })}
	toggleable={hasBody}
	shimmer={isRunning}
	class={message.isQueued && !message.error
		? 'opacity-60 hover:opacity-100 transition-opacity'
		: ''}
	labelClass="truncate"
	contentClass="p-0 overflow-hidden space-y-0"
	{headerRight}
>
	{#if diff && hasDiff && expanded}
		<ToolCodeDiffView {diff} diffLines={lines} streaming={isStreaming} />
	{/if}
	{#if message.error}
		<div
			class="px-3 py-2 text-2xs text-red-600 dark:text-red-400 whitespace-pre-wrap break-words {hasDiff
				? 'border-t border-border-light'
				: ''}"
		>
			{message.error}
		</div>
	{/if}
</ChatCollapsibleCard>
