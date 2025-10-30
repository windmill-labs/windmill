<script lang="ts">
	import { Button } from './common'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import { ChevronDown } from 'lucide-svelte'

	interface Props {
		open?: boolean
		text: string
		small?: boolean
		children?: import('svelte').Snippet
		class?: string
	}

	let {
		open = $bindable(false),
		text,
		small = false,
		children,
		class: className = ''
	}: Props = $props()
</script>

<div class={twMerge('flex', className)}>
	<Button
		variant="default"
		btnClasses="text-primary {small ? 'text-xs' : ''} "
		on:click={() => (open = !open)}
		endIcon={{ icon: ChevronDown, classes: open ? 'transform rotate-180' : '' }}
	>
		{text}
	</Button>
</div>
{#if open}
	<div transition:slide|local={{ duration: 100 }}>{@render children?.()}</div>
{/if}
