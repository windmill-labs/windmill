<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'numberinputcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	let defaultValue: number | undefined = undefined
	let placeholder: string | undefined = undefined
	let value: number | undefined = undefined

	let min: number | undefined = undefined
	let max: number | undefined = undefined
	let step = 1

	let outputs = initOutput($worldStore, id, {
		result: undefined as number | undefined
	})

	$: handleDefault(defaultValue)

	$: outputs?.result.set(value)

	function handleDefault(defaultValue: number | undefined) {
		value = defaultValue
	}

	$: css = concatCustomCss($app.css?.numberinputcomponent, customCss)
</script>

<InputValue {id} input={configuration.step} bind:value={step} />
<InputValue {id} input={configuration.min} bind:value={min} />
<InputValue {id} input={configuration.max} bind:value={max} />
<InputValue {id} input={configuration.placeholder} bind:value={placeholder} />
<InputValue {id} input={configuration.defaultValue} bind:value={defaultValue} />

<AlignWrapper {render} {verticalAlignment}>
	<input
		on:pointerdown|stopPropagation
		on:focus={() => ($selectedComponent = id)}
		class={twMerge(
			'windmillapp w-full py-1.5 text-sm focus:ring-indigo-100 px-2 mx-0.5',
			css?.input?.class ?? ''
		)}
		style={css?.input?.style ?? ''}
		bind:value
		{min}
		{max}
		{step}
		type="number"
		inputmode="numeric"
		pattern="\d*"
		{placeholder}
	/>
</AlignWrapper>
