<script lang="ts">
	import { Skeleton } from '$lib/components/common'
	import { onMount } from 'svelte'

	let loaded = false

	onMount(async () => {
		//@ts-ignore

		await import('http://localhost/app.iife.js')
		loaded = false
	})
</script>

<div id="root" />

{#if !loaded}
	<Skeleton layout={[10]} />
{/if}
