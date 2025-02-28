<script lang="ts">
	import { getContext } from 'svelte'

	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import type { FileUploadData } from '../../inputType'
	import { initCss } from '../../utils'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import { writable, type Writable } from 'svelte/store'
	import FileUpload from '$lib/components/common/fileUpload/FileUpload.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { computeS3FileInputPolicy } from '../../editor/appUtilsS3'
	import { defaultIfEmptyString } from '$lib/utils'
	import { userStore } from '$lib/stores'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'s3fileinputcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined
	export let onFileChange: string[] | undefined = undefined

	let resolvedConfig = initConfig(
		components['s3fileinputcomponent'].initialData.configuration,
		configuration
	)

	let fileUploads: Writable<FileUploadData[]> = writable([])
	const { app, worldStore, componentControl, runnableComponents, workspace } =
		getContext<AppViewerContext>('AppViewerContext')

	$componentControl[id] = {
		clearFiles: () => {
			outputs.result.set([])
			$fileUploads = []
		}
	}

	let value: { path: string; filename: string }[] | undefined = undefined
	const outputs = initOutput($worldStore, id, {
		result: value ?? ([] as { path: string; filename: string }[] | undefined),
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
											Copy direct link
										</Button>
									{/if}
									*/
	let forceDisplayUploads: boolean = false

	const { appPath, isEditor } = getContext<AppViewerContext>('AppViewerContext')
	function computeForceViewerPolicies() {
		if (!isEditor) {
			return undefined
		}
		const policy = computeS3FileInputPolicy((configuration as any)?.type?.configuration?.s3, $app)
		return policy
	}
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
		allowDelete={resolvedConfigS3.allowDelete}
		containerText={resolvedConfigS3.text}
		customResourcePath={resolvedConfigS3.resource}
		customResourceType="s3"
		customClass={css?.container?.class}
		customStyle={css?.container?.style}
		disabled={resolvedConfigS3?.disabled}
		{fileUploads}
		{workspace}
		on:addition={(evt) => {
			const curr = outputs.result.peak()
			value = curr.concat(evt.detail)
			outputs.result.set(value)
			onFileChange?.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((cb) => cb?.()))
		}}
		on:deletion={(evt) => {
			const curr = outputs.result.peak()
			value = curr.filter((file) => file.path !== evt.detail?.path)
			outputs.result.set(value)
		}}
		{forceDisplayUploads}
		appPath={defaultIfEmptyString($appPath, `u/${$userStore?.username ?? 'unknown'}/newapp`)}
		{computeForceViewerPolicies}
	/>
{/if}
