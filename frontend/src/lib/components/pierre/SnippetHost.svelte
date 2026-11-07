<script lang="ts" generics="T">
	import type { Snippet } from 'svelte'

	let { snippet, arg }: { snippet: Snippet<[T]>; arg: T } = $props()
</script>

{@render snippet(arg)}
