<script lang="ts">
	import { Button } from '$lib/components/common'
	import { X } from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'

	// The single message typed while a turn was streaming, waiting to be
	// auto-sent when the turn finishes. Rendered above the whole input stack
	// (session bars, context badges, textarea) so it reads as "next in the
	// conversation". Pressing Enter again appends another line to it; the X
	// removes it and restores its text into the input so nothing is lost.
	const aiChatManager = getAiChatManager()
</script>

<!-- Image-only queues have empty text; without the image row the queued draft
     would be invisible — undismissable, then auto-sent as a surprise turn. -->
{#if aiChatManager.queuedMessage || aiChatManager.queuedImages.length > 0}
	<div
		class="mb-1 flex flex-row items-start gap-1 rounded-md bg-surface-input px-3 py-2 opacity-60"
		title={aiChatManager.queuedMessage}
	>
		<div class="min-w-0 grow">
			{#if aiChatManager.queuedImages.length > 0}
				<div class="flex flex-row flex-wrap gap-1 {aiChatManager.queuedMessage ? 'mb-1' : ''}">
					{#each aiChatManager.queuedImages as image, i (i)}
						<img
							src={image.previewUrl ?? image.dataUrl}
							alt={image.name ?? 'queued image'}
							class="h-6 w-6 object-cover rounded border border-border-light"
						/>
					{/each}
				</div>
			{/if}
			{#if aiChatManager.queuedMessage}
				<p class="text-xs text-secondary whitespace-pre-wrap line-clamp-2">
					{aiChatManager.queuedMessage}
				</p>
			{/if}
		</div>
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
