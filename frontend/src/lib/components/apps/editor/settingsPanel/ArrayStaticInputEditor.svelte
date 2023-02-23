<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import type { StaticAppInput } from '../../inputType'
	import { staticValues } from '../componentsPanel/componentStaticValues'
	import SubTypeEditor from './SubTypeEditor.svelte'

	type ArrayComponentInput = Extract<StaticAppInput, { fieldType: 'array' }>

	export let componentInput: ArrayComponentInput
	const dispatch = createEventDispatcher()

	function addElementByType() {
		if (componentInput.subFieldType && componentInput.value) {
			if (componentInput.subFieldType === 'boolean') {
				componentInput.value.push(false)
			} else if (componentInput.subFieldType === 'number') {
				componentInput.value.push(0)
			} else if (componentInput.subFieldType === 'object') {
				componentInput.value.push({})
			} else if (
				componentInput.subFieldType === 'text' ||
				componentInput.subFieldType === 'textarea' ||
				// TODO: Add support for these types
				componentInput.subFieldType === 'date' ||
				componentInput.subFieldType === 'time' ||
				componentInput.subFieldType === 'datetime'
			) {
				componentInput.value.push('')
			} else if (componentInput.subFieldType === 'select') {
				componentInput.value.push(staticValues[componentInput.optionValuesKey][0])
			}
		}
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
	{#if componentInput.value}
		{#each componentInput.value as value, index (index)}
			<div class="flex flex-row gap-2 items-center relative">
				<SubTypeEditor bind:componentInput bind:value />

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
		<Button
			size="xs"
			color="light"
			startIcon={{ icon: faPlus }}
			on:click={() => addElementByType()}
		>
			Add
		</Button>
	{/if}
</div>
