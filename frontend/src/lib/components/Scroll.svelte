<script>
	import { twMerge } from 'tailwind-merge'
</script>

<div class={twMerge('h-full w-full overflow-y-auto scrollbar', $$props.class)}>
	<slot />
</div>

<style>
	/* Custom scrollbar styles */
	div::-webkit-scrollbar {
		width: 14px;
	}

	div::-webkit-scrollbar-track {
		background: rgb(241, 241, 241);
	}

	div::-webkit-scrollbar-thumb {
		background: rgb(209, 209, 209);
		border-left: 3px solid transparent;
		border-right: 3px solid transparent;
		background-clip: padding-box;
		border-radius: 10px;
	}

	div::-webkit-scrollbar-thumb:hover {
		background: rgb(136, 136, 136);
		border-left: 3px solid transparent;
		border-right: 3px solid transparent;
		background-clip: padding-box;
		border-radius: 10px;
	}

	/* Dark mode scrollbar styles */
	:global(.dark) div::-webkit-scrollbar-track {
		background: rgb(31, 31, 31);
	}

	:global(.dark) div::-webkit-scrollbar-thumb {
		background: rgb(75, 75, 75);
		border-left: 3px solid transparent;
		border-right: 3px solid transparent;
		background-clip: padding-box;
		border-radius: 10px;
	}

	:global(.dark) div::-webkit-scrollbar-thumb:hover {
		background: rgb(100, 100, 100);
		border-left: 3px solid transparent;
		border-right: 3px solid transparent;
		background-clip: padding-box;
		border-radius: 10px;
	}

	/* For Firefox */
	div {
		/* Prevent layout shift from scrollbar appearance */
		scrollbar-gutter: stable both-edges;
		/* Prevent horizontal scrollbar */
		overflow-x: hidden;
	}
</style>
