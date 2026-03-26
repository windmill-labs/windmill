<script lang="ts" module>
	export let openedDrawers: { val: string[] } = $state({ val: [] })

	// When a disposable with minZIndex is open, all disposables use that as
	// their z-index base so that overlays opened on top (e.g. a Drawer from
	// inside a Modal) stack correctly above it.
	// We track per-id entries so concurrent modals don't clobber each other
	// (closing one must not reset the base while another is still open).
	let minZIndexEntries: Record<string, number> = $state({})
	let activeMinZIndex = $derived.by(() => {
		const values = Object.values(minZIndexEntries)
		return values.length > 0 ? Math.max(...values) : 0
	})
</script>

<script lang="ts">
	import { zIndexes } from '$lib/zIndexes'
	import { untrack } from 'svelte'

	interface Props {
		open?: boolean
		id?: any
		preventEscape?: boolean
		initialOffset?: number
		/** Minimum z-index base for this overlay. While any disposable with a
		 *  minZIndex is open, all disposables use that as their base so that
		 *  subsequent overlays stack above it (e.g. zIndexes.aiChat + 1 for
		 *  modals that need to render above the AI chat panel). */
		minZIndex?: number
		children?: import('svelte').Snippet<[any]>
		onOpen?: () => void
		onClose?: () => void
	}

	let {
		open = $bindable(false),
		id = (Math.random() + 1).toString(36).substring(10),
		preventEscape = false,
		initialOffset = 0,
		minZIndex = 0,
		children,
		onOpen,
		onClose
	}: Props = $props()

	let offset = $state(untrack(() => initialOffset))
	// Note: when a Modal with minZIndex is open, all disposables (including
	// already-open Drawers) are elevated. This is acceptable — relative
	// stacking order is preserved by the per-instance offset.
	let zIndex = $derived(Math.max(zIndexes.disposables, activeMinZIndex) + offset)

	export function toggleDrawer() {
		if (!open) {
			openDrawer()
		} else {
			closeDrawer()
		}
	}

	export function openDrawer() {
		open = true
		if (openedDrawers.val.includes(id)) {
			return
		}
		openedDrawers.val.push(id)
		offset = initialOffset + openedDrawers.val.length
		if (minZIndex > 0) {
			minZIndexEntries[id] = minZIndex
		}
	}

	export function closeDrawer() {
		open = false
		offset = initialOffset
		if (openedDrawers.val.includes(id)) {
			openedDrawers.val = openedDrawers.val.filter((drawer) => drawer !== id)
			if (minZIndex > 0) {
				delete minZIndexEntries[id]
			}
		}
	}

	export function isOpen() {
		return open
	}

	function handleClickAway(e) {
		const last = openedDrawers.val[openedDrawers.val.length - 1]
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
						(id == openedDrawers.val[openedDrawers.val.length - 1] ||
							openedDrawers.val.length == 0) &&
						!preventEscape
					) {
						openedDrawers.val.pop()
						event.preventDefault()
						event.stopPropagation()
						event.stopImmediatePropagation()
						closeDrawer()
						break
					}
			}
		}
	}

	if (open) {
		openedDrawers.val.push(untrack(() => id))
		offset = untrack(() => initialOffset) + openedDrawers.val.length
		if (minZIndex > 0) {
			minZIndexEntries[untrack(() => id)] = minZIndex
		}
	}

	let wasEverOpen = false
	let lastOpen = open
	$effect.pre(() => {
		if (open === untrack(() => lastOpen)) {
			return
		}
		lastOpen = open
		if (open) {
			// console.log('open', id, wasEverOpen)
			wasEverOpen = true
			onOpen?.()
		} else if (untrack(() => wasEverOpen)) {
			// console.log('close', id)
			onClose?.()
		}
	})
</script>

<svelte:window onkeydown={onKeyDown} />

{@render children?.({
	handleClickAway,
	zIndex,
	closeDrawer,
	open,
	isTop: openedDrawers.val[openedDrawers.val.length - 1] == id
})}
