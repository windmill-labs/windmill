<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { userStore } from '$lib/stores'
	import { isPartialS3Object, getS3File } from '../../editor/appUtilsS3'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'pdfcomponent'> | undefined
		render: boolean
	}

	let { id, configuration, customCss = undefined, render }: Props = $props()

	const { app, worldStore, appPath, workspace, isEditor } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		loading: false
	})

	let source: string | ArrayBuffer | undefined = $state(undefined)
	let zoom: number | undefined = $state(undefined)

	let pdfSource: string | ArrayBuffer | undefined = $state(undefined)

	let token = getContext<{ token?: string }>('AuthToken')

	async function loadSource() {
		if (isPartialS3Object(source)) {
			pdfSource = await getS3File({
				source: source.s3,
				storage: source.storage,
				presigned: source.presigned,
				appPath: $appPath,
				username: $userStore?.username,
				workspace,
				token: token?.token,
				isEditor,
				configuration
			})
		} else if (source && typeof source !== 'string' && !(source instanceof ArrayBuffer)) {
			throw new Error('Invalid PDF source object' + typeof source)
		} else if (typeof source === 'string' && source?.startsWith('s3://')) {
			pdfSource = await getS3File({
				source: source?.replace('s3://', ''),
				appPath: $appPath,
				username: $userStore?.username,
				workspace,
				token: token?.token,
				isEditor,
				configuration
			})
		} else {
			pdfSource = source
		}
	}

	$effect(() => {
		source && loadSource()
	})

	let css = $state(initCss($app.css?.pdfcomponent, customCss))
</script>

<InputValue key="source" {id} input={configuration.source} bind:value={source} />
<InputValue key="zoom" {id} input={configuration.zoom} bind:value={zoom} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.pdfcomponent}
	/>
{/each}

<InitializeComponent {id} />

{#if render}
	<div class="relative w-full h-full bg-gray-100 component-wrapper">
		{#await import('$lib/components/display/PdfViewer.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				source={pdfSource}
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
		{/await}
	</div>
{/if}
