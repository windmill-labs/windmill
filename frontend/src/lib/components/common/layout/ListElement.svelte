<script lang="ts">
	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="w-full flex-grow bg-blue-300">
	{@render children?.()}
</div>
