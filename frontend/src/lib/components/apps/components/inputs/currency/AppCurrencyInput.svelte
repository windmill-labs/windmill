<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../../inputType'
	import type { Output } from '../../../rx'
	import type { AppEditorContext } from '../../../types'
	import AlignWrapper from '../../helpers/AlignWrapper.svelte'
	import InputValue from '../../helpers/InputValue.svelte'
	import CurrencyInput from './CurrencyInput.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let defaultValue: number | undefined = undefined

	let isNegativeAllowed: boolean | undefined = undefined
	let currency: string | undefined = undefined
	let locale: string | undefined = undefined
	let value: number | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<number | null>
	}

	function handleInput() {
		outputs?.result.set(value ?? null)
	}

	function handleDefault() {
		value = defaultValue
		handleInput()
	}

	$: value != undefined && handleInput()

	$: defaultValue != undefined && handleDefault()
</script>

<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />
<InputValue {id} input={configuration.isNegativeAllowed} bind:value={isNegativeAllowed} />
<InputValue {id} input={configuration.currency} bind:value={currency} />
<InputValue {id} input={configuration.locale} bind:value={locale} />

<AlignWrapper {verticalAlignment}>
	{#key isNegativeAllowed}
		{#key locale}
			{#key currency}
				<CurrencyInput
					inputClasses={{ formatted: 'p-0', wrapper: 'w-full', formattedZero: 'text-black' }}
					bind:value
					{currency}
					{locale}
					on:focus={(e) => {
						e?.stopPropagation()
					}}
					{isNegativeAllowed}
				/>
			{/key}
		{/key}
	{/key}
</AlignWrapper>
