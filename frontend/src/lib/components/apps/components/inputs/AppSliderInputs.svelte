<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'
	import { getContext } from 'svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import { twMerge } from 'tailwind-merge'
	import { concatCustomCss } from '../../utils'
	import { initOutput } from '../../editor/appUtils'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'slidercomponent'> | undefined = undefined
	export let render: boolean
	export let initializing: boolean = true

	const { app, worldStore, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	let min = 0
	let max = 42
	let step = 1
	let slider: HTMLElement

	let outputs = initOutput($worldStore, id, {
		result: (0 as number) || null
	})

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
<InputValue
	on:done={() => (initializing = false)}
	{id}
	input={configuration.defaultValue}
	bind:value={values[0]}
/>

<AlignWrapper {render} {verticalAlignment}>
	<div class="flex items-center w-full gap-1 px-1">
		<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
			{+min}
		</span>
		<div
			class="grow"
			style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; {css?.bar?.style ??
				''}"
			on:pointerdown|stopPropagation={() => ($selectedComponent = id)}
		>
			<RangeSlider bind:slider bind:values {step} min={+min} max={+max} />
		</div>
		<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
			{+max}
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
