<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Button from './button/Button.svelte'
	import { X } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		noBg?: boolean
		small?: boolean
		Icon?: any | undefined
		class?: string
	}

	let { noBg = false, small = false, Icon, class: className }: Props = $props()

	const dispatch = createEventDispatcher()
</script>

<Button
	on:click={() => dispatch('close')}
	startIcon={{ icon: Icon ?? X }}
	iconOnly
	size="sm"
	color="light"
	btnClasses={twMerge(
		'hover:bg-surface-hover rounded-full p-0',
		noBg ? '' : 'bg-surface-secondary',
		small ? 'w-6 h-6' : 'w-8 h-8',
		className ?? ''
	)}
/>
