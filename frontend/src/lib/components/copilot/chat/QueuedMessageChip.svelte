<script lang="ts">
	import { Button } from '$lib/components/common'
	import { FileText, X } from 'lucide-svelte'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import { contextElementKey } from './context'
	import { getAiChatManager } from './aiChatManagerContext'

	// The single message typed while a turn was streaming, waiting to be
	// auto-sent when the turn finishes. Rendered above the whole input stack
	// (session bars, context badges, textarea) so it reads as "next in the
	// conversation". Pressing Enter again appends another line to it; clicking
	// the chip body (its X, or ArrowUp in the empty input) removes it and
	// restores its content into the input so nothing is lost.
	const aiChatManager = getAiChatManager()
</script>

<!-- Attachment-only and context-only queues have empty text; without their
     image / file / badge row the queued draft would be invisible —
     undismissable, then auto-sent as a surprise turn. Context badges render
     here only for context-ONLY queues: text queues pin the same chips, but
     those stay visible in the composer, and repeating them would read as two
     selections. -->
{#if aiChatManager.queuedMessage || aiChatManager.queuedImages.length > 0 || aiChatManager.queuedFiles.length > 0 || (aiChatManager.queuedContext?.length ?? 0) > 0}
	<!-- The body and the X are sibling buttons for the same action (an X inside a
	     clickable chip would be a nested interactive control, invalid ARIA). -->
	<div
		class="mb-1 flex flex-row items-start gap-1 rounded-md bg-surface-input px-3 py-2 opacity-60 hover:opacity-100"
	>
		{#if aiChatManager.queuedMessage || aiChatManager.queuedImages.length > 0 || aiChatManager.queuedFiles.length > 0}
			<button
				type="button"
				class="min-w-0 grow text-left cursor-pointer"
				title={aiChatManager.queuedMessage}
				aria-label="Remove queued message and put it back in the input"
				onclick={() => aiChatManager.dequeueMessage()}
			>
				{#if aiChatManager.queuedImages.length > 0}
					<div class="flex flex-row flex-wrap gap-1 {aiChatManager.queuedMessage ? 'mb-1' : ''}">
						{#each aiChatManager.queuedImages as image, i (i)}
							<img
								src={image.dataUrl}
								alt={image.name ?? 'queued image'}
								class="h-6 w-6 object-cover rounded border border-border-light"
							/>
						{/each}
					</div>
				{/if}
				{#if aiChatManager.queuedFiles.length > 0}
					<div class="flex flex-row flex-wrap gap-1 {aiChatManager.queuedMessage ? 'mb-1' : ''}">
						{#each aiChatManager.queuedFiles as file, i (i)}
							<span
								class="flex flex-row items-center gap-1 px-1.5 rounded border border-border-light text-2xs text-secondary max-w-36"
								title={file.name}
							>
								<FileText size={10} class="shrink-0" />
								<span class="truncate min-w-0">{file.name}</span>
							</span>
						{/each}
					</div>
				{/if}
				{#if aiChatManager.queuedMessage}
					<p class="text-xs text-secondary whitespace-pre-wrap line-clamp-2">
						{aiChatManager.queuedMessage}
					</p>
				{/if}
			</button>
		{:else if aiChatManager.queuedContext?.length}
			<!-- Context badges are interactive themselves (popover preview), so a
			     context-only queue gets a plain row instead of the clickable body —
			     nesting the badges in it would be invalid ARIA and a badge click
			     would dequeue out from under the opening popover. The X (and
			     ArrowUp in the empty input) still restores the queue. -->
			<div class="min-w-0 grow flex flex-row flex-wrap gap-1">
				{#each aiChatManager.queuedContext as element (contextElementKey(element))}
					<ContextElementBadge contextElement={element} compact />
				{/each}
			</div>
		{/if}
		<Button
			variant="subtle"
			unifiedSize="xs"
			iconOnly
			title="Remove queued message and put it back in the input"
			startIcon={{ icon: X }}
			on:click={() => aiChatManager.dequeueMessage()}
		/>
	</div>
{/if}
