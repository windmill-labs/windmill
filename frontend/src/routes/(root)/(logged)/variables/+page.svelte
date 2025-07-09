<script>
	import { onMount } from 'svelte'
	import { goto } from '$app/navigation'

	onMount(() => {
		goto('/assets')
	})
</script>
