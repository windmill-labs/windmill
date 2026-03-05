<script lang="ts">
	interface Props {
		children?: import('svelte').Snippet;
		headerAction?: import('svelte').Snippet;
	}

	let { children, headerAction }: Props = $props();
</script>

<thead class="bg-surface-secondary sticky top-0 z-10 border-b">
	{@render children?.()}
	<tr class="absolute top-2 right-2">
		{@render headerAction?.()}
	</tr>
</thead>
