<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../../componentsPanel/componentStaticValues'
	import type { StaticAppInput } from '../../../inputType'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import JsonEditor from './JsonEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '$lib/components/apps/types'
	import IconSelectInput from './IconSelectInput.svelte'
	import ColorInput from './ColorInput.svelte'
	import { Icon } from 'svelte-awesome'
	import { faDollarSign } from '@fortawesome/free-solid-svg-icons'

	export let componentInput: StaticAppInput | undefined

	const { onchange } = getContext<AppViewerContext>('AppViewerContext')
	const { pickVariableCallback } = getContext<AppEditorContext>('AppEditorContext')

	$: componentInput && onchange?.()
</script>

{#if componentInput?.type === 'static'}
	{#if componentInput.fieldType === 'number'}
		<input on:keydown|stopPropagation type="number" bind:value={componentInput.value} />
	{:else if componentInput.fieldType === 'textarea'}
		<textarea on:keydown|stopPropagation bind:value={componentInput.value} />
	{:else if componentInput.fieldType === 'date'}
		<input on:keydown|stopPropagation type="date" bind:value={componentInput.value} />
	{:else if componentInput.fieldType === 'boolean'}
		<Toggle bind:checked={componentInput.value} />
	{:else if componentInput.fieldType === 'select'}
		<select on:keydown|stopPropagation on:keydown|stopPropagation bind:value={componentInput.value}>
			{#each staticValues[componentInput.optionValuesKey] || [] as option}
				<option value={option}>
					{option}
				</option>
			{/each}
		</select>
	{:else if componentInput.fieldType === 'icon-select'}
		<IconSelectInput bind:componentInput />
	{:else if componentInput.fieldType === 'labeledresource'}
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

						if (componentInput && path) {
							componentInput.value['value'] = `$res:${path}`
						}
					}}
				/>
			</div>
		{:else}
			Inconsistent labeled resource object
		{/if}
	{:else if componentInput.fieldType === 'color'}
		<ColorInput bind:componentInput />
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
			<div class="flex w-full flex-col">
				<JsonEditor
					bind:value={componentInput.value}
					code={JSON.stringify(componentInput.value, null, 2)}
				/>
			</div>
		{/if}
	{:else if componentInput.fieldType === 'array'}
		<ArrayStaticInputEditor bind:componentInput on:deleteArrayItem />
	{:else}
		<div class="flex gap-1">
			<input
				on:keydown|stopPropagation
				type="text"
				placeholder="Static value"
				bind:value={componentInput.value}
			/>
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
		</div>
	{/if}
{/if}
