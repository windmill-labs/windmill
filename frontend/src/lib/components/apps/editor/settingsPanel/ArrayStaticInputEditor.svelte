<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import type { InputType, StaticAppInput, StaticInput } from '../../inputType'
	import { staticValues } from '../componentsPanel/componentStaticValues'
	import SubTypeEditor from './SubTypeEditor.svelte'

	export let componentInput: StaticInput<any[]>
	export let subFieldType: InputType | undefined = undefined
	export let optionValuesKey: keyof typeof staticValues | undefined = undefined

	const dispatch = createEventDispatcher()

	function addElementByType() {
		console.log('ADD')
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
			} else if (subFieldType === 'labeledresource') {
				value.push({ value: '', label: '' })
			} else if (
				subFieldType === 'text' ||
				subFieldType === 'textarea' ||
				// TODO: Add support for these types
				subFieldType === 'date' ||
				subFieldType === 'time' ||
				subFieldType === 'datetime'
			) {
				value.push('')
			} else if (subFieldType === 'select' && optionValuesKey) {
				value.push(staticValues[optionValuesKey][0])
			}
		} else {
			value.push('')
		}

		console.log(value, componentInput.value)
		componentInput = componentInput
	}

	function deleteElementByType(index: number) {
		if (componentInput.value) {
			componentInput.value.splice(index, 1)

			dispatch('deleteArrayItem', { index })

			componentInput = componentInput
		}
	}
</script>

<div class="flex gap-2 flex-col mt-2">
	{#if Array.isArray(componentInput.value)}
		{#each componentInput.value as value, index (index)}
			<div class="flex flex-row gap-2 items-center relative">
				<SubTypeEditor {subFieldType} bind:componentInput bind:value />

				<div class="absolute top-1 right-1">
					<Button
						size="xs"
						color="light"
						variant="border"
						on:click={() => deleteElementByType(index)}
						iconOnly
						btnClasses="!text-red-500"
						startIcon={{ icon: faTrashAlt }}
					/>
				</div>
			</div>
		{/each}
	{/if}
	<Button size="xs" color="light" startIcon={{ icon: faPlus }} on:click={() => addElementByType()}>
		Add
	</Button>
</div>
