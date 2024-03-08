<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import Loader from '../helpers/Loader.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'imagecomponent'> | undefined = undefined
	export let render: boolean

	const resolvedConfig = initConfig(
		components['imagecomponent'].initialData.configuration,
		configuration
	)

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	const fit: Record<string, string> = {
		cover: 'object-cover',
		contain: 'object-contain',
		fill: 'object-fill'
	}

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let css = initCss($app.css?.imagecomponent, customCss)
</script>

<InitializeComponent {id} />

{#each Object.keys(components['imagecomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.imagecomponent}
	/>
{/each}

{#if render}
	<Loader loading={resolvedConfig.source == undefined}>
		<img
			on:pointerdown|preventDefault
			src={resolvedConfig.sourceKind == 'png encoded as base64'
				? 'data:image/png;base64,' + resolvedConfig.source
				: resolvedConfig.sourceKind == 'jpeg encoded as base64'
				? 'data:image/jpeg;base64,' + resolvedConfig.source
				: resolvedConfig.sourceKind == 'svg encoded as base64'
				? 'data:image/svg+xml;base64,' + resolvedConfig.source
				: resolvedConfig.source}
			alt={resolvedConfig.altText}
			style={css?.image?.style ?? ''}
			class={twMerge(
				`w-full h-full ${fit[resolvedConfig.imageFit || 'cover']}`,
				css?.image?.class,
				'wm-image'
			)}
		/>
	</Loader>
{/if}
