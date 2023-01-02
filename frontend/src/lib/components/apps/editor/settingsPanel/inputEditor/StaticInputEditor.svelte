<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../../componentsPanel/componentStaticValues'
	import type { StaticAppInput } from '../../../inputType'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import JsonEditor from './JsonEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '$lib/components/apps/types'

	export let componentInput: StaticAppInput | undefined

	const { onchange } = getContext<AppEditorContext>('AppEditorContext')

	$: componentInput && onchange?.()
</script>

{#if componentInput?.type === 'static'}
	{#if componentInput.fieldType === 'number'}
		<input type="number" bind:value={componentInput.value} />
	{:else if componentInput.fieldType === 'textarea'}
		<textarea type="text" bind:value={componentInput.value} />
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
	{:else if componentInput.fieldType === 'object'}
		{#if componentInput?.format?.startsWith('resource-')}
			<ResourcePicker
				initialValue={componentInput.value?.split('$res:')[1] || ''}
				on:change={(e) => {
					let path = e.detail

					if (componentInput && path) {
						componentInput.value = `$res:${path}`
					}
				}}
				resourceType={componentInput.format.split('-').length > 1
					? componentInput.format.substring('resource-'.length)
					: undefined}
			/>
		{:else}
			<JsonEditor
				bind:value={componentInput.value}
				code={JSON.stringify(componentInput.value, null, 2)}
			/>
		{/if}
	{:else if componentInput.fieldType === 'array'}
		<ArrayStaticInputEditor bind:componentInput />
	{:else}
		<input type="text" placeholder="Static value" bind:value={componentInput.value} />
	{/if}
{/if}
