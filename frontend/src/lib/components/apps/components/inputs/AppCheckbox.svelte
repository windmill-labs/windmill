<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import { getContext, onDestroy, untrack } from 'svelte'
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
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		recomputeIds?: string[] | undefined
		customCss?: ComponentCustomCSS<'checkboxcomponent'> | undefined
		render: boolean
		extraKey?: string | undefined
		preclickAction?: (() => Promise<void>) | undefined
		noInitialize?: boolean
		onToggle?: string[] | undefined
		controls?: { left: () => boolean; right: () => boolean | string } | undefined
	}

	let {
		id,
		configuration,
		horizontalAlignment = undefined,
		verticalAlignment = undefined,
		recomputeIds = undefined,
		customCss = undefined,
		render,
		extraKey = undefined,
		preclickAction = undefined,
		noInitialize = false,
		onToggle = undefined,
		controls = undefined
	}: Props = $props()

	const { app, worldStore, componentControl, runnableComponents } =
		getContext<AppViewerContext>('AppViewerContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const rowInputs: ListInputs | undefined = getContext<ListInputs>('RowInputs')

	let resolvedConfig = $state(
		initConfig(components['checkboxcomponent'].initialData.configuration, configuration)
	)

	let value: boolean = $state(resolvedConfig.defaultValue ?? false)

	$componentControl[id] = {
		setValue(nvalue: boolean) {
			value = nvalue
			if (recomputeIds) {
				recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
			}
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
			listInputs.set(id, value)
		}
		if (rowContext && rowInputs) {
			rowInputs.set(id, value)
		}
	}

	function handleDefault() {
		value = resolvedConfig.defaultValue ?? false
		handleInput()
		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
		rowInputs?.remove(id)
	})

	$effect(() => {
		value != undefined && untrack(() => handleInput())
	})

	$effect(() => {
		resolvedConfig.defaultValue != undefined && untrack(() => handleDefault())
	})

	let css = $state(initCss($app.css?.checkboxcomponent, customCss))
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

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.checkboxcomponent}
	/>
{/each}

{#if !noInitialize}
	<InitializeComponent {id} />
{/if}

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
			if (recomputeIds) {
				recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
			}
			if (onToggle) {
				onToggle.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb()))
			}
		}}
		disabled={resolvedConfig.disabled}
	/>
</AlignWrapper>
