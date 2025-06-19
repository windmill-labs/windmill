<script lang="ts">
	import RangeSlider from 'svelte-range-slider-pips'
	import { getContext, onDestroy } from 'svelte'
	import type {
		ListInputs,
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		RichConfigurations
	} from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { twMerge } from 'tailwind-merge'
	import { initCss } from '../../utils'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'slidercomponent'> | undefined = undefined
	export let render: boolean

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['slidercomponent'].initialData.configuration,
		configuration
	)

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let values: [number] = [resolvedConfig.defaultValue ?? 0]

	$: resolvedConfig.defaultValue != undefined && handleDefault()

	function handleDefault() {
		values = [resolvedConfig?.defaultValue ?? 0]
	}

	let slider: HTMLElement

	const outputs = initOutput($worldStore, id, {
		result: (0 as number) || null
	})

	$componentControl[id] = {
		setValue(nvalue: number) {
			values = [nvalue]
		}
	}

	$: values && handleValues()

	function handleValues() {
		// Disallow 'e' character in numbers
		// if(value && value.toString().includes('e')) {
		// 	value = +value.toString().replaceAll('e', '')
		// }
		const num = isNaN(+values[0]) ? null : +values[0]
		outputs?.result.set(num)

		if (iterContext && listInputs) {
			listInputs.set(id, num)
		}
	}

	let css = initCss(app.val.css?.slidercomponent, customCss)

	let lastStyle: string | undefined = undefined
	$: if (css && slider && lastStyle !== css?.handle?.style) {
		const handle = slider.querySelector<HTMLSpanElement>('.rangeNub')
		if (handle) {
			lastStyle = css?.handle?.style
			handle.style.cssText = css?.handle?.style ?? ''
		}
	}

	const spanClass =
		'text-center text-sm font-medium bg-blue-100 text-blue-800 rounded px-2.5 py-0.5'
	function computeWidth() {
		let maxValue = resolvedConfig.max ?? 0 + (resolvedConfig.step ?? 0)

		if (typeof document !== 'undefined') {
			const span = document.createElement('span')
			span.style.visibility = 'hidden'
			span.style.position = 'absolute'
			span.style.whiteSpace = 'nowrap'
			span.className = spanClass
			span.textContent = maxValue.toString()
			document.body.appendChild(span)
			document.body.removeChild(span)
		}
	}

	$: if (resolvedConfig.max != undefined && resolvedConfig.step && render) {
		computeWidth()
	}
</script>

{#each Object.keys(components['slidercomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={app.val.css?.slidercomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} hFull {verticalAlignment}>
	<div class="flex {resolvedConfig.vertical ? 'flex-col' : ''} items-center w-full h-full gap-1">
		<div
			class={twMerge(
				'grow',
				css?.bar?.class,
				'font-mono wm-slider-bar',
				resolvedConfig?.vertical ? 'h-full' : 'w-full'
			)}
			style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; {css?.bar?.style ??
				''}"
			on:pointerdown|stopPropagation={() => ($selectedComponent = [id])}
		>
			<RangeSlider
				id="slider-input"
				springValues={{ stiffness: 1, damping: 1 }}
				vertical={resolvedConfig.vertical}
				bind:slider
				bind:values
				step={resolvedConfig.step ?? 1}
				pipstep={(resolvedConfig.axisStep ?? 1) / (resolvedConfig.step ?? 1)}
				min={+(resolvedConfig?.min ?? 0)}
				max={+(resolvedConfig?.max ?? 0)}
				disabled={resolvedConfig.disabled}
				pips
				float
				first="label"
				last="label"
			/>
		</div>
	</div>
</AlignWrapper>

<style>
	:global(#slider-input.rangeSlider) {
		font-size: 12px;
		text-transform: uppercase;
	}

	:global(#slider-input.rangeSlider .rangeHandle) {
		width: 2em;
		height: 2em;
	}

	:global(#slider-input.rangeSlider .rangeFloat) {
		opacity: 1;
		background: transparent;
		top: 50%;
		transform: translate(-50%, -50%);
	}
</style>
