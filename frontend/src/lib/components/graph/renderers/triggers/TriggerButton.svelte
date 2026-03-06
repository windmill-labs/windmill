<script lang="ts">
	import { createBubbler } from 'svelte/legacy';

	const bubble = createBubbler();
	import { twMerge } from 'tailwind-merge'

	interface Props {
		selected: boolean;
		children?: import('svelte').Snippet;
	}

	let { selected, children }: Props = $props();
</script>

<button
	onclick={bubble('click')}
	class={twMerge(
		'hover:bg-surface-hover rounded-md border text-xs w-[23px] h-[23px] relative center-center cursor-pointer bg-surface',
		selected ? 'outline-1 outline-tertiary outline' : 'outline-0'
	)}
>
	{@render children?.()}
</button>
