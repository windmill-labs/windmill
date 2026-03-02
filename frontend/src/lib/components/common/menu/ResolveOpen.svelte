<script lang="ts">
	import { run } from 'svelte/legacy';

	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)
	interface Props {
		open?: boolean;
	}

	let { open = false }: Props = $props();

	run(() => {
		dispatchIfMounted(open ? 'open' : 'close')
	});
</script>
