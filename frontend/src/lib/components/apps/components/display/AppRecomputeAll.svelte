<script lang="ts">
	import { getContext, untrack } from 'svelte'
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

	interface Props {
		id: string
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'jobiddisplaycomponent'> | undefined
		configuration: RichConfigurations
		render: boolean
		horizontalAlignment?: 'left' | 'center' | 'right' | undefined
	}

	let {
		id,
		initializing = $bindable(undefined),
		customCss = undefined,
		configuration,
		render,
		horizontalAlignment = undefined
	}: Props = $props()

	const { app, worldStore, policy, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['recomputeallcomponent'].initialData.configuration, configuration)
	)

	initOutput($worldStore, id, {
		loading: undefined
	})

	initializing = false

	let css = $state(initCss($app.css?.recomputeallcomponent, customCss))

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
	$effect(() => {
		resolvedConfig.defaultRefreshInterval && untrack(() => handleRefreshInterval())
	})
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

<AlignWrapper {horizontalAlignment}>
	{#if render && policy}
		<RecomputeAllWrapper
			containerClass={css?.container?.class}
			containerStyle={css?.container?.style}
		/>
	{/if}
</AlignWrapper>
