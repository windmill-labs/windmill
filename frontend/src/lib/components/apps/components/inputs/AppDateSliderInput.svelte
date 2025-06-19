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
	import { parseISO, format as formatDateFns } from 'date-fns'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'dateslidercomponent'> | undefined = undefined
	export let render: boolean

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['dateslidercomponent'].initialData.configuration,
		configuration
	)

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let values: [number] = [
		resolvedConfig.defaultValue
			? new Date(resolvedConfig.defaultValue).getTime()
			: new Date().getTime()
	]

	$: resolvedConfig.defaultValue != undefined && handleDefault()

	function handleDefault() {
		values = [
			resolvedConfig.defaultValue
				? new Date(resolvedConfig.defaultValue).getTime()
				: new Date().getTime()
		]
	}

	let slider: HTMLElement

	const outputs = initOutput($worldStore, id, {
		result: null as string | null
	})

	$componentControl[id] = {
		setValue(nvalue: number) {
			values = [nvalue]
		}
	}

	$: values && handleValues()

	function formatDate(dateString: string, formatString: string = 'dd.MM.yyyy') {
		if (formatString === '') {
			formatString = 'dd.MM.yyyy'
		}

		try {
			const isoDate = parseISO(dateString)
			return formatDateFns(isoDate, formatString)
		} catch (error) {
			return 'Error formatting date:' + error.message
		}
	}

	function handleValues() {
		const dateString = values[0] ? new Date(values[0]).toISOString() : new Date().toISOString()

		outputs?.result.set(formatDate(dateString, resolvedConfig.outputFormat))

		if (iterContext && listInputs) {
			listInputs.set(id, formatDate(dateString, resolvedConfig.outputFormat))
		}
	}

	let css = initCss(app.val.css?.dateslidercomponent, customCss)

	let lastStyle: string | undefined = undefined
	$: if (css && slider && lastStyle !== css?.handle?.style) {
		const handle = slider.querySelector<HTMLSpanElement>('.rangeNub')
		if (handle) {
			lastStyle = css?.handle?.style
			handle.style.cssText = css?.handle?.style ?? ''
		}
	}

	let width = 0

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
			width = span?.offsetWidth
			document.body.removeChild(span)
		}
	}

	$: if (resolvedConfig.max != undefined && resolvedConfig.step && render) {
		computeWidth()
	}
</script>

{#each Object.keys(components['dateslidercomponent'].initialData.configuration) as key (key)}
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
		componentStyle={app.val.css?.dateslidercomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} hFull {verticalAlignment}>
	<div
		class="flex {resolvedConfig.vertical ? 'flex-col' : ''} items-center w-full h-full gap-1 px-1"
	>
		<span
			class={twMerge(css?.limits?.class, 'font-mono whitespace-nowrap  wm-slider-limits')}
			style={css?.limits?.style ?? ''}
		>
			{resolvedConfig?.min
				? new Date(resolvedConfig?.min).toDateString()
				: new Date().toDateString()}
		</span>
		<div
			class={twMerge(
				'grow',
				css?.bar?.class,
				'font-mono wm-slider-bar',
				resolvedConfig?.vertical ? 'h-full' : 'w-full'
			)}
			style={css?.bar?.style}
			on:pointerdown|stopPropagation={() => ($selectedComponent = [id])}
		>
			<RangeSlider
				springValues={{ stiffness: 1, damping: 1 }}
				vertical={resolvedConfig.vertical}
				bind:slider
				bind:values
				step={(resolvedConfig.step ?? 1) * 1000 * 60 * 60 * 24}
				min={resolvedConfig?.min ? new Date(resolvedConfig?.min).getTime() : new Date().getTime()}
				max={resolvedConfig?.max ? new Date(resolvedConfig?.max).getTime() : new Date().getTime()}
				disabled={resolvedConfig.disabled}
			/>
		</div>
		<span
			class={twMerge(css?.limits?.class, 'font-mono whitespace-nowrap wm-slider-limits')}
			style={css?.limits?.style ?? ''}
		>
			{resolvedConfig?.max
				? new Date(resolvedConfig?.max).toDateString()
				: new Date().toDateString()}
		</span>
		<span class="mx-2">
			<span
				class={twMerge(
					spanClass,
					css?.value?.class ?? '',
					'font-mono  whitespace-nowrap  wm-slider-value'
				)}
				style={`${css?.value?.style ?? ''} ${width ? `width: ${width}px;` : ''}`}
			>
				{values[0] ? new Date(values[0]).toDateString() : new Date().toDateString()}
			</span>
		</span>
	</div>
</AlignWrapper>

<style global>
	.rangeSlider.vertical {
		height: 80%;
		min-height: 10px !important;
	}
</style>
