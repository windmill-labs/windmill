<script lang="ts">
	import { Markdown } from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import {
		Loader2,
		CheckCircle2,
		AlertTriangle,
		Brain,
		ChevronDown,
		ChevronRight
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'
	import CodeDisplay from '$lib/components/copilot/chat/script/CodeDisplay.svelte'
	import LinkRenderer from '$lib/components/copilot/chat/LinkRenderer.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { workspaceStore } from '$lib/stores'

	interface Props {
		role: 'user' | 'assistant' | 'tool' | 'system'
		content: string
		reasoning?: string
		streaming?: boolean
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
		reasoning = undefined,
		streaming = false,
		loading = false,
		success = undefined,
		stepName = undefined,
		enableMarkdown = true,
		enableS3Display = true,
		customCss = undefined
	}: Props = $props()

	// Reasoning / "thinking" affordance, mirroring the copilot chat's reasoning box.
	const reasoningText = $derived(role !== 'user' ? reasoning?.trim() || undefined : undefined)
	// Spinner while reasoning streams ahead of the answer.
	const reasoningStreaming = $derived(!!reasoningText && streaming && !content)
	// Expanded while thinking, collapse once the answer begins — unless toggled.
	let reasoningToggled = $state<boolean | undefined>(undefined)
	const reasoningExpanded = $derived(reasoningToggled ?? reasoningStreaming)

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

	{#if reasoningText}
		<!-- Reasoning / "thinking" affordance, consistent with the copilot chat -->
		<div
			class="mx-3 mt-3 mb-2 bg-surface border border-border-light rounded-md overflow-hidden text-xs"
		>
			<button
				class={twMerge(
					'w-full p-2 bg-surface-secondary/30 hover:bg-surface-hover transition-colors flex items-center gap-2 text-left',
					reasoningExpanded ? 'border-b border-border-light' : ''
				)}
				aria-expanded={reasoningExpanded}
				onclick={() => (reasoningToggled = !reasoningExpanded)}
			>
				{#if reasoningExpanded}
					<ChevronDown class="w-3 h-3 text-secondary" />
				{:else}
					<ChevronRight class="w-3 h-3 text-secondary" />
				{/if}
				{#if reasoningStreaming}
					<Loader2 class="w-3.5 h-3.5 animate-spin text-blue-500" />
				{:else}
					<Brain class="w-3.5 h-3.5 text-secondary" />
				{/if}
				<span class="text-primary font-medium text-2xs">Thinking</span>
			</button>

			{#if reasoningExpanded}
				<div
					transition:slide={{ duration: 150 }}
					class="p-2 bg-surface text-secondary break-words prose prose-sm dark:prose-invert max-w-full leading-snug
						prose-p:text-2xs prose-li:text-2xs prose-code:text-2xs prose-pre:text-2xs prose-ul:!pl-5
						prose-headings:font-medium prose-headings:text-secondary prose-headings:mt-2 prose-headings:mb-1
						prose-h1:text-2xs prose-h2:text-2xs prose-h3:text-2xs prose-h4:text-2xs prose-h5:text-2xs prose-h6:text-2xs
						prose-strong:text-secondary"
				>
					<Markdown md={reasoningText} plugins={[gfmPlugin()]} />
				</div>
			{/if}
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
	{:else if !reasoningText}
		<p class="text-tertiary text-sm px-3 py-3">No result</p>
	{/if}
</div>
