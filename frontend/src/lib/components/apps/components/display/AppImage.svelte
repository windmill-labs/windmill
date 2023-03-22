<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components, selectOptions } from '../../editor/component'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import Loader from '../helpers/Loader.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'imagecomponent'> | undefined = undefined
	export let render: boolean

	let resolvedConfig = initConfig(
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

	$: css = concatCustomCss($app.css?.imagecomponent, customCss)
</script>

{#each Object.keys(components['imagecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if render}
	<Loader loading={resolvedConfig.source == undefined}>
		<img
			on:pointerdown|preventDefault
			src={resolvedConfig.source}
			alt={resolvedConfig.altText}
			style={css?.image?.style ?? ''}
			class={twMerge(
				`w-full h-full ${fit[resolvedConfig.imageFit || 'cover']}`,
				css?.image?.class ?? ''
			)}
		/>
	</Loader>
{/if}
