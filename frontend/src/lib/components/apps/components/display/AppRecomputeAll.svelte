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
	import AlignWrapper from '../helpers/AlignWrapper.svelte'

	export let id: string
	export let initializing: boolean | undefined = false
	export let customCss: ComponentCustomCSS<'jobiddisplaycomponent'> | undefined = undefined
	export let configuration: RichConfigurations
	export let render: boolean
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined

	const { app, worldStore, policy, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = initConfig(
		components['recomputeallcomponent'].initialData.configuration,
		configuration
	)

	initOutput($worldStore, id, {
		loading: undefined
	})

	initializing = false

	let css = initCss(app.val.css?.recomputeallcomponent, customCss)

	$: resolvedConfig.defaultRefreshInterval && handleRefreshInterval()

	function handleRefreshInterval() {
		if (resolvedConfig.defaultRefreshInterval !== undefined) {
			const newInterval =
				typeof resolvedConfig.defaultRefreshInterval === 'number'
					? resolvedConfig.defaultRefreshInterval * 1000
					: parseInt(resolvedConfig.defaultRefreshInterval) * 1000

			if (newInterval !== $recomputeAllContext.interval && newInterval) {
				$recomputeAllContext.setInter?.(newInterval)
			}
		}
	}
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
		componentStyle={app.val.css?.recomputeallcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {horizontalAlignment}>
	{#if render && policy}
		<RecomputeAllWrapper
			containerClass={css?.container?.class}
			containerStyle={css?.container?.style}
		/>
	{/if}
</AlignWrapper>
