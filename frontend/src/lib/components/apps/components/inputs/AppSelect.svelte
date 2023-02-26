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
	let items: { label: string; value: any }[]
	let multiple: boolean = false
	let placeholder: string = 'Select an item'

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string | undefined>
	}

	let value: string | undefined = undefined

	$: items && handleItems()

	let listItems: { label: string; value: string }[] = []

	function handleItems() {
		listItems = Array.isArray(items)
			? items.map((item) => {
					return {
						label: item.label,
						value: JSON.stringify(item.value)
					}
			  })
			: []
		if (items?.[0]?.['value'] != value) {
			value = JSON.stringify(items?.[0]?.['value'])
			outputs?.result.set(value)
		}
	}

	function onChange(e: CustomEvent) {
		e?.stopPropagation()
		window.dispatchEvent(new Event('pointerup'))
		let result: any = undefined
		try {
			result = JSON.parse(e.detail?.['value'])
		} catch (_) {}
		outputs?.result.set(result)
	}
</script>

<InputValue {id} input={configuration.items} bind:value={items} />
<InputValue {id} input={configuration.multiple} bind:value={multiple} />
<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<div class="app-select w-full" style="height: 34px" on:pointerdown|stopPropagation>
		<Select
			--height="34px"
			{multiple}
			on:clear={onChange}
			on:change={onChange}
			items={listItems}
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
		padding: 0 !important;
	}
	.svelte-select-list {
		z-index: 1000 !important;
	}
</style>
