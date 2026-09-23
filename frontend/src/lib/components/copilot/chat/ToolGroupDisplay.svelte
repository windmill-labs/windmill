<script lang="ts">
	import type { Snippet } from 'svelte'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import type { DisplayMessage, ToolDisplayMessage } from './shared'
	import { isFlowEditCall } from './toolGroups'

	interface Props {
		/** Flow path, or '' for the flow open in the editor. */
		target: string
		entries: { message: DisplayMessage; index: number }[]
		entry: Snippet<[DisplayMessage, number]>
	}

	let { target, entries, entry }: Props = $props()

	let expanded = $state(false)

	const calls = $derived(
		entries.map((e) => e.message).filter((m): m is ToolDisplayMessage => m.role === 'tool')
	)
	const running = $derived(calls.some((m) => m.isLoading || m.isQueued || m.isStreamingArguments))
	const editCount = $derived(calls.filter(isFlowEditCall).length)
	const failedCount = $derived(calls.filter((m) => m.error !== undefined).length)
	// Every edit of one flow carries the same chip, so the group shows it once.
	const previewCard = $derived(calls.findLast((m) => m.previewCard && !m.error)?.previewCard)
	// What the group is doing right now, so a collapsed run still names its current step.
	const liveLabel = $derived(
		running ? calls.findLast((m) => m.isLoading || m.isStreamingArguments)?.content : undefined
	)
</script>

{#snippet status()}
	<div class="flex items-center gap-2 shrink-0">
		{#if failedCount > 0}
			<span class="font-main text-2xs text-red-600 dark:text-red-400">{failedCount} failed</span>
		{/if}
		{#if previewCard && !running}
			<ToolPreviewCard card={previewCard} />
		{/if}
	</div>
{/snippet}

<ChatCollapsibleCard
	labelPrefix={running ? 'Editing' : 'Edited'}
	label="{target || 'the flow'} · {editCount} {editCount === 1 ? 'change' : 'changes'}"
	{expanded}
	onToggle={() => (expanded = !expanded)}
	shimmer={running}
	labelClass="truncate"
	headerRight={failedCount > 0 || (previewCard && !running) ? status : undefined}
	contentClass="border-0 border-l rounded-none bg-transparent p-0 pl-1 ml-1.5 mt-0.5"
>
	{#snippet belowHeader()}
		{#if liveLabel && !expanded}
			<div class="pl-3 text-2xs text-tertiary truncate">{liveLabel}</div>
		{/if}
	{/snippet}
	{#each entries as { message, index } (index)}
		{@render entry(message, index)}
	{/each}
</ChatCollapsibleCard>
