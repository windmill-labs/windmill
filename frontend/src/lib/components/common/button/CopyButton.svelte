<script lang="ts">
	import { Check, Copy } from 'lucide-svelte'
	import Button from './Button.svelte'
	import { copyToClipboard } from '$lib/utils'

	// Icon-only copy-to-clipboard button: swaps to a green check for a moment
	// after a successful copy. Safe inside menus/tooltips (clicks don't propagate).
	let {
		value,
		sendToast = false,
		title = 'Copy to clipboard',
		class: className = ''
	}: {
		value: string
		// Also pop the global "copied to clipboard" toast on success.
		sendToast?: boolean
		// Hover title while not copied (e.g. "Copy id: wm-fork-x"); "Copied!"
		// always takes over after a copy.
		title?: string
		class?: string
	} = $props()

	let copied = $state(false)
	let resetTimeout: ReturnType<typeof setTimeout> | undefined

	async function copy() {
		if (!(await copyToClipboard(value, sendToast))) return
		copied = true
		clearTimeout(resetTimeout)
		resetTimeout = setTimeout(() => (copied = false), 1500)
	}
</script>

<Button
	variant="subtle"
	unifiedSize="xs"
	iconOnly
	title={copied ? 'Copied!' : title}
	startIcon={copied
		? { icon: Check, classes: 'text-green-600 dark:text-green-400' }
		: { icon: Copy }}
	onClick={copy}
	wrapperClasses={className}
/>
