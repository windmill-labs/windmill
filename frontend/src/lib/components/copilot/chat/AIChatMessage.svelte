<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { DisplayMessage, ToolDisplayMessage } from './shared'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import AssistantMessage from './AssistantMessage.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { Button } from '$lib/components/common'
	import { RefreshCwIcon, Undo2Icon } from 'lucide-svelte'
	import AIChatInput from './AIChatInput.svelte'
	import type { ContextElement } from './context'
	import ToolExecutionDisplay from './ToolExecutionDisplay.svelte'
	import CompactionBoundary from './CompactionBoundary.svelte'
	import { messageDraft, segments } from './chatDraft'
	import { lineCountLabel } from './pasteTokens'

	const aiChatManager = getAiChatManager()

	// Per-message expand/collapse state for paste chips shown in the bubble.
	let expandedPastes = $state<Set<number>>(new Set())

	function togglePaste(e: MouseEvent, id: number) {
		e.stopPropagation() // don't trigger edit-message on the bubble
		const next = new Set(expandedPastes)
		next.has(id) ? next.delete(id) : next.add(id)
		expandedPastes = next
	}

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		message: DisplayMessage
		messageIndex: number
		editingMessageIndex: number | null
		isLast?: boolean
	}

	let {
		message,
		messageIndex,
		availableContext,
		selectedContext = $bindable(),
		editingMessageIndex = $bindable(null),
		isLast = false
	}: Props = $props()

	function editMessage() {
		if (message.role !== 'user' || editingMessageIndex !== null || aiChatManager.loading) {
			return
		}
		editingMessageIndex = messageIndex
	}
</script>

{#if message.role === 'summary'}
	<CompactionBoundary content={message.content} />
{:else}
	<div
		class={twMerge(
			'mb-2 min-w-0',
			message.role === 'tool' && 'mb-1',
			message.role === 'user' && messageIndex > 0 && 'mt-4 mb-6',
			isLast && '!mb-12',
			message.role !== 'user' ? 'cursor-default' : 'cursor-pointer'
		)}
		role="button"
		tabindex="0"
		onclick={() => editMessage()}
		onkeydown={() => {}}
	>
		{#if message.role === 'user' && message.contextElements && editingMessageIndex !== messageIndex}
			<div class="flex flex-row gap-1 mb-1 overflow-scroll no-scrollbar px-2">
				{#each message.contextElements as element}
					<ContextElementBadge contextElement={element} compact />
				{/each}
			</div>
		{/if}
		{#if message.role === 'user' && editingMessageIndex === messageIndex}
			<div class="px-2 max-w-lg">
				<AIChatInput
					{availableContext}
					bind:selectedContext
					initialInstructions={message.content}
					initialPastes={message.pastes}
					{editingMessageIndex}
					onClickOutside={() => (editingMessageIndex = null)}
					onKeyDown={(e) => {
						if (e.key === 'Escape') {
							editingMessageIndex = null
						}
					}}
					onEditEnd={() => (editingMessageIndex = null)}
				/>
			</div>
		{:else}
			<div class={twMerge('text-sm py-1 px-2', message.role === 'tool' && 'text-primary py-0')}>
				{#if message.role === 'assistant'}
					<div class="px-[1px]"><AssistantMessage {message} /></div>
				{:else if message.role === 'tool'}
					<div class="px-[1px]"
						><ToolExecutionDisplay message={message as ToolDisplayMessage} /></div
					>
				{:else}
					<div
						class="text-xs px-3 py-2 w-fit max-w-[min(32rem,100%)] bg-surface-accent-selected text-accent rounded-lg relative group break-words"
					>
						{#each segments(messageDraft(message)) as seg}{#if seg.type === 'text'}<span
									class="whitespace-pre-wrap">{seg.value}</span
								>{:else if expandedPastes.has(seg.att.id)}<button
									type="button"
									class="my-0.5 px-1.5 py-0.5 rounded bg-surface-secondary text-secondary text-2xs"
									onclick={(e) => togglePaste(e, seg.att.id)}
									>{lineCountLabel(seg.att.lines)} · click to collapse</button
								><span class="block whitespace-pre-wrap mt-1">{seg.att.content}</span>{:else}<button
									type="button"
									class="px-1.5 py-0.5 rounded bg-surface-secondary text-secondary text-2xs"
									onclick={(e) => togglePaste(e, seg.att.id)}
									>Pasted {lineCountLabel(seg.att.lines)} · click to expand</button
								>{/if}{/each}
					</div>
				{/if}
			</div>
		{/if}
		{#if message.role === 'user' && message.snapshot}
			<div
				class="mx-2 text-2xs text-tertiary flex flex-row items-center justify-between gap-2 mt-2"
			>
				Saved {message.snapshot.type === 'flow' ? 'a flow' : 'an app'} snapshot
				<Button
					unifiedSize="xs"
					variant="subtle"
					on:click={() => {
						if (message.snapshot) {
							if (message.snapshot.type === 'flow') {
								aiChatManager.flowAiChatHelpers?.revertToSnapshot(message.snapshot.value)
							} else if (message.snapshot.type === 'app') {
								aiChatManager.appAiChatHelpers?.revertToSnapshot(message.snapshot.value)
							}
						}
					}}
					title="Revert to snapshot"
					startIcon={{ icon: Undo2Icon }}
				>
					Revert
				</Button>
			</div>
		{/if}
	</div>
	{#if message.role === 'user' && message.error}
		<div class="flex justify-end px-2 -mt-1">
			<Button
				size="xs2"
				variant="default"
				title="Retry generation"
				startIcon={{ icon: RefreshCwIcon }}
				onclick={() => aiChatManager.retryRequest(messageIndex)}
			>
				Retry
			</Button>
		</div>
	{/if}
{/if}
