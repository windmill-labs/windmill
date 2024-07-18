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
	import GridEditorTopbar from '../../editor/GridEditorTopbar.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'jobiddisplaycomponent'> | undefined = undefined
	export let configuration: RichConfigurations
	export let render: boolean

	const { app, worldStore, policy } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['topbarcomponent'].initialData.configuration,
		configuration
	)

	initOutput($worldStore, id, {
		result: undefined
	})

	initializing = false

	let css = initCss($app.css?.topbarcomponent, customCss)
</script>

{#each Object.keys(components['topbarcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.topbarcomponent}
	/>
{/each}

<InitializeComponent {id} />
{#if render && policy}
	<GridEditorTopbar
		{policy}
		displayTitle={resolvedConfig?.displayTitle}
		displayRecompute={resolvedConfig?.displayRecompute}
		displayAuthor={resolvedConfig?.displayAuthor}
		titleOverride={resolvedConfig?.titleOverride}
		containerClass={css?.container?.class}
		containerStyle={css?.container?.style}
	/>
{/if}
