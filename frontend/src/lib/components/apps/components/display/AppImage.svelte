<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components, configurationKeys } from '../../editor/component'
	import type { staticValues } from '../../editor/componentsPanel/componentStaticValues'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import Loader from '../helpers/Loader.svelte'

	type FitOption = (typeof staticValues)['objectFitOptions'][number]

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'imagecomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	const fit: Record<FitOption, string> = {
		cover: 'object-cover',
		contain: 'object-contain',
		fill: 'object-fill'
	}

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let resolvedConfig = initConfig(components['imagecomponent'].initialData.configuration)

	$: css = concatCustomCss($app.css?.imagecomponent, customCss)
</script>

{#each configurationKeys['imagecomponent'] as key (key)}
	<InputValue {key} {id} input={configuration[key]} bind:value={resolvedConfig[key]} />
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
