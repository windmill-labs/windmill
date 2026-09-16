<script lang="ts">
	interface Props {
		children?: import('svelte').Snippet
		headerAction?: import('svelte').Snippet
	}

	let { children, headerAction }: Props = $props()
</script>

<!-- Stuck 1px above the scroller's edge rather than at it: at fractional device scale
     factors the edge and the header round to different pixels, and rows scrolling
     beneath show through the seam. Clipping the header's top row hides it. -->
<thead class="bg-surface-secondary sticky -top-px z-10 border-b">
	{@render children?.()}
	<tr class="absolute top-2 right-2">
		{@render headerAction?.()}
	</tr>
</thead>
