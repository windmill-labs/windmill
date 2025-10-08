<script lang="ts">
	import { Markdown } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Loader2, CheckCircle2, AlertTriangle } from 'lucide-svelte'
	import CodeDisplay from '$lib/components/copilot/chat/script/CodeDisplay.svelte'
	import LinkRenderer from '$lib/components/copilot/chat/LinkRenderer.svelte'
	import { type ChatMessage } from './FlowChatManager.svelte'

	interface Props {
		message: ChatMessage
	}

	let { message }: Props = $props()
</script>

<div
	class={`flex ${message.message_type === 'user' ? 'justify-end' : 'justify-start'} ${message.loading || message.streaming ? 'min-h-[200px] items-start' : ''}`}
	data-message-id={message.id}
>
	<div
		class="max-w-[90%] min-w-0 rounded-lg
			{message.message_type === 'user'
			? 'bg-surface-secondary p-3'
			: `bg-surface border ${message.success !== false ? 'border-gray-200 dark:border-gray-600' : '!border-red-500'}`}"
	>
		{#if message.step_name}
			<div
				class="bg-surface-secondary text-2xs text-tertiary mb-2 font-medium py-1 px-2 rounded-t-lg"
				>{message.step_name}</div
			>
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
				class="flex flex-row items-center gap-2 px-3 pb-3 text-sm {!message.step_name
					? 'pt-3'
					: ''}"
			>
				{#if message.message_type === 'tool'}
					{#if message.success !== false}
						<CheckCircle2 class="w-3.5 h-3.5 text-green-500" />
					{:else}
						<AlertTriangle class="w-3.5 h-3.5 text-red-500" />
					{/if}
				{/if}
				<div class="dark:prose-invert break-words whitespace-pre-wrap prose-headings:!text-base">
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
			</div>
		{:else}
			<p class="text-tertiary text-sm">No result</p>
		{/if}
	</div>
</div>
