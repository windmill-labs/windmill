<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { loadIcon } from '../icon'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'

	export let id: string
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = 'left'
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'iconcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['iconcomponent'].initialData.configuration,
		configuration
	)
	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let iconComponent: any

	$: handleIcon(resolvedConfig, iconComponent)

	async function handleIcon(conf, comp) {
		if (conf.icon && comp) {
			await loadIcon(conf.icon, iconComponent, conf.size, conf.strokeWidth, conf.color)
		}
	}

	let css = initCss($app.css?.iconcomponent, customCss)
</script>

{#each Object.keys(components['iconcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.iconcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper
	{render}
	{horizontalAlignment}
	{verticalAlignment}
	class={twMerge(css?.container?.class, 'wm-icon-container')}
	style={css?.container?.style ?? ''}
>
	{#if resolvedConfig.icon}
		<div
			bind:this={iconComponent}
			class={twMerge(css?.icon?.class, 'wm-icon')}
			style={css?.icon?.style ?? ''}
		></div>
	{/if}
</AlignWrapper>
