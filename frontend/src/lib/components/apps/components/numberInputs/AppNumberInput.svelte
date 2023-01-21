<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputDefaultValue from '../helpers/InputDefaultValue.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let input: HTMLInputElement

	let defaultValue: number | undefined = undefined
	let placeholder: string | undefined = undefined

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<number | null>
	}

	function handleInput() {
		const value = input?.value
		const num = isNaN(+value) ? null : +value
		outputs?.result.set(num)
	}

	$: input && handleInput()
</script>

<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />
<InputDefaultValue bind:input {defaultValue} />

<AlignWrapper {verticalAlignment}>
	<input
		bind:this={input}
		on:input={handleInput}
		on:focus={(e) => {
			e?.stopPropagation()
			window.dispatchEvent(new Event('pointerup'))
		}}
		type="number"
		inputmode="numeric"
		pattern="\d*"
		{placeholder}
	/>
</AlignWrapper>
