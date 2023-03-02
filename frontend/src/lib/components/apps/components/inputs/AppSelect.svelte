<script lang="ts">
	import { getContext } from 'svelte'
	import Select from 'svelte-select'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../../../../defaults'

	export const staticOutputs: string[] = ['result']
	export let id: string
	export let configuration: Record<string, AppInput>
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'input'> | undefined = undefined

	const { app, worldStore, connectingInput, selectedComponent } =
		getContext<AppEditorContext>('AppEditorContext')
	let items: { label: string; value: any }[]
	let multiple: boolean = false
	let placeholder: string = 'Select an item'

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string | undefined>
	}

	// $: outputs && handleOutputs()

	// function handleOutputs() {
	// 	value = outputs.result.peak()
	// }

	let value: string | undefined = outputs?.result.peak()

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
		const valuePeak = outputs?.result.peak()
		if (!valuePeak) {
			const value0 = items?.[0]?.['value']
			if (value0 != value) {
				value = JSON.stringify(value0)
				outputs?.result.set(value0)
			}
		} else {
			value = JSON.stringify(valuePeak)
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

	$: css = concatCustomCss($app.css?.selectcomponent, customCss)

	let defaultValue: any = undefined

	$: {
		if (defaultValue) {
			value = JSON.stringify(defaultValue)
			outputs?.result.set(defaultValue)
		}
	}
</script>

<InputValue {id} input={configuration.items} bind:value={items} />
<InputValue {id} input={configuration.multiple} bind:value={multiple} />
<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	<div class="app-select w-full mx-0.5" style="height: 34px" on:pointerdown|stopPropagation>
		<Select
			{multiple}
			on:clear={onChange}
			on:change={onChange}
			items={listItems}
			class={css?.input?.class ?? ''}
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={SELECT_INPUT_DEFAULT_STYLE.containerStyles + css?.input?.style}
			{value}
			{placeholder}
			on:click={() => {
				if (!$connectingInput.opened) {
					$selectedComponent = id
				}
			}}
			on:focus={() => {
				$selectedComponent = id
			}}
			floatingConfig={{
				strategy: 'fixed'
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
