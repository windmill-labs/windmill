<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { getContext, onDestroy, untrack } from 'svelte'
	import RangeSlider from 'svelte-range-slider-pips'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'rangecomponent'> | undefined
		render: boolean
	}

	let {
		id,
		configuration,
		verticalAlignment = undefined,
		customCss = undefined,
		render
	}: Props = $props()

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = $state(
		initConfig(components['rangecomponent'].initialData.configuration, configuration)
	)

	let values: [number, number] = $state([0, 1])

	function handleDefault() {
		values = [resolvedConfig?.defaultLow ?? 0, resolvedConfig?.defaultHigh ?? 1]
	}

	let slider: HTMLElement | undefined = $state()

	let outputs = initOutput($worldStore, id, {
		result: null as [number, number] | null
	})

	$componentControl[id] = {
		setValue(nvalue: [number, number]) {
			values = nvalue
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let css = $state(initCss($app.css?.rangecomponent, customCss))

	let lastStyle: string | undefined = $state(undefined)

	const format = (v, i, p) => {
		return `${v}`
	}
	$effect.pre(() => {
		resolvedConfig.defaultLow != undefined || resolvedConfig.defaultHigh != undefined
		untrack(() => handleDefault())
	})
	$effect.pre(() => {
		;[values[0], values[1]]
		outputs?.result.set(values)
		if (iterContext && listInputs) {
			listInputs.set(id, values)
		}
	})
	$effect.pre(() => {
		if (css && slider && lastStyle !== css?.handles?.style) {
			const handles = slider.querySelectorAll<HTMLSpanElement>('.rangeNub')
			if (handles) {
				lastStyle = css?.handles?.style
				handles.forEach((handle) => (handle.style.cssText = css?.handles?.style ?? ''))
			}
		}
	})
</script>

{#each Object.keys(components['rangecomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.rangecomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<div class="flex flex-col w-full">
		<div class="flex items-center w-full gap-1">
			<div
				class={twMerge('grow', 'wm-slider-bar')}
				style="--range-handle-focus: {'#7e9abd'}; --range-handle: {'#7e9abd'}; {css?.bar?.style ??
					''}"
				onpointerdown={stopPropagation(bubble('pointerdown'))}
			>
				<RangeSlider
					id="range-slider"
					springValues={{ stiffness: 1, damping: 1 }}
					bind:slider
					bind:values
					min={resolvedConfig.min == undefined ? 0 : +resolvedConfig.min}
					max={resolvedConfig.max == undefined ? 1 : +resolvedConfig.max}
					range
					disabled={resolvedConfig.disabled}
					pips
					float
					first="label"
					last="label"
					step={resolvedConfig.step ?? 1}
					pipstep={(resolvedConfig.axisStep ?? 1) / (resolvedConfig.step ?? 1)}
					formatter={format}
				/>
			</div>
		</div>
	</div>
</AlignWrapper>

<style>
	:global(#range-slider.rangeSlider) {
		font-size: 12px;
		text-transform: uppercase;
	}

	:global(.dark #range-slider.rangeSlider) {
		background-color: #3b4252;
	}

	:global(#range-slider.rangeSlider .rangeHandle) {
		width: 2em;
		height: 2em;
	}

	:global(#range-slider.rangeSlider .rangeFloat) {
		opacity: 1;
		background: transparent;
		top: 50%;
		transform: translate(-50%, -50%);
	}

	:global(.dark #range-slider.rangeSlider > .rangePips > .pip) {
		color: #eeeeee;
	}
</style>
