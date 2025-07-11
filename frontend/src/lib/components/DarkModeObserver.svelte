<script module lang="ts">
	import type { StateStore } from '$lib/utils'

	export function useIsDarkMode({
		onChange
	}: {
		onChange?: (newDarkMode: boolean) => void
	} = {}): StateStore<boolean> {
		let isDarkMode: StateStore<boolean> = $state({ val: false })
		let observer: MutationObserver | undefined = undefined

		onMount(() => {
			isDarkMode.val = document.documentElement.classList.contains('dark')

			observer = new MutationObserver((mutationsList) => {
				for (let mutation of mutationsList) {
					if (mutation.type === 'attributes' && mutation.attributeName === 'class') {
						const newDarkMode = document.documentElement.classList.contains('dark')
						onChange?.(newDarkMode)
						isDarkMode.val = newDarkMode
					}
				}
			})

			observer.observe(document.documentElement, { attributes: true })
		})

		onDestroy(() => {
			observer?.disconnect()
		})
		return isDarkMode
	}
</script>

<script lang="ts">
	import { onMount, onDestroy } from 'svelte'
	import { createEventDispatcher } from 'svelte'

	let { darkMode = $bindable(false) }: { darkMode?: boolean } = $props()
	const dispatch = createEventDispatcher()
	let isDarkMode = useIsDarkMode({
		onChange: (newDarkMode) => dispatch('change', newDarkMode)
	})
	$effect(() => {
		if (darkMode !== isDarkMode.val) {
			darkMode = isDarkMode.val
		}
	})
</script>
