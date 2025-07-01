<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { twMerge } from 'tailwind-merge'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { Alert } from '$lib/components/common'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { appendClass } from '../../editor/componentsPanel/cssUtils'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'alertcomponent'> | undefined
		render: boolean
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
	}

	let {
		id,
		configuration,
		customCss = undefined,
		render,
		verticalAlignment = undefined
	}: Props = $props()

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let resolvedConfig = $state(
		initConfig(components['alertcomponent'].initialData.configuration, configuration)
	)

	initOutput($worldStore, id, {})

	let css = $state(initCss($app.css?.alertcomponent, customCss))
</script>

{#each Object.keys(components['alertcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.alertcomponent}
	/>
{/each}

<InitializeComponent {id} />

{#if render}
	<AlignWrapper {verticalAlignment}>
		<div
			class={twMerge('w-full', css?.container?.class, 'wm-alert-card-container')}
			style={css?.container?.style}
		>
			<Alert
				title={resolvedConfig.title ?? ''}
				type={resolvedConfig.type}
				notRounded={resolvedConfig.notRounded}
				tooltip={resolvedConfig.tooltip}
				size={resolvedConfig.size}
				collapsible={resolvedConfig.collapsible}
				bgClass={appendClass(css?.background?.class, 'wm-alert-card-background')}
				bgStyle={css?.background?.style}
				iconClass={appendClass(css?.icon?.class, 'wm-alert-card-icon')}
				iconStyle={css?.icon?.style}
				titleClass={appendClass(css?.title?.class, 'wm-alert-card-title')}
				titleStyle={css?.title?.style}
				descriptionClass={appendClass(css?.description?.class, 'wm-alert-card-description')}
				descriptionStyle={css?.description?.style}
				isCollapsed={resolvedConfig.initiallyCollapsed}
			>
				{resolvedConfig.description}
			</Alert>
		</div>
	</AlignWrapper>
{/if}
