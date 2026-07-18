<script lang="ts">
	import { Button } from '$lib/components/common'
	import { X } from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'

	// The single message typed while a turn was streaming, waiting to be
	// auto-sent when the turn finishes. Rendered above the whole input stack
	// (session bars, context badges, textarea) so it reads as "next in the
	// conversation". Pressing Enter again appends another line to it; clicking
	// the chip (or its X, or ArrowUp in the empty input) removes it and
	// restores its text into the input so nothing is lost.
	const aiChatManager = getAiChatManager()
</script>

{#if aiChatManager.queuedMessage}
	<div
		class="mb-1 flex flex-row items-start gap-1 rounded-md bg-surface-input px-3 py-2 opacity-60 hover:opacity-100 cursor-pointer"
		title={aiChatManager.queuedMessage}
		role="button"
		tabindex="0"
		onclick={() => aiChatManager.dequeueMessage()}
		onkeydown={(e) => {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault()
				aiChatManager.dequeueMessage()
			}
		}}
	>
		<div class="min-w-0 grow">
			<p class="text-xs text-secondary whitespace-pre-wrap line-clamp-2">
				{aiChatManager.queuedMessage}
			</p>
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
