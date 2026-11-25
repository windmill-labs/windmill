<script lang="ts">
	import autosize from '$lib/autosize'
	import { Button } from '$lib/components/common'
	import { ArrowUp } from 'lucide-svelte'

	let {
		position = undefined,
		label = undefined,
		ariaLabel,
		onSend,
		onClose
	}: {
		/** Fixed-position anchor (host viewport coords), top-left of the input. Without one, the
		 * box sits where its parent puts it. */
		position?: { x: number; y: number }
		/** Short name of what the prompt is about, shown as the placeholder hint. Without one, the
		 * hint says that sending leaves this page for a new AI session. */
		label?: string
		ariaLabel: string
		/** Resolving to `false` keeps the box open with its text. */
		onSend: (prompt: string) => void | Promise<boolean | void>
		onClose: () => void
	} = $props()

	let value = $state('')
	let sending = $state(false)
	let textarea = $state<HTMLTextAreaElement | undefined>(undefined)

	// Focus when the anchor first appears (its subject was just selected).
	$effect(() => {
		textarea?.focus()
	})

	export function focus() {
		textarea?.focus()
	}

	async function submit() {
		const v = value.trim()
		if (!v || sending) return
		sending = true
		try {
			if ((await onSend(v)) !== false) value = ''
		} finally {
			sending = false
		}
	}
</script>

<!-- Floating mini-composer anchored to a pick: an element of the preview, or selected code.
     Sends the prompt to the session chat (or a new session) with that pick as context. -->
<!-- The whole pill is the field: the border/focus-ring lives on this box and the
     send arrow sits inside it, centered against a single-line input. -->
<div
	class="{position
		? 'fixed z-[10000]'
		: ''} flex items-center gap-1 rounded-md border border-border-light bg-surface-input py-1 pl-1 pr-1 shadow-md transition-colors focus-within:border-border-selected"
	style={position
		? `left: ${position.x}px; top: ${position.y}px; width: 260px; max-width: calc(100vw - 16px);`
		: undefined}
	role="dialog"
	aria-label={ariaLabel}
>
	<textarea
		bind:this={textarea}
		bind:value
		use:autosize={{ maxHeight: '120px', minHeight: 20 }}
		rows={1}
		placeholder={label ? `Ask about ${label}…` : 'Ask in a new AI session…'}
		class="min-w-0 flex-1 resize-none !border-0 !bg-transparent !shadow-none px-1.5 !pt-[5px] !pb-[3px] text-xs leading-tight !ring-0 focus:!border-0 focus:!ring-0 focus:outline-none"
		onkeydowncapture={(e) => {
			// Capturing, as Monaco's own shortcuts run before a handler Svelte delegates to the root:
			// over a code editor, Ctrl+Enter would run the code.
			if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
				e.preventDefault()
				e.stopPropagation()
				submit()
			} else if (e.key === 'Escape') {
				e.preventDefault()
				// It closes the box alone, not the flow step an editor may be in.
				e.stopPropagation()
				onClose()
			}
		}}
	></textarea>
	<Button
		size="xs2"
		variant="accent"
		iconOnly
		startIcon={{ icon: ArrowUp }}
		disabled={sending || value.trim().length === 0}
		on:click={submit}
		title="Send"
	/>
</div>
