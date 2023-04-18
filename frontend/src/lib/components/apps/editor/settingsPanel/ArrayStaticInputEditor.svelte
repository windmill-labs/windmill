<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { GripVertical, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import type { InputType, StaticInput, StaticOptions } from '../../inputType'
	import SubTypeEditor from './SubTypeEditor.svelte'
	import { flip } from 'svelte/animate'
	import { dndzone, SOURCES, TRIGGERS } from 'svelte-dnd-action'

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
				value.push({ value: 'foo', label: 'bar' })
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
			}
		} else {
			value.push('')
		}
		componentInput = componentInput

		if (componentInput.value) {
			items.push({
				value: componentInput.value[componentInput.value.length - 1],
				id: generateRandomString(),
				originalIndex: componentInput.value.length - 1
			})
		}
	}

	function deleteElementByType(index: number) {
		if (componentInput.value) {
			componentInput.value.splice(index, 1)
			redraw = redraw + 1
			dispatch('deleteArrayItem', { index })

			// Remove the corresponding item from the items array
			items = items.filter((item) => item.originalIndex !== index)

			// Update the originalIndex of the remaining items
			items.forEach((item, i) => {
				item.originalIndex = i
			})
		}
	}

	let redraw = 0

	function generateRandomString() {
		let chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
		let result = ''

		for (let i = 0; i < 24; i++) {
			result += chars.charAt(Math.floor(Math.random() * chars.length))
		}

		return result
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

	let items =
		componentInput.value?.map((item, index) => {
			return { value: item, id: generateRandomString(), originalIndex: index }
		}) ?? []
</script>

<div class="flex gap-2 flex-col mt-2">
	{#key redraw}
		{#if Array.isArray(items) && componentInput.value}
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
					<div animate:flip={{ duration: flipDurationMs }} class="border-0 outline-none">
						<!-- svelte-ignore a11y-no-noninteractive-tabindex -->

						<div class="flex flex-row gap-2 items-center relative my-1">
							<SubTypeEditor
								{subFieldType}
								bind:componentInput
								bind:value={componentInput.value[index]}
							/>
							<div class="flex justify-between flex-col items-center">
								<div
									tabindex={dragDisabled ? 0 : -1}
									class="w-4 h-4"
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
	{/key}
	<Button size="xs" color="light" startIcon={{ icon: faPlus }} on:click={() => addElementByType()}>
		Add
	</Button>
</div>
