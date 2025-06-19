<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import PdfViewer from '$lib/components/display/PdfViewer.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'pdfcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		loading: false
	})

	let source: string | ArrayBuffer | undefined = undefined
	let zoom: number | undefined = undefined

	let css = initCss(app.val.css?.pdfcomponent, customCss)
</script>

<InputValue key="source" {id} input={configuration.source} bind:value={source} />
<InputValue key="zoom" {id} input={configuration.zoom} bind:value={zoom} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={app.val.css?.pdfcomponent}
	/>
{/each}

<InitializeComponent {id} />

{#if render}
	<div class="relative w-full h-full bg-gray-100 component-wrapper">
		<PdfViewer
			{source}
			{zoom}
			class={css?.container?.class}
			style={css?.container?.style}
			on:loading={() => {
				outputs.loading.set(true)
			}}
			on:loaded={() => {
				outputs.loading.set(false)
			}}
		/>
	</div>
{/if}
