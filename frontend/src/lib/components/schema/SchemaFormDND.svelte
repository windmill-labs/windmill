<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { dragHandle } from '@windmill-labs/svelte-dnd-action'
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

	let items = computeItems()

	let dragDisabled = true
	$: schema && dragDisabled && updateItems()

	function computeItems() {
		return (
			(schema?.order ?? Object.keys(schema?.properties ?? {}) ?? []).map((key) => ({
				id: key,
				value: key
			})) ?? []
		)
	}

	function updateItems() {
		const newItems = computeItems()
		if (!deepEqual(newItems, items)) {
			items = newItems
		}
	}

	function handleConsider(e) {
		dragDisabled = false
		const { items: newItems } = e.detail

		items = newItems
	}

	function handleFinalize(e) {
		const { items: newItems } = e.detail

		dragDisabled = true
		items = newItems

		dispatch(
			'reorder',
			items.map((item) => item.value)
		)
	}
</script>

<!-- {JSON.stringify(schema)}
{dragDisabled} -->
<!-- {JSON.stringify(items)} -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
<SchemaForm
	on:click
	on:change
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
		flipDurationMs,
		dropTargetStyle: {},
		type: dndType ?? 'top-level'
	}}
	{items}
>
	<svelte:fragment slot="actions">
		<div class="w-4 h-8 cursor-move ml-2 handle" aria-label="drag-handle" use:dragHandle>
			<GripVertical size={16} />
		</div>
	</svelte:fragment>
</SchemaForm>
