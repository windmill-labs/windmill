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
	export let inputType: 'date' | 'time' | 'datetime-local'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']
	export let customCss: ComponentCustomCSS<'input'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement
	let labelValue: string = 'Title'
	let minValue: string = ''
	let maxValue: string = ''
	let defaultValue: string | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string>
	}

	function handleInput() {
		outputs?.result.set(input.value)
	}

	$: input && handleInput()

	$: css = concatCustomCss($app.css?.dateinputcomponent, customCss)
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.minDate} bind:value={minValue} />
<InputValue {id} input={configuration.maxDate} bind:value={maxValue} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />

<InputDefaultValue bind:input {defaultValue} />

<AlignWrapper {render} {verticalAlignment}>
	<input
		type={inputType}
		bind:this={input}
		on:input={handleInput}
		min={minValue}
		max={maxValue}
		placeholder="Type..."
		class={twMerge('mx-0.5', css?.input?.class ?? '')}
		style={css?.input?.style ?? ''}
	/>
</AlignWrapper>
