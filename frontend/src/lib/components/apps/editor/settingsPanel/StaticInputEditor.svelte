<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../componentsPanel/componentStaticValues'
	import type { StaticAppInput } from '../../inputType'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import ArrayStaticInputEditor from './ArrayStaticInputEditor.svelte'

	export let componentInput: StaticAppInput | undefined
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
	{:else if componentInput.fieldType === 'object'}
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
	{:else if componentInput.fieldType === 'array'}
		<ArrayStaticInputEditor bind:componentInput {canHide} />
	{:else}
		<input bind:value={componentInput.value} />
	{/if}
{/if}
