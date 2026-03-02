<script lang="ts">
	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="flex flex-col">
	<div class="overflow-hidden rounded-md border">
		<table class="min-w-full divide-y divide-gray-300">
			<tbody class="divide-y bg-surface">
				{@render children?.()}
			</tbody>
		</table>
	</div>
</div>
