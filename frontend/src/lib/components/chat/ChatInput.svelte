<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ArrowUp, Square } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()

	interface Props {
		value: string
		placeholder?: string
		disabled?: boolean
		onSend: () => void
		onKeydown?: (e: KeyboardEvent) => void
		customCss?: {
			container?: { class?: string; style?: string }
			input?: { class?: string; style?: string }
			button?: { class?: string; style?: string }
		}
		bindTextarea?: HTMLTextAreaElement
		showCancelButton?: boolean
		onCancel?: () => void
		cancelTitle?: string
		sendTitle?: string
	}

	let {
		value = $bindable(),
		placeholder = 'Type your message here...',
		disabled = false,
		onSend,
		onKeydown = undefined,
		customCss = undefined,
		bindTextarea = $bindable(undefined),
		showCancelButton = false,
		onCancel = undefined,
		cancelTitle = 'Cancel execution',
		sendTitle = 'Send message'
	}: Props = $props()

	function handleKeydown(e: KeyboardEvent) {
		onKeydown?.(e)
	}
</script>

<div
	class="flex items-center gap-2 rounded-lg border border-gray-200 dark:border-gray-600 bg-surface-input px-3 py-2 {customCss
		?.container?.class ?? ''}"
	style={customCss?.container?.style}
>
	<textarea
		bind:this={bindTextarea}
		bind:value
		use:autosize
		onkeydown={handleKeydown}
		onpointerdown={stopPropagation(bubble('pointerdown'))}
		{placeholder}
		class="flex-1 min-h-[24px] max-h-32 resize-none !border-0 text-sm placeholder-gray-400 !outline-none !ring-0 p-0 !shadow-none focus:!border-0 focus:!outline-none focus:!ring-0 focus:!shadow-none {customCss
			?.input?.class ?? ''}"
		style={customCss?.input?.style}
		rows={3}
	></textarea>
	{#if showCancelButton && onCancel}
		<Button
			color="red"
			size="xs2"
			btnClasses="!rounded-full !p-1.5 {customCss?.button?.class ?? ''}"
			style={customCss?.button?.style}
			startIcon={{ icon: Square }}
			on:click={onCancel}
			iconOnly
			title={cancelTitle}
		/>
	{:else}
		<Button
			color="blue"
			size="xs2"
			btnClasses="!rounded-full !p-1.5 {customCss?.button?.class ?? ''}"
			style={customCss?.button?.style}
			startIcon={{ icon: ArrowUp }}
			disabled={!value.trim() || disabled}
			on:click={onSend}
			iconOnly
			title={sendTitle}
		/>
	{/if}
</div>
