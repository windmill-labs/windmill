<script lang="ts" module>
	export let openedDrawers: { val: string[] } = $state({ val: [] })
</script>

<script lang="ts">
	import { zIndexes } from '$lib/zIndexes'
	import { untrack } from 'svelte'

	interface Props {
		open?: boolean
		id?: any
		preventEscape?: boolean
		initialOffset?: number
		children?: import('svelte').Snippet<[any]>
		onOpen?: () => void
		onClose?: () => void
	}

	let {
		open = $bindable(false),
		id = (Math.random() + 1).toString(36).substring(10),
		preventEscape = false,
		initialOffset = 0,
		children,
		onOpen,
		onClose
	}: Props = $props()

	let offset = $state(initialOffset)
	let zIndex = $derived(zIndexes.disposables + offset)

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
	}

	export function closeDrawer() {
		open = false
		offset = initialOffset
		if (openedDrawers.val.includes(id)) {
			openedDrawers.val = openedDrawers.val.filter((drawer) => drawer !== id)
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
		openedDrawers.val.push(id)
		offset = initialOffset + openedDrawers.val.length
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
