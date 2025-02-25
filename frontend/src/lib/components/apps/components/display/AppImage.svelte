<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import { HelpersService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import Loader from '../helpers/Loader.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'imagecomponent'> | undefined = undefined
	export let render: boolean

	const resolvedConfig = initConfig(
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

	let css = initCss($app.css?.imagecomponent, customCss)

	let imageUrl: string | undefined = undefined

	async function getS3Image(source: string | undefined) {
		if (!source || !$workspaceStore) return ''

		try {
			const file = await HelpersService.fileDownload({
				workspace: $workspaceStore,
				fileKey: source
			})
			const imageUrl = URL.createObjectURL(file)
			return imageUrl
		} catch (error) {
			return ''
		}
	}

	async function loadImage() {
		if (resolvedConfig.sourceKind === 's3 (workspace storage)') {
			imageUrl = await getS3Image(resolvedConfig.source)
		} else if (resolvedConfig.sourceKind === 'png encoded as base64') {
			imageUrl = 'data:image/png;base64,' + resolvedConfig.source
		} else if (resolvedConfig.sourceKind === 'jpeg encoded as base64') {
			imageUrl = 'data:image/jpeg;base64,' + resolvedConfig.source
		} else if (resolvedConfig.sourceKind === 'svg encoded as base64') {
			imageUrl = 'data:image/svg+xml;base64,' + resolvedConfig.source
		} else {
			imageUrl = resolvedConfig.source
		}
	}

	$: resolvedConfig && loadImage()
</script>

<InitializeComponent {id} />

{#each Object.keys(components['imagecomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.imagecomponent}
	/>
{/each}

{#if render}
	<Loader loading={imageUrl === undefined || !$workspaceStore}>
		{#if imageUrl}
			<img
				on:pointerdown|preventDefault
				src={imageUrl}
				alt={resolvedConfig.altText}
				style={css?.image?.style ?? ''}
				class={twMerge(
					`w-full h-full ${fit[resolvedConfig.imageFit || 'cover']}`,
					css?.image?.class,
					'wm-image'
				)}
			/>
		{/if}
	</Loader>
{/if}
