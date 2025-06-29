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

	interface Props {
		id: string
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'iconcomponent'> | undefined
		render: boolean
	}

	let {
		id,
		horizontalAlignment = 'left',
		verticalAlignment = undefined,
		configuration,
		customCss = undefined,
		render
	}: Props = $props()

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['iconcomponent'].initialData.configuration, configuration)
	)
	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let iconComponent: any = $state()

	async function handleIcon(conf, comp) {
		if (conf.icon && comp) {
			await loadIcon(conf.icon, iconComponent, conf.size, conf.strokeWidth, conf.color)
		}
	}

	let css = $state(initCss($app.css?.iconcomponent, customCss))
	$effect(() => {
		handleIcon(resolvedConfig, iconComponent)
	})
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
