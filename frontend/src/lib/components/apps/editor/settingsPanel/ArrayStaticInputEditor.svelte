<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import type { InputType } from 'zlib'
	import type { AppInput } from '../../inputType'
	import SubTypeEditor from './SubTypeEditor.svelte'

	export let canHide: boolean = false
	export let componentInput: Extract<AppInput, { type: 'static'; fieldType: 'array' }>

	function addElementByType(subFieldType: InputType) {
		if (!componentInput.value) {
			if (subFieldType === 'string') {
				componentInput.value.push('')
			} else if (subFieldType === 'number') {
				componentInput.value.push(0)
			} else if (subFieldType === 'boolean') {
				componentInput.value.push(false)
			} else if (subFieldType === 'object') {
				componentInput.value.push({})
			} else if (subFieldType === 'array') {
				componentInput.value.push([])
			}
		}
	}
</script>

<div class="flex gap-2 flex-col mt-2">
	{#if componentInput.value}
		{#each componentInput.value as value, index (index)}
			<SubTypeEditor bind:componentInput bind:value {canHide} />
		{/each}
		<Button
			size="xs"
			color="dark"
			startIcon={{ icon: faPlus }}
			on:click={() => {
				if (componentInput.subFieldType) {
					addElementByType(componentInput.subFieldType)
				}
			}}
		>
			Add
		</Button>
	{/if}
</div>
