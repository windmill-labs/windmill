<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initOutput } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		HorizontalAlignment,
		RichConfigurations,
		VerticalAlignment
	} from '../../types'
	import { TailwindClassPatterns, initCss, hasTailwindClass } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: HorizontalAlignment | undefined = undefined
	export let verticalAlignment: VerticalAlignment | undefined = undefined
	export let customCss:
		| ComponentCustomCSS<'verticaldividercomponent' | 'horizontaldividercomponent'>
		| undefined = undefined
	export let position: 'horizontal' | 'vertical'
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let size = 2
	let color = '#00000060'

	let css = initCss(app.val.css?.[position + 'dividercomponent'], customCss)

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	function getSize() {
		if (position === 'horizontal') {
			return hasTailwindClass(css?.divider?.class, TailwindClassPatterns.height)
				? ''
				: `height: ${size}px;`
		} else if (position === 'vertical') {
			return hasTailwindClass(css?.divider?.class, TailwindClassPatterns.width)
				? ''
				: `width: ${size}px;`
		}
	}
</script>

<InputValue key="size" {id} input={configuration.size} bind:value={size} />
<InputValue key="color" {id} input={configuration.color} bind:value={color} />
<InitializeComponent {id} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={app.val.css?.[position + 'dividercomponent']}
	/>
{/each}

<AlignWrapper
	{horizontalAlignment}
	{verticalAlignment}
	class={twMerge(
		css?.container?.class,
		position === 'horizontal' ? 'wm-horizontal-divider-container' : 'wm-vertical-divider-container',
		'h-full'
	)}
	style={css?.container?.style}
	{render}
>
	<div
		class={twMerge(
			`rounded-full ${position === 'horizontal' ? 'w-full' : 'h-full'}`,
			css?.divider?.class,
			position === 'horizontal' ? 'wm-horizontal-divider' : 'wm-vertical-divider'
		)}
		style="
			{getSize()}
			{css?.divider?.style ?? ''}
			{hasTailwindClass(css?.divider?.class, TailwindClassPatterns.bg)
			? ''
			: `background-color: ${color};`}
		"
	></div>
</AlignWrapper>
