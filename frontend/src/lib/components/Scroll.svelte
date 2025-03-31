<script>
	import { twMerge } from 'tailwind-merge'
</script>

<div class={twMerge('h-full w-full overflow-y-auto', $$props.class)}>
	<slot />
</div>

<style>
	/* Custom scrollbar styles */
	div::-webkit-scrollbar {
		width: 12px;
	}

	/* For Firefox */
	div {
		/* Prevent layout shift from scrollbar appearance */
		scrollbar-gutter: stable;
		/* Prevent horizontal scrollbar */
		overflow-x: hidden;
	}
</style>
