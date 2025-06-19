<script lang="ts">
	import { getContext, onDestroy } from 'svelte'
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
	export let onChange: string[] | undefined = undefined
	const { app, worldStore, selectedComponent, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = initConfig(
		components['numberinputcomponent'].initialData.configuration,
		configuration
	)

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let outputs = initOutput($worldStore, id, {
		result: undefined as number | undefined
	})

	let initValue = outputs?.result.peak()
	let value: number | undefined =
		!iterContext && initValue != undefined ? initValue : resolvedConfig.defaultValue

	$componentControl[id] = {
		setValue(nvalue: number | undefined) {
			value = nvalue
			outputs?.result.set(value)
		}
	}
	let initialHandleDefault = true

	$: handleDefault(resolvedConfig.defaultValue)

	$: value, onChangeValue()

	function onChangeValue() {
		outputs?.result.set(value ?? undefined)
		if (iterContext && listInputs) {
			listInputs.set(id, value ?? undefined)
		}
		fireOnChange()
	}

	function fireOnChange() {
		if (onChange) {
			onChange.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function handleDefault(defaultValue: number | undefined) {
		if (initialHandleDefault) {
			initialHandleDefault = false
			if (value != undefined) {
				return
			}
		}
		value = defaultValue
	}

	let css = initCss(app.val.css?.numberinputcomponent, customCss)
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
		componentStyle={app.val.css?.numberinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	<input
		on:pointerdown|stopPropagation={() => ($selectedComponent = [id])}
		on:focus={() => ($selectedComponent = [id])}
		class={twMerge(
			'windmillapp w-full py-1.5 px-2 text-sm  app-editor-input',
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
