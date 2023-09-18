<script lang="ts">
	import { getContext } from 'svelte'
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

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'numberinputcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = initConfig(
		components['numberinputcomponent'].initialData.configuration,
		configuration
	)

	let value: number | undefined = resolvedConfig.defaultValue

	$componentControl[id] = {
		setValue(nvalue: number | undefined) {
			value = nvalue
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined as number | undefined
	})

	$: handleDefault(resolvedConfig.defaultValue)

	$: {
		outputs?.result.set(value)
		if (iterContext && listInputs) {
			listInputs(id, value)
		}
	}

	function handleDefault(defaultValue: number | undefined) {
		value = defaultValue
	}

	let css = initCss($app.css?.numberinputcomponent, customCss)
</script>

{#each Object.keys(components['numberinputcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.numberinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<input
		on:pointerdown|stopPropagation={() => ($selectedComponent = [id])}
		on:focus={() => ($selectedComponent = [id])}
		class={twMerge(
			'windmillapp w-full py-1.5 text-sm focus:ring-indigo-100 px-2',
			css?.input?.class ?? '',
			'wm-number-input'
		)}
		style={css?.input?.style ?? ''}
		bind:value
		min={resolvedConfig.min}
		max={resolvedConfig.max}
		step={resolvedConfig.step}
		type="number"
		inputmode="numeric"
		pattern="\d*"
		placeholder={resolvedConfig.placeholder}
	/>
</AlignWrapper>
