<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import type { StaticAppInput } from '../../inputType'
	import { staticValues } from '../componentsPanel/componentStaticValues'
	import SubTypeEditor from './SubTypeEditor.svelte'

	type ArrayComponentInput = Extract<StaticAppInput, { fieldType: 'array' }>

	export let canHide: boolean = false
	export let componentInput: ArrayComponentInput

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
			componentInput = componentInput
		}
	}
</script>

<div class="flex gap-2 flex-col mt-2">
	{#if componentInput.value}
		{#each componentInput.value as value, index (index)}
			<div class="flex flex-row gap-2 items-center">
				<SubTypeEditor bind:componentInput bind:value {canHide} />

				<div>
					<Button
						size="xs"
						color="dark"
						on:click={() => deleteElementByType(index)}
						iconOnly
						startIcon={{ icon: faTrashAlt }}
					/>
				</div>
			</div>
		{/each}
		<Button size="xs" color="dark" startIcon={{ icon: faPlus }} on:click={() => addElementByType()}>
			Add
		</Button>
	{/if}
</div>
