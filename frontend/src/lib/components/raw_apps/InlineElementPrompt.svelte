<script lang="ts">
	import autosize from '$lib/autosize'
	import { Button } from '$lib/components/common'
	import { ArrowUp } from 'lucide-svelte'

	let {
		x,
		y,
		label,
		onSend,
		onClose
	}: {
		/** Fixed-position anchor (host viewport coords), top-left of the input. */
		x: number
		y: number
		/** Short element label shown as the placeholder hint (e.g. "button.btn"). */
		label: string
		onSend: (prompt: string) => void
		onClose: () => void
	} = $props()

	let value = $state('')
	let textarea = $state<HTMLTextAreaElement | undefined>(undefined)

	// Focus when the anchor first appears (the element was just selected).
	$effect(() => {
		textarea?.focus()
	})

	function submit() {
		const v = value.trim()
		if (!v) return
		onSend(v)
		value = ''
	}
</script>

<!-- Floating mini-composer anchored to the selected element in the preview. Sends
     the prompt to the session chat with that element as context. -->
<!-- The whole pill is the field: the border/focus-ring lives on this box and the
     send arrow sits inside it, centered against a single-line input. -->
<div
	class="fixed z-[10000] flex items-center gap-1 rounded-md border border-border-light bg-surface-input py-1 pl-1 pr-1 shadow-md transition-colors focus-within:border-border-selected"
	style="left: {x}px; top: {y}px; width: 260px; max-width: calc(100vw - 16px);"
	role="dialog"
	aria-label="Prompt about the selected element"
>
	<textarea
		bind:this={textarea}
		bind:value
		use:autosize={{ maxHeight: '120px', minHeight: 20 }}
		rows={1}
		placeholder={`Ask about ${label}…`}
		class="min-w-0 flex-1 resize-none !border-0 !bg-transparent !shadow-none px-1.5 !pt-[5px] !pb-[3px] text-xs leading-tight !ring-0 focus:!border-0 focus:!ring-0 focus:outline-none"
		onkeydown={(e) => {
			if (e.key === 'Enter' && !e.shiftKey) {
				e.preventDefault()
				submit()
			} else if (e.key === 'Escape') {
				e.preventDefault()
				onClose()
			}
		}}
	></textarea>
	<Button
		size="xs2"
		variant="accent"
		iconOnly
		startIcon={{ icon: ArrowUp }}
		disabled={value.trim().length === 0}
		on:click={submit}
		title="Send"
	/>
</div>
