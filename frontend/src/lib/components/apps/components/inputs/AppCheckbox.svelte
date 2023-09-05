<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext } from 'svelte'
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
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let recomputeIds: string[] | undefined = undefined
	export let customCss: ComponentCustomCSS<'checkboxcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined
	export let preclickAction: (() => Promise<void>) | undefined = undefined

	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined

	const { app, worldStore, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const rowInputs: ListInputs | undefined = getContext<ListInputs>('RowInputs')

	let resolvedConfig = initConfig(
		components['checkboxcomponent'].initialData.configuration,
		configuration
	)

	let value: boolean = resolvedConfig.defaultValue ?? false

	$componentControl[id] = {
		setValue(nvalue: boolean) {
			value = nvalue
		}
	}

	if (controls) {
		$componentControl[id] = { ...$componentControl[id], ...controls }
	}

	// As the checkbox is a special case and has no input
	// we need to manually set the output

	let outputs = initOutput($worldStore, id, {
		result: false
	})

	function handleInput() {
		outputs.result.set(value)
		if (iterContext && listInputs) {
			listInputs(id, value)
		}
		if (rowContext && rowInputs) {
			rowInputs(id, value)
		}
		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	function handleDefault() {
		value = resolvedConfig.defaultValue ?? false
		handleInput()
	}

	$: value != undefined && handleInput()

	$: resolvedConfig.defaultValue != undefined && handleDefault()

	$: css = concatCustomCss($app.css?.checkboxcomponent, customCss)
</script>

{#each Object.keys(components['checkboxcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		{extraKey}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />
<AlignWrapper
	{render}
	{horizontalAlignment}
	{verticalAlignment}
	class={twMerge(css?.container?.class, 'wm-toggle-container')}
	style={css?.container?.style}
>
	<Toggle
		size="sm"
		bind:checked={value}
		options={{ right: resolvedConfig.label }}
		textClass={twMerge(css?.text?.class, 'wm-toggle-text')}
		textStyle={css?.text?.style ?? ''}
		on:change={(e) => {
			preclickAction?.()
			value = e.detail
		}}
		disabled={resolvedConfig.disabled}
	/>
</AlignWrapper>
