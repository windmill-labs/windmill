<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { SOURCES, TRIGGERS, dndzone } from 'svelte-dnd-action'

	export let keys: string[] = []
	export let dndEnabled: boolean = true
	export let dndType: string

	const dispatch = createEventDispatcher()
	const flipDurationMs = 200

	let dragDisabled: boolean = false

	let items = keys.map((key) => ({ id: key, value: key })) ?? []

	// Whenever keys change, update items

	$: if (keys.length !== items.length) {
		items = keys.map((key) => ({ id: key, value: key }))
	}

	function handleConsider(e) {
		const {
			items: newItems,
			info: { source, trigger }
		} = e.detail

		items = newItems
		// Ensure dragging is stopped on drag finish via keyboard
		if (source === SOURCES.KEYBOARD && trigger === TRIGGERS.DRAG_STOPPED) {
			dragDisabled = true
		}
	}

	function handleFinalize(e) {
		const {
			items: newItems,
			info: { source }
		} = e.detail

		items = newItems

		if (source === SOURCES.POINTER) {
			dragDisabled = true
		}

		keys = items.map((item) => item.value)

		dispatch('finalize', keys)
	}

	function startDrag(e) {
		e.preventDefault()
		dragDisabled = false
	}

	function handleKeyDown(e) {
		if ((e.key === 'Enter' || e.key === ' ') && dragDisabled) dragDisabled = false
	}
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
<div
	class={$$props.class}
	use:dndzone={{
		items,
		dragDisabled: dragDisabled || !dndEnabled,
		flipDurationMs,
		dropTargetStyle: {},
		type: dndEnabled ? dndType ?? 'top-level' : 'dnd-disabled'
	}}
	on:consider={handleConsider}
	on:finalize={handleFinalize}
>
	<slot {items} {startDrag} {handleKeyDown} {dragDisabled} />
</div>
