<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../../componentsPanel/componentStaticValues'
	import type { InputType, StaticInput } from '../../../inputType'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import JsonEditor from './JsonEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '$lib/components/apps/types'
	import IconSelectInput from './IconSelectInput.svelte'
	import ColorInput from './ColorInput.svelte'
	import { Icon } from 'svelte-awesome'
	import { faDollarSign } from '@fortawesome/free-solid-svg-icons'

	export let componentInput: StaticInput<any> | undefined
	export let fieldType: InputType | undefined = undefined
	export let subFieldType: InputType | undefined = undefined
	export let optionValuesKey: keyof typeof staticValues | undefined = undefined
	export let format: string | undefined = undefined
	export let noVariablePicker: boolean = false

	const { onchange } = getContext<AppViewerContext>('AppViewerContext')
	const { pickVariableCallback } = getContext<AppEditorContext>('AppEditorContext')

	$: componentInput && onchange?.()
</script>

{#if componentInput?.type === 'static'}
	{#if fieldType === 'number'}
		<input on:keydown|stopPropagation type="number" bind:value={componentInput.value} />
	{:else if fieldType === 'textarea'}
		<textarea on:keydown|stopPropagation bind:value={componentInput.value} />
	{:else if fieldType === 'date'}
		<input on:keydown|stopPropagation type="date" bind:value={componentInput.value} />
	{:else if fieldType === 'boolean'}
		<Toggle bind:checked={componentInput.value} />
	{:else if fieldType === 'select' && optionValuesKey}
		<select on:keydown|stopPropagation on:keydown|stopPropagation bind:value={componentInput.value}>
			{#each staticValues[optionValuesKey] || [] as option}
				<option value={option}>
					{option}
				</option>
			{/each}
		</select>
	{:else if fieldType === 'icon-select'}
		<IconSelectInput bind:componentInput />
	{:else if fieldType === 'labeledresource'}
		{#if componentInput?.value && typeof componentInput?.value == 'object' && 'label' in componentInput?.value}
			<div class="flex flex-col gap-1 w-full">
				<input
					on:keydown|stopPropagation
					placeholder="Label"
					type="text"
					bind:value={componentInput.value['label']}
				/>
				<ResourcePicker
					initialValue={componentInput.value?.['value']?.split('$res:')[1] || ''}
					on:change={(e) => {
						let path = e.detail
						if (componentInput) {
							if (path) {
								componentInput.value['value'] = `$res:${path}`
							} else {
								componentInput.value['value'] = undefined
							}
						}
					}}
				/>
			</div>
		{:else}
			Inconsistent labeled resource object
		{/if}
	{:else if fieldType === 'color'}
		<ColorInput bind:componentInput />
	{:else if fieldType === 'object'}
		{#if format?.startsWith('resource-')}
			<ResourcePicker
				initialValue={componentInput.value?.split('$res:')[1] || ''}
				on:change={(e) => {
					let path = e.detail
					if (componentInput) {
						if (path) {
							componentInput.value = `$res:${path}`
						} else {
							componentInput.value = undefined
						}
					}
				}}
				resourceType={format.split('-').length > 1
					? format.substring('resource-'.length)
					: undefined}
			/>
		{:else}
			<div class="flex w-full flex-col">
				<JsonEditor
					bind:value={componentInput.value}
					code={JSON.stringify(componentInput.value, null, 2)}
				/>
			</div>
		{/if}
	{:else if fieldType === 'array'}
		<ArrayStaticInputEditor {subFieldType} bind:componentInput on:deleteArrayItem />
	{:else}
		<div class="flex gap-1">
			<input
				on:keydown|stopPropagation
				type="text"
				placeholder="Static value"
				bind:value={componentInput.value}
			/>
			{#if !noVariablePicker}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					class="min-w-min min-h-[34px] items-center leading-4 px-3 text-gray-600 cursor-pointer border border-gray-700 rounded center-center"
					on:click={() => {
						$pickVariableCallback = (variable) => {
							if (componentInput) {
								componentInput.value = `$var:${variable}`
							}
						}
					}}
					><Icon data={faDollarSign} />
				</div>
			{/if}
		</div>
	{/if}
{/if}
