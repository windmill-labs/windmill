<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { SOURCES, TRIGGERS } from 'svelte-dnd-action'
	import SchemaForm from '../SchemaForm.svelte'
	import { GripVertical } from 'lucide-svelte'
	import type { Schema } from '$lib/common'
	import { deepEqual } from 'fast-equals'

	export let dndType: string | undefined = undefined
	export let schema: Schema
	export let args: Record<string, any> = {}
	export let prettifyHeader: boolean = false
	export let lightweightMode: boolean = false
	export let onlyMaskPassword: boolean = false
	export let disablePortal: boolean = false
	export let disabled: boolean = false

	const dispatch = createEventDispatcher()
	const flipDurationMs = 200

	$: keys = schema.order ?? Object.keys(schema.properties ?? {}) ?? []

	let dragDisabled: boolean = false
	let items = keys?.map((key) => ({ id: key, value: key })) ?? []

	function updateItems() {
		const oldKeys = items.map((item) => item.value)
		if (!deepEqual(oldKeys, keys)) {
			items = keys.map((key) => ({ id: key, value: key }))
		}
	}

	$: keys && updateItems()

	function handleConsider(e) {
		const {
			items: newItems,
			info: { source, trigger }
		} = e.detail

		items = newItems
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

		dispatch(
			'reorder',
			items.map((item) => item.value)
		)
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
<SchemaForm
	on:click
	on:reorder
	on:consider={handleConsider}
	on:finalize={handleFinalize}
	{lightweightMode}
	{args}
	{prettifyHeader}
	{onlyMaskPassword}
	{disablePortal}
	{disabled}
	bind:schema
	dndConfig={{
		items,
		dragDisabled: dragDisabled,
		flipDurationMs,
		dropTargetStyle: {},
		type: dndType ?? 'top-level'
	}}
	{items}
>
	<svelte:fragment slot="actions">
		<div
			tabindex={dragDisabled ? 0 : -1}
			class="w-4 h-8 cursor-move ml-2"
			on:mousedown={startDrag}
			on:touchstart={startDrag}
			on:keydown={handleKeyDown}
		>
			<GripVertical size={16} />
		</div>
	</svelte:fragment>
</SchemaForm>
