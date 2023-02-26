<script lang="ts">
	import { Badge } from '$lib/components/common'
	import RangeSlider from 'svelte-range-slider-pips'
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

	const { worldStore, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')
	let min = 0
	let max = 42
	let step = 1

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<number | null>
	}

	let values: [number] = [outputs?.result.peak() ?? 0]

	$: values && handleValues()

	function handleValues() {
		// Disallow 'e' character in numbers
		// if(value && value.toString().includes('e')) {
		// 	value = +value.toString().replaceAll('e', '')
		// }
		const num = isNaN(+values[0]) ? null : +values[0]
		outputs?.result.set(num)
	}
</script>

<InputValue {id} input={configuration.step} bind:value={step} />
<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />
<InputValue {id} input={configuration.defaultValue} bind:value={values[0]} />

<AlignWrapper {verticalAlignment}>
	<div class="flex items-center w-full gap-1 px-1">
		<span>{min}</span>
		<div class="grow" on:pointerdown|stopPropagation={() => ($selectedComponent = id)}>
			<RangeSlider {step} min={min ?? 0} max={max ?? 1} bind:values />
		</div>
		<span>{max}</span>
		<span class="mx-2"><Badge large color="blue">{values[0]}</Badge></span>
	</div>
</AlignWrapper>
