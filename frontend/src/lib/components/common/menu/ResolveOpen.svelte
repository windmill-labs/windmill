<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()
	export let open: boolean = false

	$: {
		if (open) {
			dispatch('open')
		} else {
			dispatch('close')
		}
	}
</script>
