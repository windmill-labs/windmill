<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	let observer: MutationObserver
	onMount(() => {
		observer = new MutationObserver((mutationsList, observer) => {
			for (let mutation of mutationsList) {
				if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
					dispatch('change', document.documentElement.classList.contains('dark'))
				}
			}
		})

		observer.observe(document.documentElement, { attributes: true })
	})

	onDestroy(() => {
		observer.disconnect()
	})
</script>
