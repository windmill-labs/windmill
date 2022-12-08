<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { staticValues } from '../componentsPanel/componentStaticValues'
	import type { AppInput } from '../../inputType'
	import Button from '$lib/components/common/button/Button.svelte'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'

	export let componentInput: AppInput | undefined
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
	{:else if componentInput.fieldType === 'array'}
		<div class="flex gap-2 flex-col mt-2">
			{#if componentInput.value}
				{#each componentInput.value as value, index}
					<div class="border rounded-sm">
						<SimpleEditor
							lang="json"
							code={JSON.stringify(componentInput.value[index], null, 2)}
							class="few-lines-editor"
							on:change={(e) => {
								if (componentInput?.type === 'static' && componentInput.value) {
									componentInput.value[index] = JSON.parse(e.detail.code)
								}
							}}
						/>
					</div>
				{/each}
				<Button
					size="xs"
					color="dark"
					startIcon={{ icon: faPlus }}
					on:click={() => {
						if (
							componentInput?.fieldType === 'array' &&
							componentInput.type === 'static' &&
							componentInput.value
						) {
							componentInput.value.push({})
						}
					}}
				>
					Add
				</Button>
			{/if}
		</div>
	{:else}
		<input bind:value={componentInput.value} />
	{/if}
{/if}
