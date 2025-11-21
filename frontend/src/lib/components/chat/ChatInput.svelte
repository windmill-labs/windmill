<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ArrowUp } from 'lucide-svelte'
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
			input?: { class?: string; style?: string }
			button?: { class?: string; style?: string }
		}
	}

	let {
		value = $bindable(),
		placeholder = 'Type your message here...',
		disabled = false,
		onSend,
		onKeydown = undefined,
		customCss = undefined
	}: Props = $props()

	function handleKeydown(e: KeyboardEvent) {
		onKeydown?.(e)
	}
</script>

<div
	class="flex items-center gap-2 rounded-lg border border-gray-200 dark:border-gray-600 bg-surface-input px-3 py-2"
>
	<textarea
		bind:value
		use:autosize
		onkeydown={handleKeydown}
		onpointerdown={stopPropagation(bubble('pointerdown'))}
		{placeholder}
		{disabled}
		class="flex-1 min-h-[24px] max-h-32 resize-none !border-0 text-sm placeholder-gray-400 !outline-none !ring-0 p-0 !shadow-none focus:!border-0 focus:!outline-none focus:!ring-0 focus:!shadow-none"
		rows={3}
	></textarea>
	<Button
		color="blue"
		size="xs2"
		btnClasses="!rounded-full !p-1.5"
		startIcon={{ icon: ArrowUp }}
		disabled={!value.trim() || disabled}
		on:click={onSend}
		iconOnly
		title="Send message"
	/>
</div>

