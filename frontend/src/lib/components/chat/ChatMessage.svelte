<script lang="ts">
	import { Markdown } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { Loader2, CheckCircle2, AlertTriangle } from 'lucide-svelte'
	import CodeDisplay from '$lib/components/copilot/chat/script/CodeDisplay.svelte'
	import LinkRenderer from '$lib/components/copilot/chat/LinkRenderer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		role: 'user' | 'assistant' | 'tool' | 'system'
		content: string
		loading?: boolean
		success?: boolean
		stepName?: string
		enableMarkdown?: boolean
		enableS3Display?: boolean
		customCss?: {
			userMessage?: { class?: string; style?: string }
			assistantMessage?: { class?: string; style?: string }
		}
	}

	let {
		role,
		content,
		loading = false,
		success = undefined,
		stepName = undefined,
		enableMarkdown = true,
		enableS3Display = true,
		customCss = undefined
	}: Props = $props()

	// Parse S3 objects if enabled
	const s3Object = $derived.by(() => {
		if (!enableS3Display || role !== 'assistant' || !content) return undefined
		try {
			const parsed = JSON.parse(content)
			if (parsed?.type === 'windmill_s3_object' && parsed?.s3) {
				return parsed
			}
		} catch (e) {
			// Not JSON
		}
		return undefined
	})

	const messageClass = $derived.by(() => {
		const base = 'max-w-[90%] min-w-0 rounded-lg w-fit break-words'
		if (role === 'user') {
			const userClass = `${base} ml-auto bg-surface-secondary p-3`
			return customCss?.userMessage?.class
				? `${userClass} ${customCss.userMessage.class}`
				: userClass
		}
		// assistant, tool, and system messages use the same styling
		const assistantClass = `${base} mr-auto bg-surface border ${success !== false ? 'border-gray-200 dark:border-gray-600' : '!border-red-500'}`
		return customCss?.assistantMessage?.class
			? `${assistantClass} ${customCss.assistantMessage.class}`
			: assistantClass
	})

	const messageStyle = $derived.by(() => {
		if (role === 'user') {
			return customCss?.userMessage?.style
		}
		return customCss?.assistantMessage?.style
	})
</script>

<div class={messageClass} style={messageStyle}>
	{#if stepName}
		<div
			class="bg-surface-secondary text-2xs text-tertiary mb-2 font-medium py-1 px-2 rounded-t-lg"
		>
			{stepName}
		</div>
	{/if}

	{#if role === 'user'}
		<p class="whitespace-pre-wrap text-sm text-right">{content}</p>
	{:else if loading}
		<div class="flex items-center gap-2 text-tertiary px-3 py-3">
			<Loader2 size={16} class="animate-spin" />
			<span>Processing...</span>
		</div>
	{:else if content}
		{#if s3Object}
			<div class="px-3 pb-3 {!stepName ? 'pt-3' : ''}">
				<DisplayResult result={s3Object} workspaceId={$workspaceStore} noControls={true} />
			</div>
		{:else if enableMarkdown}
			<div
				class="flex flex-row items-center gap-2 px-3 pb-3 text-sm {!stepName
					? 'pt-3'
					: ''} overflow-x-auto"
			>
				{#if role === 'tool'}
					{#if success !== false}
						<CheckCircle2 class="w-3.5 h-3.5 text-green-500" />
					{:else}
						<AlertTriangle class="w-3.5 h-3.5 text-red-500" />
					{/if}
				{/if}
				<div class="prose prose-sm dark:prose-invert break-words prose-headings:!text-base">
					<Markdown
						md={content}
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
			<p class="whitespace-pre-wrap text-sm px-3 pb-3 {!stepName ? 'pt-3' : ''}">{content}</p>
		{/if}
	{:else}
		<p class="text-tertiary text-sm px-3 py-3">No result</p>
	{/if}
</div>
