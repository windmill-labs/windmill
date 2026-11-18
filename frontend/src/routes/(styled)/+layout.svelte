<script lang="ts">
	// The root layout leaves app.css to the layouts under it so that the public app routes
	// can skip it; every other top-level route lives in this group to get it.
	import '$lib/assets/app.css'

	let { children }: { children?: import('svelte').Snippet } = $props()
</script>

{@render children?.()}
