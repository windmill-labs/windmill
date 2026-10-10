<script lang="ts">
	// The root layout leaves app.css to the layouts under it so that the public app routes
	// can skip it; every other top-level route lives in this group to get it.
	import '$lib/assets/app.css'
	import CustomCss from '$lib/components/CustomCss.svelte'

	let { children }: { children?: import('svelte').Snippet } = $props()
</script>

<CustomCss />
{@render children?.()}
