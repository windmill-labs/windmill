<script lang="ts">
	import { createEventDispatcher, untrack } from 'svelte'
	import { dragHandle } from '@windmill-labs/svelte-dnd-action'
	import SchemaForm from '../SchemaForm.svelte'
	import { GripVertical } from 'lucide-svelte'
	import type { Schema } from '$lib/common'
	import { deepEqual } from 'fast-equals'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	import { generateRandomString, readFieldsRecursively, type DynamicInput } from '$lib/utils'
	interface Props {
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
		helperScript?: DynamicInput.HelperScript
		className?: string
		dndType?: string
		lightHeaderFont?: boolean
	}

	let {
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
		helperScript = undefined,
		diff = {},
		nestedClasses = '',
		isValid = $bindable(true),
		noVariablePicker = false,
		className = '',
		dndType = generateRandomString(),
		lightHeaderFont
	}: Props = $props()

	$effect.pre(() => {
		if (args == undefined) {
			args = {}
		}
	})
	const dispatch = createEventDispatcher()
	const flipDurationMs = 200

	let items = $state(computeItems())

	let dragDisabledState = $state(true)

	function computeItems() {
		return (
			($state.snapshot(schema?.order) ?? Object.keys(schema?.properties ?? {}) ?? []).map(
				(key) => ({
					id: key,
					value: key
				})
			) ?? []
		)
	}

	function updateItems() {
		const newItems = computeItems()
		if (!deepEqual(newItems, items)) {
			items = newItems
		}
	}

	function handleConsider(e) {
		dragDisabledState = false
		const { items: newItems } = e.detail
		items = $state.snapshot(newItems)
	}

	function handleFinalize(e) {
		const { items: newItems } = e.detail
		dragDisabledState = true
		items = $state.snapshot(newItems)
		const newOrder = items.map((item) => item.value)
		dispatch('reorder', newOrder)
	}
	$effect(() => {
		readFieldsRecursively(schema)
		dragDisabledState && untrack(() => updateItems())
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
	{helperScript}
	{className}
	bind:schema
	dndConfig={disableDnd
		? undefined
		: {
				items,
				flipDurationMs,
				dropTargetStyle: {},
				type: dndType
			}}
	{items}
	{diff}
	{nestedParent}
	{shouldDispatchChanges}
	bind:isValid
	{noVariablePicker}
	{lightHeaderFont}
>
	{#snippet actions()}
		{#if !disableDnd}
			<div class="w-4 h-6 cursor-move ml-2 handle mt-[9px]" aria-label="drag-handle" use:dragHandle>
				<GripVertical size={16} />
			</div>
		{/if}
	{/snippet}
</SchemaForm>
