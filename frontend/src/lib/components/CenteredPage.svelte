<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	interface Props {
		children?: import('svelte').Snippet;
		[key: string]: any
	}

	let { children, ...rest }: Props = $props();
</script>

<div class="pb-8">
	<div class={twMerge('max-w-7xl mx-auto px-4 sm:px-6 md:px-8', rest.class)}>
		{@render children?.()}
	</div>
</div>
