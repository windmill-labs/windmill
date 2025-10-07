<script lang="ts">
	import { Markdown } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Loader2, CheckCircle2 } from 'lucide-svelte'
	import type { FlowConversationMessage } from '$lib/gen'
	import CodeDisplay from '$lib/components/copilot/chat/script/CodeDisplay.svelte'
	import LinkRenderer from '$lib/components/copilot/chat/LinkRenderer.svelte'

	interface Props {
		message: FlowConversationMessage & { loading?: boolean; streaming?: boolean }
	}

	let { message }: Props = $props()
</script>

<div
	class={`flex ${message.message_type === 'user' ? 'justify-end' : 'justify-start'} ${message.loading || message.streaming ? 'min-h-[200px] items-start' : ''}`}
	data-message-id={message.id}
>
	<div
		class="max-w-[90%] min-w-0 rounded-lg {message.message_type === 'tool'
			? 'bg-surface border border-gray-200 dark:border-gray-700 overflow-hidden font-mono text-xs'
			: message.message_type === 'user'
				? 'bg-surface-secondary p-3'
				: 'bg-surface border border-gray-200 dark:border-gray-600 p-3'}"
	>
		{#if message.message_type === 'tool'}
			<!-- Tool message compact layout -->
			<div
				class="w-full p-3 bg-surface-secondary flex items-center gap-2 text-left border-b border-gray-200 dark:border-gray-700"
			>
				<CheckCircle2 class="w-3.5 h-3.5 text-green-500" />
				<span class="text-primary font-medium text-2xs">{message.content}</span>
			</div>
			{#if message.step_name}
				<div class="px-3 pt-2 pb-1 text-2xs text-tertiary font-medium">{message.step_name}</div>
			{/if}
		{:else}
			{#if message.step_name}
				<div class="text-2xs text-tertiary mb-2 font-medium">{message.step_name}</div>
			{/if}
			{#if message.message_type === 'user'}
				<p class="whitespace-pre-wrap text-sm break-words">{message.content}</p>
			{:else if message.loading}
				<div class="flex items-center gap-2 text-tertiary">
					<Loader2 size={16} class="animate-spin" />
					<span>Processing...</span>
				</div>
			{:else if message.content}
				<div
					class="prose prose-sm dark:prose-invert prose-ul:!pl-6 break-words whitespace-pre-wrap"
				>
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
		{/if}
	</div>
</div>
