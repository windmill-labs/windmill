<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import {
		type AppViewerContext,
		type ComponentCustomCSS,
		type RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import { components } from '../../editor/component'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import RecomputeAllWrapper from '../../editor/RecomputeAllWrapper.svelte'

	export let id: string
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'jobiddisplaycomponent'> | undefined = undefined
	export let configuration: RichConfigurations
	export let render: boolean

	const { app, worldStore, policy } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['recomputeallcomponent'].initialData.configuration,
		configuration
	)

	initOutput($worldStore, id, {
		loading: undefined
	})

	initializing = false

	let css = initCss($app.css?.recomputeallcomponent, customCss)
</script>

{#each Object.keys(components['recomputeallcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.recomputeallcomponent}
	/>
{/each}

<InitializeComponent {id} />

{#if render && policy}
	<RecomputeAllWrapper
		containerClass={css?.container?.class}
		containerStyle={css?.container?.style}
	/>
{/if}
