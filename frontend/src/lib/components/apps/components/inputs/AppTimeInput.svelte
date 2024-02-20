<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let inputType: 'date'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'timeinputcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['timeinputcomponent'].initialData.configuration,
		configuration
	)

	let value: string | undefined = undefined

	$componentControl[id] = {
		setValue(nvalue: string) {
			value = nvalue
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	$: handleDefault(resolvedConfig.defaultValue)

	$: {
		if (value) {
			outputs?.result.set(value)
		} else {
			outputs?.result.set(undefined)
		}
	}

	function handleDefault(defaultValue: string | undefined) {
		value = defaultValue
	}
	let css = initCss($app.css?.timeinputcomponent, customCss)
</script>

{#each Object.keys(components['timeinputcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.timeinputcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} {verticalAlignment}>
	{#if inputType === 'date'}
		<input
			on:focus={() => ($selectedComponent = [id])}
			on:pointerdown|stopPropagation
			type="time"
			bind:value
			min={resolvedConfig.minTime}
			max={resolvedConfig.maxTime}
			placeholder="Type..."
			class={twMerge('w-full ', css?.input?.class)}
			style={css?.input?.style ?? ''}
		/>
	{/if}
</AlignWrapper>
