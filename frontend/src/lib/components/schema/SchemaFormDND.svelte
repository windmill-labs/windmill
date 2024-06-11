<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { SOURCES, TRIGGERS } from 'svelte-dnd-action'
	import SchemaForm from '../SchemaForm.svelte'
	import { GripVertical } from 'lucide-svelte'
	import type { Schema } from '$lib/common'

	export let keys: string[] = []
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

	let items = keys.map((key) => ({ id: key, value: key })) ?? []

	let dragDisabled: boolean = false

	function handleConsider(e) {
		const {
			items: newItems,
			info: { source, trigger }
		} = e.detail

		schema.order = newItems.map((x) => x.id)
		schema = schema
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

		schema.order = newItems.map((x) => x.id)
		schema = schema
		items = newItems

		if (source === SOURCES.POINTER) {
			dragDisabled = true
		}

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
<div class={$$props.class}>
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
		{schema}
		dndConfig={{
			items: items,
			dragDisabled: dragDisabled,
			flipDurationMs,
			dropTargetStyle: {},
			type: dndType ?? 'top-level'
		}}
	>
		<svelte:fragment slot="actions">
			<div
				tabindex={dragDisabled ? 0 : -1}
				class="w-4 h-4 cursor-move ml-2"
				on:mousedown={startDrag}
				on:touchstart={startDrag}
				on:keydown={handleKeyDown}
			>
				<GripVertical size={16} />
			</div>
		</svelte:fragment>
	</SchemaForm>
</div>
