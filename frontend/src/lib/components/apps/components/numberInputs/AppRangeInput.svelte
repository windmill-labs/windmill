<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import { Badge } from '$lib/components/common'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	let min = 0
	let max = 42
	let step = 1

	let values: [number, number] = [0, 0]

	$: outputs = $worldStore?.outputsById[id] as {
		result: Output<[number, number] | null>
	}

	$: outputs?.result.set(values)
</script>

<InputValue {id} input={configuration.step} bind:value={step} />
<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />
<InputValue {id} input={configuration.defaultLow} bind:value={values[0]} />
<InputValue {id} input={configuration.defaultHigh} bind:value={values[1]} />

<AlignWrapper {verticalAlignment}>
	<div class="flex flex-col w-full">
		<div class="flex items-center w-full gap-1 px-1">
			<span>{min}</span>
			<div class="grow" on:pointerdown|stopPropagation>
				<RangeSlider {step} range min={min ?? 0} max={max ?? 1} bind:values />
				<!-- <DoubleRange bind:start bind:end /> -->
			</div>
			<span>{max}</span>
		</div>
		<div class="flex justify-between px-1">
			<div>
				<Badge color="blue">{values[0]}</Badge>
			</div>
			<div>
				<Badge color="blue">{values[1]}</Badge>
			</div>
		</div>
	</div>
</AlignWrapper>
