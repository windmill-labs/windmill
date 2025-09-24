<script lang="ts">
	interface Props {
		children?: import('svelte').Snippet<[any]>
	}

	let { children }: Props = $props()
</script>

<div class="hover:z-50 flex w-full h-8">
	{@render children?.({})}
</div>
