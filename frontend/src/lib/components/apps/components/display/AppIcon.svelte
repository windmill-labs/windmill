<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import { AlignWrapper } from '../helpers'
	import { loadIcon } from '../icon'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

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

	$: css = concatCustomCss($app.css?.iconcomponent, customCss)
</script>

{#each Object.keys(components['iconcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}
<InitializeComponent {id} />

<AlignWrapper
	{render}
	{horizontalAlignment}
	{verticalAlignment}
	class={css?.container?.class ?? ''}
	style={css?.container?.style ?? ''}
>
	{#if resolvedConfig.icon && iconComponent}
		<svelte:component
			this={iconComponent}
			size={resolvedConfig.size || 24}
			color={resolvedConfig.color || 'currentColor'}
			strokeWidth={resolvedConfig.strokeWidth || 2}
			class={css?.icon?.class ?? ''}
			style={css?.icon?.style ?? ''}
		/>
	{/if}
</AlignWrapper>
