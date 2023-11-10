<script>
	import { createEventDispatcher } from 'svelte'
	import Button from './button/Button.svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let noBg = false
	const dispatch = createEventDispatcher()
</script>

<Button
	on:click={() => dispatch('close')}
	startIcon={{ icon: X }}
	iconOnly
	size="sm"
	color="light"
	btnClasses={twMerge(
		'hover:bg-surface-hover w-8 h-8 rounded-full p-0',
		noBg ? '' : 'bg-surface-secondary'
	)}
/>
