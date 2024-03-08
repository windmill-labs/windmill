<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { createEventDispatcher } from 'svelte'

	export let darkMode: boolean = false
	const dispatch = createEventDispatcher()

	let observer: MutationObserver
	onMount(() => {
		darkMode = document.documentElement.classList.contains('dark')

		observer = new MutationObserver((mutationsList, observer) => {
			for (let mutation of mutationsList) {
				if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
					const newDarkMode = document.documentElement.classList.contains('dark')
					dispatch('change', newDarkMode)

					darkMode = newDarkMode
				}
			}
		})

		observer.observe(document.documentElement, { attributes: true })
	})

	onDestroy(() => {
		observer.disconnect()
	})
</script>
