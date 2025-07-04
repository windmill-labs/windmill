<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext, onDestroy, untrack } from 'svelte'
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
		customCss?: ComponentCustomCSS<'numberinputcomponent'> | undefined
		render: boolean
		onChange?: string[] | undefined
	}

	let {
		id,
		configuration,
		verticalAlignment = undefined,
		customCss = undefined,
		render,
		onChange = undefined
	}: Props = $props()
	const { app, worldStore, selectedComponent, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	let resolvedConfig = $state(
		initConfig(components['numberinputcomponent'].initialData.configuration, configuration)
	)

	onDestroy(() => {
		listInputs?.remove(id)
	})

	let outputs = initOutput($worldStore, id, {
		result: undefined as number | undefined
	})

	let initValue = outputs?.result.peak()
	let value: number | undefined = $state(
		!iterContext && initValue != undefined ? initValue : resolvedConfig.defaultValue
	)

	$componentControl[id] = {
		setValue(nvalue: number | undefined) {
			value = nvalue
			outputs?.result.set(value)
		}
	}
	let initialHandleDefault = true

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

	let css = $state(initCss($app.css?.numberinputcomponent, customCss))
	$effect.pre(() => {
		resolvedConfig.defaultValue
		untrack(() => handleDefault(resolvedConfig.defaultValue))
	})
	$effect.pre(() => {
		value
		untrack(() => onChangeValue())
	})
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
		onpointerdown={stopPropagation(() => ($selectedComponent = [id]))}
		onfocus={() => ($selectedComponent = [id])}
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
