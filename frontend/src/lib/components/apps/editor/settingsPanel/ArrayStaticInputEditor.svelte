<script lang="ts">
	import { Button } from '$lib/components/common'
	import { GripVertical, Plus, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import type { InputType, StaticInput, StaticOptions } from '../../inputType'
	import SubTypeEditor from './SubTypeEditor.svelte'
	import { flip } from 'svelte/animate'
	import { dndzone, SOURCES, TRIGGERS } from 'svelte-dnd-action'
	import { generateRandomString, pluralize } from '$lib/utils'
	import Toggle from '$lib/components/Toggle.svelte'
	import QuickAddColumn from './QuickAddColumn.svelte'

	const flipDurationMs = 200

	export let componentInput: StaticInput<any[]>
	export let subFieldType: InputType | undefined = undefined
	export let selectOptions: StaticOptions['selectOptions'] | undefined = undefined

	const dispatch = createEventDispatcher()

	function addElementByType() {
		if (!Array.isArray(componentInput.value)) {
			componentInput.value = []
		}
		let value: any[] = componentInput.value
		if (subFieldType) {
			if (subFieldType === 'boolean') {
				value.push(false)
			} else if (subFieldType === 'number') {
				value.push(0)
			} else if (subFieldType === 'object') {
				value.push({})
			} else if (subFieldType === 'labeledresource' || subFieldType === 'labeledselect') {
				value.push({ value: 'value', label: 'label' })
			} else if (subFieldType === 'tab-select') {
				value.push({ id: '', index: 0 })
			} else if (
				subFieldType === 'text' ||
				subFieldType === 'textarea' ||
				// TODO: Add support for these types
				subFieldType === 'date' ||
				subFieldType === 'time' ||
				subFieldType === 'datetime'
			) {
				value.push('')
			} else if (subFieldType === 'select' && selectOptions) {
				value.push(selectOptions[0])
			} else if (subFieldType === 'ag-grid') {
				value.push({ field: 'newField', editable: true, flex: 1 })
			} else if (subFieldType === 'table-column') {
				value.push({ field: 'newColumn', headerName: 'New column', type: 'text' })
			} else if (subFieldType === 'plotly') {
				value.push({
					value: {
						type: 'static',
						fieldType: 'array',
						subFieldType: 'number',
						value: [2, 4, 5, 6]
					},
					name: 'New dataset',
					aggregation_method: 'sum',
					type: 'bar',
					toolip: '',
					color: `#${Math.floor(Math.random() * 0xffffff)
						.toString(16)
						.padEnd(6, '0')}`
				})
			} else if (subFieldType === 'chartjs') {
				value.push({
					value: {
						type: 'static',
						fieldType: 'array',
						subFieldType: 'number',
						value: [2, 4, 5, 6]
					},
					name: 'New dataset'
				})
			}
		} else {
			value.push('')
		}
		componentInput = componentInput

		if (componentInput.value) {
			items.push({
				value: componentInput.value[componentInput.value.length - 1],
				id: generateRandomString()
			})
		}
	}

	function deleteElementByType(index: number) {
		if (componentInput.value) {
			componentInput.value.splice(index, 1)
			items.splice(index, 1) // Add this
			items = items
			componentInput.value = componentInput.value
			dispatch('deleteArrayItem', { index })
		}
	}

	let dragDisabled = true

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

		// Ensure dragging is stopped on drag finish via pointer (mouse, touch)
		if (source === SOURCES.POINTER) {
			dragDisabled = true
		}

		const reorderedValues = items.map((item) => item.value)
		componentInput.value = reorderedValues
	}

	function startDrag(e) {
		// preventing default to prevent lag on touch devices (because of the browser checking for screen scrolling)
		e.preventDefault()
		dragDisabled = false
	}

	function handleKeyDown(e) {
		if ((e.key === 'Enter' || e.key === ' ') && dragDisabled) dragDisabled = false
	}

	let items = (Array.isArray(componentInput.value) ? componentInput.value : []).map(
		(item, index) => {
			return { value: item, id: generateRandomString() }
		}
	)

	$: items != undefined && handleItemsChange()

	function handleItemsChange() {
		componentInput.value = items.map((item) => item.value)
	}

	let raw: boolean = false
</script>

<div class="flex gap-2 flex-col mt-2 w-full">
	{#if Array.isArray(items) && componentInput.value}
		<div class="flex flex-row items-center justify-between">
			<div class="text-xs text-tertiary font-semibold">{pluralize(items.length, 'item')}</div>

			{#if subFieldType === 'ag-grid' || subFieldType === 'table-column'}
				<Toggle
					options={{
						right: 'Raw'
					}}
					size="xs"
					bind:checked={raw}
				/>
			{/if}
		</div>
		<section
			use:dndzone={{
				items,
				dragDisabled,
				flipDurationMs,
				dropTargetStyle: {}
			}}
			on:consider={handleConsider}
			on:finalize={handleFinalize}
		>
			{#each items as item, index (item.id)}
				<div animate:flip={{ duration: flipDurationMs }} class="border-0 outline-none w-full">
					<!-- svelte-ignore a11y-no-noninteractive-tabindex -->

					<div class="flex flex-row gap-2 items-center relative my-1 w-full">
						<div class="grow min-w-0">
							<SubTypeEditor
								subFieldType={raw ? 'object' : subFieldType}
								bind:componentInput
								bind:value={item.value}
								on:remove={() => deleteElementByType(index)}
							/>
						</div>

						<div class="flex justify-between flex-col items-center">
							<!-- svelte-ignore a11y-no-static-element-interactions -->
							<div
								tabindex={dragDisabled ? 0 : -1}
								class="w-4 h-4 cursor-move"
								on:mousedown={startDrag}
								on:touchstart={startDrag}
								on:keydown={handleKeyDown}
							>
								<GripVertical size={16} />
							</div>
							<button
								class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
								aria-label="Remove item"
								on:click|preventDefault|stopPropagation={() => deleteElementByType(index)}
							>
								<X size={14} />
							</button>
						</div>
					</div>
				</div>
			{/each}
		</section>
	{/if}
	<Button size="xs" color="light" startIcon={{ icon: Plus }} on:click={() => addElementByType()}>
		Add
	</Button>

	{#if subFieldType === 'table-column'}
		<QuickAddColumn
			columns={componentInput.value?.map((item) => item.field)}
			on:add={({ detail }) => {
				if (!componentInput.value) componentInput.value = []

				componentInput.value.push({ field: detail, headerName: detail, type: 'text' })
				componentInput = componentInput

				if (componentInput.value) {
					items.push({
						value: componentInput.value[componentInput.value.length - 1],
						id: generateRandomString()
					})
				}
			}}
		/>
	{/if}
</div>
