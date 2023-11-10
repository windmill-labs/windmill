<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import Button from './common/button/Button.svelte'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	export let text: string
	export let tooltip: string | undefined = undefined
	export let view = false
	export let size: 'xs' | 'sm' | 'md' | 'lg' = 'md'
</script>

<Button
	color="light"
	on:click={() => (view = !view)}
	{size}
	variant="border"
	endIcon={{
		icon: ChevronDown,
		classes: twMerge('duration-300 ', view ? 'rotate-180' : 'rotate-0')
	}}
	>{text}
	{#if tooltip}
		<Tooltip wrapperClass="mx-1">{tooltip}</Tooltip>
	{/if}
</Button>
{#if view}
	<div class="my-4 px-2" transition:slide|local><slot /></div>
{/if}
