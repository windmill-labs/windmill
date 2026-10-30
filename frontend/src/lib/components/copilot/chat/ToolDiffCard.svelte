<script lang="ts">
	import type { ToolDisplayMessage } from './shared'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import ToolCodeDiffView from './ToolCodeDiffView.svelte'
	import ToolPreviewCard from './ToolPreviewCard.svelte'
	import ToolConfirmationFooter from './ToolConfirmationFooter.svelte'
	import OutOfScopeFooter from './OutOfScopeFooter.svelte'
	import { Check, XCircle } from 'lucide-svelte'
	import {
		diffIsWholeFile,
		diffLineCounts,
		toolCodeDiff,
		toolDiffLineCounts,
		toolDiffLines
	} from './toolCodeDiff'

	interface Props {
		message: ToolDisplayMessage
		hidePreviewChip?: boolean
	}

	let { message, hidePreviewChip = false }: Props = $props()

	const diff = $derived(toolCodeDiff(message))
	const isStreaming = $derived(Boolean(message.isStreamingArguments))
	const hasDiff = $derived(Boolean(diff && (diff.before !== '' || diff.after !== '')))
	const awaitingConfirmation = $derived(Boolean(message.isLoading && message.needsConfirmation))
	const pendingOutOfScope = $derived(
		message.isLoading && message.outOfScope && !message.outOfScope.decision
			? message.outOfScope
			: undefined
	)
	// Either answer row sits in the body, so the card stays open while one waits.
	const awaitingAnswer = $derived(awaitingConfirmation || !!pendingOutOfScope)
	// The card body is a bordered box: opened with nothing in it, it draws an empty line.
	const hasBody = $derived(hasDiff || Boolean(message.error) || awaitingAnswer)

	// Keyed by call id: a bare flag would carry the expansion onto the next message that
	// reuses this instance. An active or failed call opens by itself, and remains open
	// when it settles so the new diff does not disappear under the user. A call whose diff is a
	// whole file (`write_script`, full-code `edit_code`, raw app writes and deletes) opens only
	// on failure. The key also changes on failure, so a collapse made while running cannot hide
	// the error.
	const toggleKey = $derived(`${message.tool_call_id}:${message.error ? 'error' : ''}`)
	let toggled = $state<{ id: string; open: boolean } | undefined>(undefined)
	// A change the user declined is their decision, not a failure: it stays closed and neutral,
	// or its diff reads as a change that happened.
	const declined = $derived(Boolean(message.declinedByUser))
	const opensByDefault = $derived(
		Boolean(
			(message.error && !declined) ||
				(hasDiff &&
					!diffIsWholeFile(message) &&
					(message.isLoading || message.isQueued || message.isStreamingArguments))
		)
	)
	$effect(() => {
		if (opensByDefault && toggled?.id !== toggleKey) {
			toggled = { id: toggleKey, open: true }
		}
	})
	// The Apply/Reject footer is inside the body, so a pending confirmation keeps it open.
	const expanded = $derived(
		awaitingAnswer ||
			(hasBody && (toggled?.id === toggleKey ? toggled.open : opensByDefault))
	)
	const lines = $derived(diff && expanded ? toolDiffLines(diff, isStreaming) : undefined)
	const counts = $derived(
		diff && expanded
			? isStreaming
				? diffLineCounts(diff, true)
				: toolDiffLineCounts(lines ?? [])
			: undefined
	)

	const isRunning = $derived(Boolean(message.isLoading && !awaitingAnswer))
	const showPreviewChip = $derived(
		Boolean(!hidePreviewChip && message.previewCard && !message.isLoading && !message.error)
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
	toggleable={hasBody && !awaitingAnswer}
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
	{#if awaitingConfirmation}
		<ToolConfirmationFooter
			class="px-3 py-2 border-t border-border-light"
			toolCallId={message.tool_call_id}
			rejectIcon={XCircle}
			rejectDestructive
			confirmLabel="Apply"
			confirmIcon={Check}
		/>
	{:else if pendingOutOfScope}
		<OutOfScopeFooter
			class="px-3 py-2 border-t border-border-light"
			toolCallId={message.tool_call_id}
			path={pendingOutOfScope.path}
			root={pendingOutOfScope.root}
			scope={pendingOutOfScope.scope}
			verb={pendingOutOfScope.verb}
		/>
	{/if}
	{#if message.error}
		<div
			class="px-3 py-2 text-2xs {declined
				? 'text-secondary'
				: 'text-red-600 dark:text-red-400'} whitespace-pre-wrap break-words {hasDiff
				? 'border-t border-border-light'
				: ''}"
		>
			{message.error}
		</div>
	{/if}
</ChatCollapsibleCard>
