<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { dragHandle } from '@windmill-labs/svelte-dnd-action'
	import SchemaForm from '../SchemaForm.svelte'
	import { GripVertical } from 'lucide-svelte'
	import type { Schema } from '$lib/common'
	import { deepEqual } from 'fast-equals'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	interface Props {
		dndType?: string | undefined
		schema: Schema
		args?: Record<string, any>
		prettifyHeader?: boolean
		onlyMaskPassword?: boolean
		disablePortal?: boolean
		disabled?: boolean
		hiddenArgs?: string[]
		nestedParent?: { label: string; nestedParent: any | undefined } | undefined
		disableDnd?: boolean
		shouldDispatchChanges?: boolean
		diff?: Record<string, SchemaDiff>
		nestedClasses?: string
		isValid?: boolean
		noVariablePicker?: boolean
	}

	let {
		dndType = undefined,
		schema = $bindable(),
		args = $bindable(undefined),
		prettifyHeader = false,
		onlyMaskPassword = false,
		disablePortal = false,
		disabled = false,
		hiddenArgs = [],
		nestedParent = undefined,
		disableDnd = false,
		shouldDispatchChanges = false,
		diff = {},
		nestedClasses = '',
		isValid = $bindable(true),
		noVariablePicker = false
	}: Props = $props()

	$effect.pre(() => {
		if (args == undefined) {
			args = {}
		}
	})
	const dispatch = createEventDispatcher()
	const flipDurationMs = 200

	let items = $state(computeItems())

	let dragDisabled = $state(true)

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
	$effect(() => {
		schema && dragDisabled && updateItems()
	})
</script>

<!-- <pre class="text-xs">3 {JSON.stringify(schema, null, 2)}</pre> -->
<!-- {JSON.stringify(schema)}
{dragDisabled} -->
<!-- {JSON.stringify(items)} -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<SchemaForm
	{nestedClasses}
	{hiddenArgs}
	on:click
	on:change
	on:reorder
	on:consider={handleConsider}
	on:finalize={handleFinalize}
	on:acceptChange
	on:rejectChange
	on:nestedChange
	bind:args
	{prettifyHeader}
	{onlyMaskPassword}
	{disablePortal}
	{disabled}
	bind:schema
	dndConfig={disableDnd
		? undefined
		: {
				items,
				flipDurationMs,
				dropTargetStyle: {},
				type: dndType ?? 'top-level'
			}}
	{items}
	{diff}
	{nestedParent}
	{shouldDispatchChanges}
	bind:isValid
	{noVariablePicker}
>
	{#snippet actions()}
		{#if !disableDnd}
			<div class="w-4 h-8 cursor-move ml-2 handle" aria-label="drag-handle" use:dragHandle>
				<GripVertical size={16} />
			</div>
		{/if}
	{/snippet}
</SchemaForm>
