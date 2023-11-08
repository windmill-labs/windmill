<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { AlignWrapper } from '../helpers'
	import { loadIcon } from '../icon'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

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

	$: handleIcon(resolvedConfig.icon)

	async function handleIcon(i?: string) {
		iconComponent = i ? await loadIcon(i) : undefined
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
	{#if resolvedConfig.icon && iconComponent}
		<svelte:component
			this={iconComponent}
			size={resolvedConfig.size || 24}
			color={resolvedConfig.color || 'currentColor'}
			strokeWidth={resolvedConfig.strokeWidth || 2}
			class={twMerge(css?.icon?.class, 'wm-icon')}
			style={css?.icon?.style ?? ''}
		/>
	{/if}
</AlignWrapper>
