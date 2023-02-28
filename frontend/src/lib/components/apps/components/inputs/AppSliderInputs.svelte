<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import { twMerge } from 'tailwind-merge'
	import { concatCustomCss } from '../../utils'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export const staticOutputs: string[] = ['result']
	export let customCss: ComponentCustomCSS<'handle' | 'limits' | 'value'> | undefined = undefined

	const { app, worldStore, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')
	let min = 0
	let max = 42
	let step = 1
	let slider: HTMLElement

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

	$: css = concatCustomCss($app.css?.slidercomponent, customCss)

	let lastStyle: string | undefined = undefined
	$: if (css && slider && lastStyle !== css?.handle?.style) {
		const handle = slider.querySelector<HTMLSpanElement>('.rangeNub')
		if (handle) {
			lastStyle = css?.handle?.style
			handle.style.cssText = css?.handle?.style ?? ''
		}
	}
</script>

<InputValue {id} input={configuration.step} bind:value={step} />
<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />
<InputValue {id} input={configuration.defaultValue} bind:value={values[0]} />

<AlignWrapper {verticalAlignment}>
	<div class="flex items-center w-full gap-1 px-1">
		<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
			{min}
		</span>
		<div class="grow" on:pointerdown|stopPropagation={() => ($selectedComponent = id)}>
			<RangeSlider bind:slider bind:values {step} min={min ?? 0} max={max ?? 1} />
		</div>
		<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
			{max}
		</span>
		<span class="mx-2">
			<span
				class={twMerge(
					'text-center text-sm font-medium bg-blue-100 text-blue-800 rounded px-2.5 py-0.5',
					css?.value?.class ?? ''
				)}
				style={css?.value?.style ?? ''}
			>
				{values[0]}
			</span>
		</span>
	</div>
</AlignWrapper>
