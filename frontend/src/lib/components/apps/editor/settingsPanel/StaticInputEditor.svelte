<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../componentsPanel/componentStaticValues'
	import type { ComponentInput } from '../../inputType'

	export let componentInput: ComponentInput | undefined
	export let canHide: boolean = false
</script>

{#if componentInput?.type === 'static'}
	{#if canHide}
		<Toggle bind:checked={componentInput.visible} options={{ right: 'Visible' }} />
	{/if}

	{#if componentInput.fieldType === 'number'}
		<input type="number" bind:value={componentInput.value} />
	{:else if componentInput.fieldType === 'textarea'}
		<textarea bind:value={componentInput.value} />
	{:else if componentInput.fieldType === 'boolean'}
		<Toggle bind:checked={componentInput.value} />
	{:else if componentInput.fieldType === 'select'}
		<select bind:value={componentInput.value}>
			{#each staticValues[componentInput.optionValuesKey] || [] as option}
				<option value={option}>
					{option}
				</option>
			{/each}
		</select>
	{:else}
		<input bind:value={componentInput.value} />
	{/if}
{/if}
