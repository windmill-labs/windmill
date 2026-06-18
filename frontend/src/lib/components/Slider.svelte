<script lang="ts">
	import { ChevronDown } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import Button from './common/button/Button.svelte'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		text: string;
		tooltip?: string | undefined;
		view?: boolean;
		size?: 'xs' | 'sm' | 'md' | 'lg';
		children?: import('svelte').Snippet;
	}

	let {
		text,
		tooltip = undefined,
		view = $bindable(false),
		size = 'md',
		children
	}: Props = $props();
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
	<div class="my-4 px-2" transition:slide|local>{@render children?.()}</div>
{/if}
