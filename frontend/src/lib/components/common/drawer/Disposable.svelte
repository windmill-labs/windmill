<script lang="ts" module>
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
	import { onDestroy, untrack } from 'svelte'
	import { overlayHostActive, overlayStack } from '../overlayHost.svelte'

	const stack = overlayStack()
	const hostActive = overlayHostActive()

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
		if (stack.val.includes(id)) {
			return
		}
		stack.val.push(id)
		offset = initialOffset + stack.val.length
		if (minZIndex > 0) {
			minZIndexEntries[id] = minZIndex
		}
	}

	// A disposable can be unmounted while still open, by an ancestor that tears its whole
	// subtree down. Its id would then sit on the stack forever, and since the topmost entry
	// arbitrates Escape, every overlay opened afterwards would stop answering it.
	onDestroy(() => {
		stack.val = stack.val.filter((drawer) => drawer !== id)
		delete minZIndexEntries[id]
	})

	export function closeDrawer() {
		open = false
		offset = initialOffset
		if (stack.val.includes(id)) {
			stack.val = stack.val.filter((drawer) => drawer !== id)
			if (minZIndex > 0) {
				delete minZIndexEntries[id]
			}
		}
	}

	export function isOpen() {
		return open
	}

	/** Whether this is the overlay on top, i.e. the one a key press is for. Overlays that keep
	 *  Escape for themselves (`preventEscape`) have to ask, or they answer keys aimed at whatever
	 *  is stacked above them. Same condition the handler below arbitrates on. */
	export function isTopmost() {
		return stack.val.length === 0 || stack.val[stack.val.length - 1] === id
	}

	function handleClickAway(e) {
		const last = stack.val[stack.val.length - 1]
		if (last === id) {
			e.stopPropagation()
			closeDrawer()
		}
	}

	function onKeyDown(event: KeyboardEvent) {
		// Hidden hosts stay mounted and still receive window keys — see overlayHost.
		if (!hostActive()) return
		if (open) {
			switch (event.key) {
				case 'Escape':
					if ((id == stack.val[stack.val.length - 1] || stack.val.length == 0) && !preventEscape) {
						stack.val.pop()
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
		stack.val.push(untrack(() => id))
		offset = untrack(() => initialOffset) + stack.val.length
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
	isTop: stack.val[stack.val.length - 1] == id
})}
