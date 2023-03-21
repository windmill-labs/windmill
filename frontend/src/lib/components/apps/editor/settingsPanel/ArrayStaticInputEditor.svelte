<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faPlus, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { InputType, StaticAppInput, StaticInput, StaticOptions } from '../../inputType'
	import SubTypeEditor from './SubTypeEditor.svelte'

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
			} else if (subFieldType === 'labeledresource') {
				value.push({ value: '', label: '' })
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
				<div>
					<SubTypeEditor {subFieldType} bind:componentInput bind:value />
				</div>
				<button
					transition:fade|local={{ duration: 100 }}
					class="z-10  rounded-full p-1 duration-200 hover:bg-gray-200"
					aria-label="Remove item"
					on:click|preventDefault|stopPropagation={() => deleteElementByType(index)}
				>
					<X size={14} />
				</button>
			</div>
		{/each}
	{/if}
	<Button size="xs" color="light" startIcon={{ icon: faPlus }} on:click={() => addElementByType()}>
		Add
	</Button>
</div>
