<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { FileInput } from '../../../common'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'

	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { RunnableComponent, RunnableWrapper } from '../helpers'
	import type { AppInput } from '../../inputType'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService, type UploadFilePart } from '$lib/gen'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'fileinputcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined

	let input: AppInput | undefined = undefined
	let runnableComponent: RunnableComponent
	let loading = false

	let resolvedConfig = initConfig(
		components['s3fileinputcomponent'].initialData.configuration,
		configuration
	)

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: [] as { name: string; data: string }[] | undefined,
		loading: false,
		jobId: undefined
	})

	// Receives Base64 encoded strings from the input component
	async function handleChange(files: File[] | undefined) {
		for (const file of files ?? []) {
			uploadFileToS3(file, file.name)
		}
		//outputs?.result.set(files)
	}

	let css = initCss($app.css?.fileinputcomponent, customCss)
	let done: boolean = false

	let uploadModalOpen = false
	let fileUploadProgress: number | undefined = undefined
	let fileUploadCancelled: boolean = false
	let fileUploadErrorMsg: string | undefined = undefined

	let allFilesByKey: Record<
		string,
		{
			type: 'folder' | 'leaf'
			full_key: string
			display_name: string
			collapsed: boolean
			parentPath: string | undefined
			nestingLevel: number
		}
	> = {}

	async function uploadFileToS3(fileToUpload: File, fileToUploadKey: string) {
		fileUploadErrorMsg = undefined
		if (fileToUpload === undefined || fileToUploadKey === undefined) {
			return
		}
		if (allFilesByKey[fileToUploadKey] !== undefined) {
			fileUploadErrorMsg =
				'A file with this name already exists in the S3 bucket. If you want to replace it, delete it first.'
			return
		}

		let upload_id: string | undefined = undefined
		let parts: UploadFilePart[] = []

		let reader = fileToUpload?.stream().getReader()
		let { value: chunk, done: readerDone } = await reader.read()
		if (chunk === undefined || readerDone) {
			sendUserToast('Error reading file, no data read', true)
			return
		}

		fileUploadProgress = 0
		while (true) {
			let { value: chunk_2, done: readerDone } = await reader.read()
			if (!readerDone && chunk_2 !== undefined && chunk.length <= 5 * 1024 * 1024) {
				// AWS enforces part to be bigger than 5MB, so we accumulate bytes until we reach that limit before triggering the request to the BE
				chunk = new Uint8Array([...chunk, ...chunk_2])
				continue
			}
			fileUploadProgress += (chunk.length * 100) / fileToUpload.size
			let response = await HelpersService.multipartFileUpload({
				workspace: $workspaceStore!,
				requestBody: {
					file_key: fileToUploadKey,
					part_content: Array.from(chunk),
					upload_id: upload_id,
					parts: parts,
					is_final: readerDone,
					cancel_upload: fileUploadCancelled
				}
			})
			upload_id = response.upload_id
			parts = response.parts
			if (response.is_done) {
				if (fileUploadCancelled) {
					sendUserToast('File upload cancelled!')
				} else {
					sendUserToast('File upload finished!')
				}
				break
			}
			if (chunk_2 === undefined) {
				sendUserToast(
					'File upload is not finished, yet there is no more data to stream. This is unexpected',
					true
				)
				return
			}
			chunk = chunk_2
		}
		uploadModalOpen = false

		if (!fileUploadCancelled) {
			/*
			selectedFileKey = { s3: fileToUploadKey }
			await loadFiles()
			await loadFileMetadataPlusPreviewAsync(selectedFileKey['s3'])
			*/
		}

		fileUploadProgress = undefined
		fileUploadCancelled = false
		fileUploadErrorMsg = undefined
	}
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.fileinputcomponent}
	/>
{/each}

{#each Object.keys(components['buttoncomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if render}
	<div class="w-full h-full p-1">
		<div class="flex w-full bg-gray-200 rounded-full h-4 overflow-hidden">
			<div class="h-full bg-blue-400" style="width: {fileUploadProgress ?? 0}%" />
		</div>

		{#if done}
			<div class="flex items-center justify-center h-full flex-col gap-2">
				{resolvedConfig.submittedFileText}
				<Button
					size="xs"
					on:click={() => {
						done = false
						outputs?.result.set(undefined)
					}}
				>
					Restart
				</Button>
			</div>
		{:else}
			<FileInput
				accept={resolvedConfig.acceptedFileTypes?.length
					? resolvedConfig.acceptedFileTypes?.join(', ')
					: undefined}
				multiple={resolvedConfig.allowMultiple}
				returnFileNames
				includeMimeType
				on:change={({ detail }) => {
					handleChange(detail)
					done = true
				}}
				class={twMerge('w-full h-full', css?.container?.class, 'wm-file-input')}
				style={css?.container?.style}
			>
				{resolvedConfig.text}
			</FileInput>
		{/if}
	</div>
{/if}

<RunnableWrapper
	noInitialize
	bind:runnableComponent
	bind:loading
	componentInput={input}
	autoRefresh={false}
	render={false}
	id={`${id}`}
	{outputs}
/>
