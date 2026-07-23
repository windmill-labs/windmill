<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import type { DisplayMessage, ToolDisplayMessage } from './shared'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import AssistantMessage from './AssistantMessage.svelte'
	import { getAiChatManager } from './aiChatManagerContext'
	import { Button } from '$lib/components/common'
	import { RefreshCwIcon, Undo2Icon } from 'lucide-svelte'
	import AIChatInput from './AIChatInput.svelte'
	import { createAttachedFileContextElement, type ContextElement } from './context'
	import ToolExecutionDisplay from './ToolExecutionDisplay.svelte'
	import CompactionBoundary from './CompactionBoundary.svelte'
	import { messageDraft, segments } from './chatDraft'
	import { lineCountLabel } from './pasteTokens'
	import ExpandableImage from '$lib/components/common/image/ExpandableImage.svelte'

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
		message: DisplayMessage
		messageIndex: number
		editingMessageIndex: number | null
		isLast?: boolean
	}

	let {
		message,
		messageIndex,
		availableContext,
		editingMessageIndex = $bindable(null),
		isLast = false
	}: Props = $props()

	// The edit box edits a copy of THIS message's original context, not the live
	// selection: chips are one-shot and gone from the selection by edit time, so
	// binding the live selection would show the wrong chips and let a submit send
	// something other than what's displayed. Seeded on entering edit; the user's
	// add/remove here rides along on submit (see restartGeneration).
	let editContext = $state<ContextElement[]>([])

	function editMessage() {
		if (message.role !== 'user' || editingMessageIndex !== null || aiChatManager.loading) {
			return
		}
		editContext = [...(message.contextElements ?? [])]
		editingMessageIndex = messageIndex
	}
</script>

{#if message.role === 'summary'}
	<CompactionBoundary content={message.content} files={message.files} />
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
		<!-- One wrapping row for every badge on the message: selected context / DOM
		     picks and attached files, side by side. Clicks stay inside the row: the
		     wrapper's click opens edit mode, which would unmount a badge's preview
		     popover the instant it opens. -->
		{#if message.role === 'user' && editingMessageIndex !== messageIndex && ((message.contextElements?.length ?? 0) > 0 || (message.files?.length ?? 0) > 0)}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="flex flex-row flex-wrap gap-1 mb-1 px-2"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
			>
				{#each message.contextElements ?? [] as element}
					<ContextElementBadge contextElement={element} compact />
				{/each}
				<!-- Index in the key: same-named entries can survive in older transcripts. -->
				{#each message.files ?? [] as file, i (`${file.id ?? file.name}:${i}`)}
					<ContextElementBadge
						contextElement={createAttachedFileContextElement(file.name, file.content)}
						compact
					/>
				{/each}
			</div>
		{/if}
		{#if message.role === 'user' && editingMessageIndex === messageIndex}
			<div class="px-2 max-w-lg">
				<AIChatInput
					{availableContext}
					bind:selectedContext={editContext}
					initialInstructions={message.content}
					initialPastes={message.pastes}
					initialImages={aiChatManager.storedImages(messageIndex)}
					initialFiles={message.files}
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
					{#if message.role === 'user' && message.images && message.images.length > 0}
						<div class="flex flex-row flex-wrap gap-1.5 mb-1">
							{#each message.images as image, i (i)}
								<ExpandableImage
									src={image.dataUrl}
									alt={image.name ?? 'attached image'}
									class="max-h-40 max-w-[min(20rem,100%)] rounded-lg border border-border-light"
								/>
							{/each}
						</div>
					{/if}
					<!-- Text-free messages show only their context chips / images / file
					     chips — no empty bubble (empty sends require chips, images, or
					     files to go out). -->
					{#if message.content.trim() !== ''}
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
									><span class="block whitespace-pre-wrap mt-1">{seg.att.content}</span
									>{:else}<button
										type="button"
										class="px-1.5 py-0.5 rounded bg-surface-secondary text-secondary text-2xs"
										onclick={(e) => togglePaste(e, seg.att.id)}
										>Pasted {lineCountLabel(seg.att.lines)} · click to expand</button
									>{/if}{/each}
						</div>
					{/if}
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
