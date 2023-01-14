<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let inputType: 'date' | 'time' | 'datetime-local'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement
	let labelValue: string = 'Title'
	let minValue: string = ''
	let maxValue: string = ''
	let disabled: boolean = false

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string>
	}

	function handleInput() {
		outputs?.result.set(input.value)
	}
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />
<InputValue {id} input={configuration.minDate} bind:value={minValue} />
<InputValue {id} input={configuration.maxDate} bind:value={maxValue} />
<InputValue {id} input={configuration.disabled} bind:value={disabled} />

<AlignWrapper {verticalAlignment}>
	<input
		type={inputType}
		bind:this={input}
		on:input={handleInput}
		min={minValue}
		max={maxValue}
		placeholder="Type..."
		class="h-full"
		{disabled}
	/>
</AlignWrapper>
