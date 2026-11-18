<script lang="ts">
	import TestDevHeader from './TestDevHeader.svelte'

	let { children } = $props()
</script>

<TestDevHeader />
{@render children()}
