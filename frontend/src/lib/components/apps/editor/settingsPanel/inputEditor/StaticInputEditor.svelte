<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../../componentsPanel/componentStaticValues'
	import type { StaticAppInput } from '../../../inputType'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'

	export let componentInput: StaticAppInput | undefined
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
		{#if componentInput.format}
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
			<div class="border rounded-sm w-full">
				<SimpleEditor
					lang="json"
					code={JSON.stringify(componentInput.value, null, 2)}
					class="few-lines-editor"
					on:change={(e) => {
						if (componentInput?.type === 'static' && componentInput.value) {
							componentInput.value = JSON.parse(e.detail.code)
						}
					}}
				/>
			</div>
		{/if}
	{:else if componentInput.fieldType === 'array'}
		<ArrayStaticInputEditor bind:componentInput />
	{:else}
		<input bind:value={componentInput.value} />
	{/if}
{/if}
