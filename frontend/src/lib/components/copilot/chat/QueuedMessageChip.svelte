<script lang="ts">
	import { Button } from '$lib/components/common'
	import { X } from 'lucide-svelte'
	import { getAiChatManager } from './aiChatManagerContext'

	// Messages typed while a turn was streaming, waiting to be auto-sent one
	// per turn in FIFO order. Rendered above the whole input stack (session
	// bars, context badges, textarea) so they read as "next in the
	// conversation". Each X removes that message from the queue and restores
	// its text into the input so nothing the user typed is lost.
	const aiChatManager = getAiChatManager()
</script>

{#if aiChatManager.queuedMessages.length > 0}
	<div class="mb-1 flex flex-col gap-1">
		{#each aiChatManager.queuedMessages as queued, index (index)}
			<div
				class="flex flex-row items-start gap-1 rounded-md bg-surface-input px-3 py-2 opacity-60"
				title={queued}
			>
				<div class="min-w-0 grow">
					<p class="text-xs text-secondary whitespace-pre-wrap line-clamp-2">
						{queued}
					</p>
				</div>
				<Button
					variant="subtle"
					unifiedSize="xs"
					iconOnly
					title="Remove queued message and put it back in the input"
					startIcon={{ icon: X }}
					on:click={() => aiChatManager.dequeueMessage(index)}
				/>
			</div>
		{/each}
	</div>
{/if}
