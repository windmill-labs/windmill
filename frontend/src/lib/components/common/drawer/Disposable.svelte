<script lang="ts" context="module">
	export let openedDrawers: string[] = []
</script>

<script lang="ts">
	import { zIndexes } from '$lib/zIndexes'

	import { createEventDispatcher } from 'svelte'

	export let open = false
	export let id = (Math.random() + 1).toString(36).substring(10)

	if (open) {
		openedDrawers.push(id)
	}

	let offset = 0

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
		offset = 0
		// remove the last opened drawer
		openedDrawers = openedDrawers.filter((drawer) => drawer !== id)
	}

	export function isOpen() {
		return open
	}

	const dispatch = createEventDispatcher()

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
					if (id == openedDrawers[openedDrawers.length - 1] || openedDrawers.length == 0) {
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

	$: open ? dispatch('open') : dispatch('close')
</script>

<svelte:window on:keydown={onKeyDown} />

<slot {handleClickAway} {zIndex} {closeDrawer} {open} />
