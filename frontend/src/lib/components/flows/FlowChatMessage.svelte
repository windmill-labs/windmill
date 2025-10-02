<script lang="ts">
	import { Markdown } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Loader2 } from 'lucide-svelte'
	import type { FlowConversationMessage } from '$lib/gen'
	import CodeDisplay from '$lib/components/copilot/chat/script/CodeDisplay.svelte'
	import LinkRenderer from '$lib/components/copilot/chat/LinkRenderer.svelte'

	interface Props {
		message: FlowConversationMessage & { loading?: boolean }
	}

	let { message }: Props = $props()
</script>

<div class="flex {message.message_type === 'user' ? 'justify-end' : 'justify-start'}">
	<div
		class="max-w-[90%] min-w-0 rounded-lg p-3 {message.message_type === 'user'
			? 'bg-surface-secondary'
			: 'bg-surface border border-gray-200 dark:border-gray-600'}"
	>
		{#if message.message_type === 'user'}
			<p class="whitespace-pre-wrap text-sm break-words">{message.content}</p>
		{:else if message.loading}
			<div class="flex items-center gap-2 text-tertiary">
				<Loader2 size={16} class="animate-spin" />
				<span>Processing...</span>
			</div>
		{:else if message.content}
			<div class="prose prose-sm dark:prose-invert prose-ul:!pl-6 break-words">
				<Markdown
					md={message.content}
					plugins={[
						gfmPlugin(),
						{
							renderer: {
								pre: CodeDisplay,
								a: LinkRenderer
							}
						}
					]}
				/>
			</div>
		{:else}
			<p class="text-tertiary text-sm">No result</p>
		{/if}
	</div>
</div>
