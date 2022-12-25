<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let value: number
	let labelValue: string = 'Title'

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<number | null>
	}
	$: if (value || !value) {
		// Disallow 'e' character in numbers
		// if(value && value.toString().includes('e')) {
		// 	value = +value.toString().replaceAll('e', '')
		// }
		const num = isNaN(+value) ? null : +value
		outputs?.result.set(num)
	}
</script>

<InputValue {id} input={configuration.label} bind:value={labelValue} />

<AlignWrapper {verticalAlignment}>
	<input
		bind:value
		type="number"
		inputmode="numeric"
		pattern="\d*"
		placeholder="Type..."
		class="h-full"
	/>
</AlignWrapper>
