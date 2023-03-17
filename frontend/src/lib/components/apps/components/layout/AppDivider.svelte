<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppInput } from '../../inputType'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		HorizontalAlignment,
		RichConfigurations,
		VerticalAlignment
	} from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: HorizontalAlignment | undefined = undefined
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let customCss: ComponentCustomCSS<'container' | 'divider'> | undefined = undefined
	export let position: 'horizontal' | 'vertical'
	export let render: boolean

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	let size = 2
	let color = '#00000060'

	$: css = concatCustomCss($app.css?.[position + 'dividercomponent'], customCss)
</script>

<InputValue {id} input={configuration.size} bind:value={size} />
<InputValue {id} input={configuration.color} bind:value={color} />

<AlignWrapper
	{horizontalAlignment}
	{verticalAlignment}
	class={twMerge(css?.container?.class, 'h-full')}
	style={css?.container?.style}
	{render}
>
	<div
		class={twMerge(
			`rounded-full ${position === 'horizontal' ? 'w-full' : 'h-full'}`,
			css?.divider?.class ?? ''
		)}
		style="
			{position === 'horizontal' ? 'height' : 'width'}: {size}px;
			background-color: {color};
			{css?.divider?.style ?? ''}
		"
	/>
</AlignWrapper>
