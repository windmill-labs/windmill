<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputDefaultValue from '../helpers/InputDefaultValue.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']
	export let customCss: ComponentCustomCSS<'input'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement

	let defaultValue: number | undefined = undefined
	let placeholder: string | undefined = undefined

	let min: number | undefined = undefined
	let max: number | undefined = undefined
	let step = 1

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<number | null>
	}

	function handleInput() {
		const value = input?.value
		const num = isNaN(+value) ? null : +value
		outputs?.result.set(num)
	}

	$: input && handleInput()

	$: css = concatCustomCss($app.css?.numberinputcomponent, customCss)
</script>

<InputValue {id} input={configuration.step} bind:value={step} />
<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />
<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />
<InputDefaultValue bind:input {defaultValue} />

<AlignWrapper {render} {verticalAlignment}>
	<input
		class={twMerge(
			'windmillapp w-full py-1.5 text-sm focus:ring-indigo-100 px-2 mx-0.5',
			css?.input?.class ?? ''
		)}
		style={css?.input?.style ?? ''}
		bind:this={input}
		on:input={handleInput}
		on:focus={(e) => {
			e?.stopPropagation()
			window.dispatchEvent(new Event('pointerup'))
		}}
		{min}
		{max}
		{step}
		type="number"
		inputmode="numeric"
		pattern="\d*"
		{placeholder}
	/>
</AlignWrapper>
