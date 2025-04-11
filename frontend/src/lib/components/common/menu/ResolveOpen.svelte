<script lang="ts">
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	export let open: boolean = false

	$: dispatchIfMounted(open ? 'open' : 'close')
</script>
