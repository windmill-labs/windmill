<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import { Button } from '$lib/components/common'
	import { GripVertical, Loader2, Plus, X } from 'lucide-svelte'
	import { createEventDispatcher, getContext, tick } from 'svelte'
	import type { InputType, StaticInput, StaticOptions } from '../../inputType'
	import SubTypeEditor from './SubTypeEditor.svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { generateRandomString, pluralize } from '$lib/utils'
	import Toggle from '$lib/components/Toggle.svelte'
	import QuickAddColumn from './QuickAddColumn.svelte'
	import RefreshDatabaseStudioTable from './RefreshDatabaseStudioTable.svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { findGridItem } from '../appUtilsCore'

	interface Props {
		componentInput: StaticInput<any[]> & { loading?: boolean }
		subFieldType?: InputType | undefined
		selectOptions?: StaticOptions['selectOptions'] | undefined
		id: string | undefined
	}

	let {
		componentInput = $bindable(),
		subFieldType = undefined,
		selectOptions = undefined,
		id
	}: Props = $props()

	const appContext = getContext<AppViewerContext>('AppViewerContext')
	const { app, selectedComponent } = appContext || {}

	let items: ReturnType<typeof getItems> = $state([])
	items = getItems(componentInput)

	const dispatch = createEventDispatcher()
	const flipDurationMs = 200

	function addElementByType() {
		if (!Array.isArray(componentInput.value)) {
			componentInput.value = []
		}
		let value: any[] = componentInput.value
		if (subFieldType) {
			if (subFieldType === 'boolean') {
				value.push(false)
			} else if (subFieldType === 'number') {
				value.push(1)
				value = value
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
			} else if (subFieldType === 'ag-chart') {
				value.push({
					value: {
						type: 'oneOf',
						selected: 'bar',
						labels: {
							bar: 'Bar',
							scatter: 'Scatter',
							line: 'Line',
							area: 'Area',
							'range-bar': 'Range Bar',
							'range-area': 'Range Area'
						},
						configuration: {
							bar: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							scatter: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							line: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							area: {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number',
									value: [25, 25, 50]
								}
							},
							'range-bar': {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number-tuple',
									value: [
										[10, 15],
										[20, 25],
										[18, 27]
									]
								}
							},
							'range-area': {
								value: {
									type: 'static',
									fieldType: 'array',
									subFieldType: 'number-tuple',
									value: [
										[10, 15],
										[20, 25],
										[18, 27]
									]
								}
							}
						}
					},
					name: 'New dataset'
				})
			} else if (subFieldType === 'number-tuple') {
				value.push([0, 5])
			}
		} else {
			value.push('')
		}

		if (componentInput.value) {
			let value = componentInput.value[componentInput.value.length - 1]
			if (value != undefined) {
				items.push({
					value,
					id: generateRandomString()
				})
			}
		}
		componentInput.value = $state.snapshot(componentInput.value)
		componentInput = componentInput
		items = getItems(componentInput)
	}

	async function updateConfiguration() {
		if (selectedComponent && id) {
			$selectedComponent = undefined
			await tick()
			$selectedComponent = [id]
		}
	}

	async function deleteElementByType(index: number) {
		if (componentInput.value) {
			const item = componentInput.value[index]
			// If deleting actions column, clear all table actions
			if (subFieldType === 'ag-grid' && item && item._isActionsColumn === true) {
				const gridItem = id ? findGridItem($app, id) : null
				if (
					gridItem &&
					(gridItem.data.type === 'aggridcomponent' ||
						gridItem.data.type === 'aggridcomponentee' ||
						gridItem.data.type === 'aggridinfinitecomponent' ||
						gridItem.data.type === 'aggridinfinitecomponentee') &&
					Array.isArray(gridItem.data.actions)
				) {
					gridItem.data.actions.length = 0
				}
				await updateConfiguration()
				return
			}

			componentInput.value.splice(index, 1)
			items.splice(index, 1)
			items = items
			componentInput.value = componentInput.value
			dispatch('deleteArrayItem', { index })
		}
	}

	function handleConsider(e) {
		const { items: newItems } = e.detail
		items = newItems
	}

	function handleFinalize(e) {
		const { items: newItems } = e.detail

		items = newItems

		const reorderedValues = items.map((item) => item.value)
		componentInput.value = reorderedValues
	}

	function getItems(componentInput: StaticInput<any[]> & { loading?: boolean }) {
		return (Array.isArray(componentInput.value) ? componentInput.value : [])
			.filter((x) => x != undefined)
			.map((item) => {
				const id: string = items?.find((x) => x === item)?.id ?? generateRandomString()
				return { value: item, id }
			})
	}

	function clearTableOnComponentReset(value: any[] | undefined) {
		if (Array.isArray(value) && value.length === 0 && items.length > 0) {
			items = []
		}
	}

	function handleItemsChange() {
		componentInput.value = items.map((item) => item.value).filter((item) => item != undefined)
	}

	let raw: boolean = $state(false)

	let refreshCount = $state(0)
	$effect(() => {
		subFieldType === 'db-explorer' && clearTableOnComponentReset(componentInput?.value)
	})
	$effect(() => {
		items != undefined && handleItemsChange()
	})

	const rnd = generateRandomString()
</script>

<div class="flex gap-2 flex-col mt-2 w-full">
	{#if Array.isArray(items) && componentInput.value}
		<div class="flex flex-row items-center justify-between mr-1">
			<div class="text-xs text-primary font-semibold">{pluralize(items.length, 'item')}</div>
			{#if subFieldType == 'labeledselect'}
				<Toggle
					options={{
						right: 'w/ value'
					}}
					size="xs"
					on:change={(e) => {
						if (e.detail) {
							items = items.map((item) => {
								if (typeof item.value === 'string') {
									return { ...item, value: { label: item.value, value: item.value } }
								} else {
									return item
								}
							})
						} else {
							items = items.map((item) => {
								if (typeof item.value === 'object' && item.value.hasOwnProperty('label')) {
									return { ...item, value: item.value.value }
								} else {
									return { ...item, value: JSON.stringify(item.value) }
								}
							})
						}
						refreshCount += 1
					}}
					checked={items.some((x) => typeof x.value != 'string')}
				/>
			{:else if subFieldType === 'ag-grid' || subFieldType === 'table-column' || subFieldType === 'db-explorer'}
				<Toggle
					options={{
						right: 'Raw'
					}}
					size="xs"
					bind:checked={raw}
				/>
			{/if}
		</div>
		{#key refreshCount}
			<section
				use:dragHandleZone={{
					items,
					flipDurationMs,
					dropTargetStyle: {},
					type: rnd
				}}
				onconsider={handleConsider}
				onfinalize={handleFinalize}
			>
				{#each items as item, index (item.id)}
					<div class="border-0 outline-none w-full">
						<!-- svelte-ignore a11y_no_noninteractive_tabindex -->

						<div class="flex flex-row gap-2 items-center relative my-1 w-full">
							<div class="grow min-w-0">
								<SubTypeEditor
									{id}
									subFieldType={raw ? 'object' : subFieldType}
									bind:componentInput
									bind:value={item.value}
									on:remove={() => deleteElementByType(index)}
								/>
							</div>

							<div class="flex justify-between flex-col items-center">
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<div class="w-4 h-4 cursor-move handle" use:dragHandle>
									<GripVertical size={16} />
								</div>
								{#if subFieldType !== 'db-explorer'}
									<button
										class="z-10 rounded-full p-1 duration-200 hover:bg-surface-hover"
										aria-label="Remove item"
										onclick={stopPropagation(preventDefault(() => deleteElementByType(index)))}
									>
										<X size={14} />
									</button>
								{/if}
							</div>
						</div>
					</div>
				{/each}
			</section>
		{/key}
	{/if}
	{#if subFieldType === 'db-explorer'}
		{#if componentInput.loading}
			<div class="flex flex-row gap-2 w-full items-center">
				<div class="flex flex-row gap-2 w-full items-center text-xs">
					<Loader2 class="animate-spin" size={14} />
					Loading columns defintions...
				</div>
			</div>
		{/if}
		<RefreshDatabaseStudioTable {id} />
	{/if}
	{#if subFieldType !== 'db-explorer'}
		<Button size="xs" color="light" startIcon={{ icon: Plus }} on:click={() => addElementByType()}>
			Add
		</Button>
		{#if subFieldType === 'table-column' || subFieldType == 'ag-grid'}
			<QuickAddColumn
				{id}
				columns={componentInput.value?.map((item) => item.field)}
				on:add={({ detail }) => {
					if (!componentInput.value) componentInput.value = []
					if (subFieldType === 'table-column') {
						componentInput.value.push({ field: detail, headerName: detail, type: 'text' })
					} else if (subFieldType === 'ag-grid') {
						componentInput.value.push({ field: detail, headerName: detail, flex: 1 })
					}

					if (componentInput.value) {
						let value = componentInput.value[componentInput.value.length - 1]
						if (value) {
							items.push({
								value,
								id: generateRandomString()
							})
						}
					}
					componentInput.value = $state.snapshot(componentInput.value)
					componentInput = componentInput
				}}
			/>
		{/if}
	{/if}
</div>
