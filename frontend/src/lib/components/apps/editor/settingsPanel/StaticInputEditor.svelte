<script lang="ts">
	import type { AppInputTransform } from '../../types'
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../componentsPanel/componentStaticValues'

	export let input: AppInputTransform
	export let canHide: boolean
</script>

{#if input.type === 'static'}
	{#if canHide}
		<Toggle bind:checked={input.visible} options={{ right: 'Visible' }} />
	{/if}

	{#if input.fieldType === 'number'}
		<input type="number" bind:value={input.value} />
	{:else if input.fieldType === 'textarea'}
		<textarea bind:value={input.value} />
	{:else if input.fieldType === 'boolean'}
		<Toggle bind:checked={input.value} />
	{:else if input.fieldType === 'select'}
		<select bind:value={input.value}>
			{#each staticValues[input.optionValuesKey] || [] as option}
				<option value={option}>
					{option}
				</option>
			{/each}
		</select>
	{:else}
		<input bind:value={input.value} />
	{/if}
{/if}
