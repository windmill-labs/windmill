<script lang="ts">
	import { getContext } from 'svelte'

	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import { writable, type Writable } from 'svelte/store'
	import FileUpload from '$lib/components/common/fileUpload/FileUpload.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'s3fileinputcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined

	let resolvedConfig = initConfig(
		components['s3fileinputcomponent'].initialData.configuration,
		configuration
	)

	type FileUploadData = {
		name: string
		size: number
		progress: number
		cancelled?: boolean
		errorMessage?: string
		path?: string
		file?: File
	}

	let fileUploads: Writable<FileUploadData[]> = writable([])
	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	$componentControl[id] = {
		clearFiles: () => {
			outputs.result.set([])
			$fileUploads = []
		}
	}

	const outputs = initOutput($worldStore, id, {
		result: [] as { path: string }[] | undefined,
		loading: false,
		jobId: undefined
	})

	$: resolvedConfigS3 = resolvedConfig.type.configuration.s3

	let css = initCss($app.css?.fileinputcomponent, customCss)
	/*

		{#if resolvedConfig.displayDirectLink && fileUpload.progress === 100}
										<Button
											color="light"
											on:click={() => {
												if (!fileUpload.path) {
													return
												}

												loadFileMetadata(fileUpload.path)
												copyToClipboard(
													`https://resolvedConfig.bucketName.s3.amazonaws.com/${fileUpload.name}`
												)
											}}
											size="xs2"
											variant="border"
										>
											Copy Direct Link
										</Button>
									{/if}
									*/
	let forceDisplayUploads: boolean = false
</script>

<InitializeComponent {id} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.fileinputcomponent}
	/>
{/each}

{#each Object.entries(components['s3fileinputcomponent'].initialData.configuration) as [key, value] (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
		initialConfig={value}
	/>
{/each}

<!-- {#if configuration.type?.['configuration']?.s3.pathTemplate}
	<InputValue
		input={configuration.type?.['configuration']?.s3.pathTemplate}
		{id}
		field="pathTemplate"
		value=""
		bind:this={inputValue}
		onDemandOnly
	/>
{/if} -->

{#if render}
	<FileUpload
		acceptedFileTypes={resolvedConfigS3.acceptedFileTypes?.length
			? resolvedConfigS3.acceptedFileTypes
			: undefined}
		pathTransformer={resolvedConfig?.type?.configuration?.s3?.pathTemplate}
		allowMultiple={resolvedConfigS3.allowMultiple}
		containerText={resolvedConfigS3.text}
		customResourcePath={resolvedConfigS3.resource}
		customResourceType="s3"
		customClass={css?.container?.class}
		customStyle={css?.container?.style}
		on:addition={(evt) => {
			const curr = outputs.result.peak()
			outputs.result.set(curr.concat(evt.detail))
		}}
		on:deletion={(evt) => {
			const curr = outputs.result.peak()
			outputs.result.set(curr.filter((file) => file.path !== evt.detail?.path))
		}}
		{forceDisplayUploads}
	/>
{/if}
