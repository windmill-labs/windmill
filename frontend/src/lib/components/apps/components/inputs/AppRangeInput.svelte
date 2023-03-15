<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { Output } from '../../rx'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import { concatCustomCss } from '../../utils'
	import { twMerge } from 'tailwind-merge'
	import { initOutput } from '../../editor/appUtils'

	export let id: string
	export let configuration: Record<string, AppInput>
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'handles' | 'bar' | 'limits' | 'values'> | undefined =
		undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let min = 0
	let max = 42
	let step = 1
	let slider: HTMLElement

	let values: [number, number] = [0, 0]

	let outputs = initOutput($worldStore, id, {
		result: null as [number, number] | null
	})

	console.log('reset')

	$: outputs?.result.set(values)

	$: css = concatCustomCss($app.css?.rangecomponent, customCss)

	let lastStyle: string | undefined = undefined
	$: if (css && slider && lastStyle !== css?.handles?.style) {
		const handles = slider.querySelectorAll<HTMLSpanElement>('.rangeNub')
		if (handles) {
			lastStyle = css?.handles?.style
			handles.forEach((handle) => (handle.style.cssText = css?.handles?.style ?? ''))
		}
	}
</script>

<InputValue {id} input={configuration.step} bind:value={step} />
<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />
<InputValue {id} input={configuration.defaultLow} bind:value={values[0]} />
<InputValue {id} input={configuration.defaultHigh} bind:value={values[1]} />

<AlignWrapper {render} {verticalAlignment}>
	<div class="flex flex-col w-full">
		<div class="flex items-center w-full gap-1 px-1">
			<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
				{+min}
			</span>
			<div
				class="grow"
				style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; {css?.bar?.style ??
					''}"
				on:pointerdown|stopPropagation
			>
				<RangeSlider
					bind:slider
					bind:values
					{step}
					min={!min ? 0 : +min}
					max={!max ? 1 : +max}
					range
				/>
				<!-- <RangeSlider {step} range min={min ?? 0} max={max ?? 1} bind:values /> -->
			</div>
			<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
				{+max}
			</span>
		</div>
		<div class="flex justify-between px-1">
			<span
				class={twMerge(
					'text-center text-sm font-medium bg-blue-100 text-blue-800 rounded px-2.5 py-0.5',
					css?.values?.class ?? ''
				)}
				style={css?.values?.style ?? ''}
			>
				{values[0]}
			</span>
			<span
				class={twMerge(
					'text-center text-sm font-medium bg-blue-100 text-blue-800 rounded px-2.5 py-0.5',
					css?.values?.class ?? ''
				)}
				style={css?.values?.style ?? ''}
			>
				{values[1]}
			</span>
		</div>
	</div>
</AlignWrapper>
