<script lang="ts" context="module">
	export let openedDrawers: string[] = []
</script>

<script lang="ts">
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	import { zIndexes } from '$lib/zIndexes'

	import { createEventDispatcher } from 'svelte'

	export let open = false
	export let id = (Math.random() + 1).toString(36).substring(10)
	export let preventEscape = false

	if (open) {
		openedDrawers.push(id)
	}

	export let initialOffset = 0

	let offset = initialOffset

	export function toggleDrawer() {
		if (!open) {
			openDrawer()
		} else {
			closeDrawer()
		}
	}

	export function openDrawer() {
		open = true
		offset = openedDrawers.length - 1

		if (openedDrawers.includes(id)) {
			return
		}

		openedDrawers.push(id)
	}

	export function closeDrawer() {
		open = false
		offset = initialOffset
		// remove the last opened drawer
		openedDrawers = openedDrawers.filter((drawer) => drawer !== id)
	}

	export function isOpen() {
		return open
	}

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	function handleClickAway(e) {
		const last = openedDrawers[openedDrawers.length - 1]

		if (last === id) {
			e.stopPropagation()
			closeDrawer()
		}
	}

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Escape':
					if (
						(id == openedDrawers[openedDrawers.length - 1] || openedDrawers.length == 0) &&
						!preventEscape
					) {
						openedDrawers.pop()
						event.preventDefault()
						event.stopPropagation()
						event.stopImmediatePropagation()
						closeDrawer()
						break
					}
			}
		}
	}

	$: zIndex = zIndexes.disposables + offset

	$: dispatchIfMounted(open ? 'open' : 'close')
</script>

<svelte:window on:keydown={onKeyDown} />

<slot {handleClickAway} {zIndex} {closeDrawer} {open} />
