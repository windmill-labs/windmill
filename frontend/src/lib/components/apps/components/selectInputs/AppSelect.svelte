<script lang="ts">
	import { getContext } from 'svelte'
	import Select from 'svelte-select'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export const staticOutputs: string[] = ['result']
	export let id: string
	export let configuration: Record<string, AppInput>
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const { worldStore, connectingInput, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')
	let label: string
	let items: string[]
	let itemKey: string
	let multiple: boolean = false
	let placeholder: string = 'Select an item'

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string | undefined>
	}

	let value = undefined

	$: items && handleItems()

	function handleItems() {
		if (items?.[0]?.['value'] != value) {
			value = items?.[0]?.['value']
			outputs?.result.set(value)
		}
	}

	function onChange(e: CustomEvent) {
		e?.stopPropagation()
		window.dispatchEvent(new Event('pointerup'))
		outputs?.result.set(e.detail?.[itemKey] || undefined)
	}
</script>

<InputValue {id} input={configuration.label} bind:value={label} />
<InputValue {id} input={configuration.items} bind:value={items} />
<InputValue {id} input={configuration.itemKey} bind:value={itemKey} />
<InputValue {id} input={configuration.multiple} bind:value={multiple} />
<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<div class="app-select w-full" style="height: 34px" on:pointerdown|stopPropagation>
		<Select
			--height="34px"
			{multiple}
			on:clear={onChange}
			on:change={onChange}
			items={Array.isArray(items) ? items : []}
			{value}
			{placeholder}
			on:click={() => {
				if (!$connectingInput.opened) {
					$selectedComponent = id
				}
			}}
		/>
	</div>
</AlignWrapper>

<style global>
	.app-select .value-container {
		@apply p-0 !important;
	}
</style>
