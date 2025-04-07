<script>
	import { twMerge } from 'tailwind-merge'
</script>

<div
	class={twMerge('h-full w-full overflow-y-auto scrollbar', $$props.class)}
	style={$$props.style}
>
	<slot />
</div>

<style>
	div {
		/* Prevent layout shift from scrollbar appearance */
		scrollbar-gutter: stable both-edges;
	}
</style>
