<script lang="ts">
	import { createBubbler, preventDefault } from 'svelte/legacy'

	const bubble = createBubbler()
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import Loader from '../helpers/Loader.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { defaultIfEmptyString } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import { computeS3ImageViewerPolicy, isPartialS3Object } from '../../editor/appUtilsS3'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'imagecomponent'> | undefined
		render: boolean
	}

	let { id, configuration, customCss = undefined, render }: Props = $props()

	function computeForceViewerPolicies() {
		if (!isEditor) {
			return undefined
		}
		const policy = computeS3ImageViewerPolicy(configuration)
		return policy
	}

	const resolvedConfig = $state(
		initConfig(components['imagecomponent'].initialData.configuration, configuration)
	)

	const { app, appPath, worldStore, workspace, isEditor } =
		getContext<AppViewerContext>('AppViewerContext')
	const fit: Record<string, string> = {
		cover: 'object-cover',
		contain: 'object-contain',
		fill: 'object-fill'
	}

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let css = $state(initCss($app.css?.imagecomponent, customCss))

	let imageUrl: string | undefined = $state(undefined)

	async function getS3Image(source: string | undefined, storage?: string, presigned?: string) {
		if (!source) return ''
		const appPathOrUser = defaultIfEmptyString(
			$appPath,
			`u/${$userStore?.username ?? 'unknown'}/newapp`
		)
		const params = new URLSearchParams()
		params.append('s3', source)
		if (storage) {
			params.append('storage', storage)
		}

		const forceViewerPolicies = computeForceViewerPolicies()
		if (forceViewerPolicies) {
			params.append('force_viewer_allowed_s3_keys', JSON.stringify([forceViewerPolicies]))
		}

		return `/api/w/${workspace}/apps_u/download_s3_file/${appPathOrUser}?${params.toString()}${presigned ? `&${presigned}` : ''}`
	}

	async function loadImage() {
		if (isPartialS3Object(resolvedConfig.source)) {
			imageUrl = await getS3Image(
				resolvedConfig.source.s3,
				resolvedConfig.source.storage,
				resolvedConfig.source.presigned
			)
		} else if (resolvedConfig.source && typeof resolvedConfig.source !== 'string') {
			throw new Error('Invalid image object' + typeof resolvedConfig.source)
		} else if (
			resolvedConfig.sourceKind === 's3 (workspace storage)' ||
			resolvedConfig.source?.startsWith('s3://')
		) {
			imageUrl = await getS3Image(resolvedConfig.source?.replace('s3://', ''))
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

	$effect(() => {
		resolvedConfig && loadImage()
	})
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
	<Loader loading={imageUrl === undefined}>
		{#if imageUrl}
			<img
				onpointerdown={preventDefault(bubble('pointerdown'))}
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
