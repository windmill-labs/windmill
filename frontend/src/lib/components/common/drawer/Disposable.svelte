<script lang="ts" context="module">
	let openedDrawers: string[] = []
</script>

<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	export let open = false
	export let id = (Math.random() + 1).toString(36).substring(10)

	let diff = 0

	export function toggleDrawer() {
		if (!open) {
			openDrawer()
		} else {
			closeDrawer()
		}
	}

	export function openDrawer() {
		openedDrawers.push(id)
		diff = openedDrawers.length - 1
		open = true
	}

	export function closeDrawer() {
		open = false
		diff = 0
		openedDrawers = openedDrawers.filter((x) => x != id)
	}

	export function isOpen() {
		return open
	}

	const dispatch = createEventDispatcher()

	function handleClickAway() {
		dispatch('clickAway')
		open = false
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

	$: zIndex = 1100 + diff

	$: open ? dispatch('open') : dispatch('close')
</script>

<svelte:window on:keydown={onKeyDown} />

<slot {handleClickAway} {zIndex} {closeDrawer} {open} />
