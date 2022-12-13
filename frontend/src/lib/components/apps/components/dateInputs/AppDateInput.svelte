<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let inputType: 'date' | 'time' | 'datetime-local'
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement
	let labelValue: string = 'Title'
	let minValue: string = ''
	let maxValue: string = ''

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<string>
	}

	function handleInput() {
		outputs?.result.set(input.value)
	}
</script>

<InputValue input={configuration.label} bind:value={labelValue} />
<InputValue input={configuration.minDate} bind:value={minValue} />
<InputValue input={configuration.maxDate} bind:value={maxValue} />

<!-- svelte-ignore a11y-label-has-associated-control -->
<label>
	<div>
		{labelValue}
	</div>
	<input
		type={inputType}
		bind:this={input}
		on:input={handleInput}
		min={minValue}
		max={maxValue}
		placeholder="Type..."
	/>
</label>
