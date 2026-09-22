<!--
@component
A tool call that edited code, shown as a diff. Monaco loads only while the card is open,
so a transcript full of edits mounts no editor until one is asked for.
-->
<script lang="ts">
	import type { ToolDisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolCodeDiffView from './ToolCodeDiffView.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import { diffLineCounts, toolCodeDiff } from './toolCodeDiff'

	interface Props {
		message: ToolDisplayMessage
	}

	let { message }: Props = $props()

	const diff = $derived(toolCodeDiff(message))
	const counts = $derived(diff ? diffLineCounts(diff) : undefined)

	// Keyed by call id: a bare flag would carry the expansion onto the next message that
	// reuses this instance. An active or failed call opens by itself, and remains open
	// when it settles so the new diff does not disappear under the user.
	let toggled = $state<{ id: string; open: boolean } | undefined>(undefined)
	const opensByDefault = $derived(
		Boolean(message.isLoading || message.isQueued || message.isStreamingArguments || message.error)
	)
	$effect(() => {
		if (opensByDefault && toggled?.id !== message.tool_call_id) {
			toggled = { id: message.tool_call_id, open: true }
		}
	})
	const expanded = $derived(toggled?.id === message.tool_call_id ? toggled.open : opensByDefault)

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
	onToggle={() => (toggled = { id: message.tool_call_id, open: !expanded })}
	toggleable={diff !== undefined || message.error !== undefined}
	shimmer={isRunning}
	class={message.isQueued && !message.error
		? 'opacity-60 hover:opacity-100 transition-opacity'
		: ''}
	labelClass="truncate"
	contentClass="p-0 overflow-hidden space-y-0"
	{headerRight}
>
	{#if diff}
		<ToolCodeDiffView {diff} />
	{/if}
	{#if message.error}
		<div
			class="px-3 py-2 text-2xs text-red-600 dark:text-red-400 whitespace-pre-wrap break-words {diff
				? 'border-t border-border-light'
				: ''}"
		>
			{message.error}
		</div>
	{/if}
</ChatCollapsibleCard>
