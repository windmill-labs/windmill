<script lang="ts">
	import { Markdown } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Loader2, CheckCircle2, AlertTriangle } from 'lucide-svelte'
	import CodeDisplay from '$lib/components/copilot/chat/script/CodeDisplay.svelte'
	import LinkRenderer from '$lib/components/copilot/chat/LinkRenderer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { type ChatMessage } from './FlowChatManager.svelte'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		message: ChatMessage
	}

	let { message }: Props = $props()

	// Parse content to detect S3 objects
	const s3Object: any | undefined = $derived.by(() => {
		if (message.message_type === 'assistant' && message.content) {
			try {
				const parsed = JSON.parse(message.content)
				// Check if it's a Windmill S3 object with type discriminator
				if (parsed?.type === 'windmill_s3_object' && parsed?.s3 && typeof parsed.s3 === 'string') {
					return parsed
				}
			} catch (e) {
				// Not JSON, treat as regular text
			}
		}
		return undefined
	})

	const messageClass = $derived.by(() => {
		const base = 'max-w-[90%] min-w-0 rounded-lg w-fit'
		if (message.message_type === 'user') {
			return `${base} ml-auto bg-surface-secondary p-3`
		}
		return `${base} mr-auto bg-surface border ${message.success !== false ? 'border-gray-200 dark:border-gray-600' : '!border-red-500'}`
	})
</script>

<div class={messageClass} data-message-id={message.id}>
	{#if message.step_name}
		<div class="bg-surface-secondary text-2xs text-tertiary mb-2 font-medium py-1 px-2 rounded-t-lg"
			>{message.step_name}</div
		>
	{/if}

	{#if message.message_type === 'user'}
		<p class="whitespace-pre-wrap text-sm break-words text-right">{message.content}</p>
	{:else if message.loading}
		<div class="flex items-center gap-2 text-tertiary">
			<Loader2 size={16} class="animate-spin" />
			<span>Processing...</span>
		</div>
	{:else if message.content}
		{#if s3Object}
			<div class="px-3 pb-3 {!message.step_name ? 'pt-3' : ''}">
				<DisplayResult result={s3Object} workspaceId={$workspaceStore} noControls={true} />
			</div>
		{:else}
			<div
				class="flex flex-row items-center gap-2 px-3 pb-3 text-sm {!message.step_name
					? 'pt-3'
					: ''} overflow-x-auto"
			>
				{#if message.message_type === 'tool'}
					{#if message.success !== false}
						<CheckCircle2 class="w-3.5 h-3.5 text-green-500" />
					{:else}
						<AlertTriangle class="w-3.5 h-3.5 text-red-500" />
					{/if}
				{/if}
				<div class="prose prose-sm dark:prose-invert break-words prose-headings:!text-base">
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
		{/if}
	{:else}
		<p class="text-tertiary text-sm">No result</p>
	{/if}
</div>
