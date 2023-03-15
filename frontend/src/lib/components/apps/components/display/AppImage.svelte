<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { staticValues } from '../../editor/componentsPanel/componentStaticValues'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'

	type FitOption = (typeof staticValues)['objectFitOptions'][number]

	export let id: string
	export let configuration: Record<string, AppInput>
	export let customCss: ComponentCustomCSS<'image'> | undefined = undefined
	export let render: boolean

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const fit: Record<FitOption, string> = {
		cover: 'object-cover',
		contain: 'object-contain',
		fill: 'object-fill'
	}

	let source: string | undefined = undefined
	let imageFit: FitOption | undefined = undefined
	let altText: string | undefined = undefined

	$: css = concatCustomCss($app.css?.imagecomponent, customCss)
</script>

<InputValue {id} input={configuration.source} bind:value={source} />
<InputValue {id} input={configuration.imageFit} bind:value={imageFit} />
<InputValue {id} input={configuration.altText} bind:value={altText} />

{#if render}
	<img
		on:pointerdown|preventDefault
		src={source}
		alt={altText}
		style={css?.image?.style ?? ''}
		class={twMerge(`w-full h-full ${fit[imageFit || 'cover']}`, css?.image?.class ?? '')}
	/>
{/if}
