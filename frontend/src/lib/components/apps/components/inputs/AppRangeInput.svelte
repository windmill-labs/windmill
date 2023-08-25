<script lang="ts">
	import { getContext } from 'svelte'
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
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'rangecomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = initConfig(
		components['rangecomponent'].initialData.configuration,
		configuration
	)

	let values: [number, number] = [0, 1]

	$: (resolvedConfig.defaultLow != undefined || resolvedConfig.defaultHigh != undefined) &&
		handleDefault()

	function handleDefault() {
		values = [resolvedConfig?.defaultLow ?? 0, resolvedConfig?.defaultHigh ?? 1]
	}

	let disabled = false

	let slider: HTMLElement

	let outputs = initOutput($worldStore, id, {
		result: null as [number, number] | null
	})

	$componentControl[id] = {
		setValue(nvalue: [number, number]) {
			values = nvalue
		}
	}

	$: {
		outputs?.result.set(values)
		if (iterContext && listInputs) {
			listInputs(id, values)
		}
	}

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

{#each Object.keys(components['rangecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<div class="flex flex-col w-full">
		<div class="flex items-center w-full gap-1 px-1">
			<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
				{+(resolvedConfig.min ?? 0)}
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
					step={resolvedConfig.step}
					min={resolvedConfig.min == undefined ? 0 : +resolvedConfig.min}
					max={resolvedConfig.max == undefined ? 1 : +resolvedConfig.max}
					range
					{disabled}
				/>
				<!-- <RangeSlider {step} range min={min ?? 0} max={max ?? 1} bind:values /> -->
			</div>
			<span class={css?.limits?.class ?? ''} style={css?.limits?.style ?? ''}>
				{+(resolvedConfig.max ?? 1)}
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
