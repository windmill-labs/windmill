<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { FileInput } from '../../../common'
	import FileProgressBar from '../../../common/FileProgressBar.svelte'

	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService, type UploadFilePart } from '$lib/gen'
	import { writable, type Writable } from 'svelte/store'
	import { Ban, CheckCheck, FileWarning, Files, RefreshCcw, Trash } from 'lucide-svelte'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'fileinputcomponent'> | undefined = undefined
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

	async function handleChange(files: File[] | undefined) {
		for (const file of files ?? []) {
			uploadFileToS3(file, file.name)
		}
	}

	$: resolvedConfigS3 = resolvedConfig.type.configuration.s3

	let css = initCss($app.css?.fileinputcomponent, customCss)

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
		if (fileToUpload === undefined || fileToUploadKey === undefined) {
			return
		}

		const path = (await inputValue?.computeExpr({ file: fileToUpload })) ?? fileToUploadKey

		$fileUploads = $fileUploads.filter((fileUpload) => fileUpload.name !== fileToUpload.name)

		const uploadData: FileUploadData = {
			name: fileToUpload.name,
			size: fileToUpload.size,
			progress: 1, // We set it to 1 so that the progress bar is visible
			cancelled: false,
			path: path,
			file: fileToUpload
		}

		if (allFilesByKey[fileToUploadKey] !== undefined) {
			uploadData.errorMessage =
				'A file with this name already exists in the S3 bucket. If you want to replace it, delete it first.'
			$fileUploads = [...$fileUploads, uploadData]

			return
		}

		$fileUploads = [...$fileUploads, uploadData]

		let upload_id: string | undefined = undefined
		let parts: UploadFilePart[] = []

		let reader = fileToUpload?.stream().getReader()
		let { value: chunk, done: readerDone } = await reader.read()
		if (chunk === undefined || readerDone) {
			sendUserToast('Error reading file, no data read', true)
			return
		}

		let fileUploadProgress = 0
		while (true) {
			const currentFileUpload = $fileUploads.find(
				(fileUpload) => fileUpload.name === uploadData.name
			)!

			if (currentFileUpload.cancelled) {
				return
			}

			let { value: chunk_2, done: readerDone } = await reader.read()
			if (!readerDone && chunk_2 !== undefined && chunk.length <= 5 * 1024 * 1024) {
				// AWS enforces part to be bigger than 5MB, so we accumulate bytes until we reach that limit before triggering the request to the BE
				chunk = new Uint8Array([...chunk, ...chunk_2])
				continue
			}

			fileUploadProgress += (chunk.length * 100) / fileToUpload.size
			uploadData.progress = fileUploadProgress

			$fileUploads = $fileUploads.map((fileUpload) => {
				if (fileUpload.name === uploadData.name) {
					return uploadData
				}
				return fileUpload
			})

			try {
				let response = await HelpersService.multipartFileUpload({
					workspace: $workspaceStore!,
					requestBody: {
						file_key: path ?? fileToUploadKey,
						part_content: Array.from(chunk),
						upload_id: upload_id,
						parts: parts,
						is_final: readerDone,
						cancel_upload: currentFileUpload.cancelled ?? false,
						s3_resource_path: resolvedConfigS3 ? resolvedConfigS3.resource.split(':')[1] : undefined
					}
				})
				upload_id = response.upload_id
				parts = response.parts
				if (response.is_done) {
					if (currentFileUpload.cancelled) {
						sendUserToast('File upload cancelled!')
					} else {
						const curr = outputs.result.peak()

						outputs.result.set(
							curr.concat({
								path: path ?? fileToUploadKey
							})
						)

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
			} catch (e) {
				sendUserToast(e, true)
				$fileUploads = $fileUploads.map((fileUpload) => {
					if (fileUpload.name === uploadData.name) {
						fileUpload.errorMessage = e
						return fileUpload
					}
					return fileUpload
				})
				return
			}
		}
	}

	let inputValue: InputValue | undefined = undefined

	async function deleteFile(fileKey: string) {
		await HelpersService.deleteS3File({
			workspace: $workspaceStore!,
			fileKey: fileKey
		})

		const curr = outputs.result.peak()

		outputs.result.set(curr.filter((file) => file.path !== fileKey))

		sendUserToast('File deleted!')
	}
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

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.fileinputcomponent}
	/>
{/each}

{#each Object.keys(components['s3fileinputcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#if configuration.type?.['configuration']?.s3.pathTemplate}
	<InputValue
		input={configuration.type?.['configuration']?.s3.pathTemplate}
		{id}
		field="pathTemplate"
		value=""
		bind:this={inputValue}
	/>
{/if}

{#if render}
	<div class="w-full h-full p-2 flex">
		{#if $fileUploads.length > 0 && !forceDisplayUploads}
			<div class="border rounded-md flex flex-col gap-1 divide-y h-full w-full p-1">
				<div class="flex h-full overflow-y-auto flex-col">
					{#each $fileUploads as fileUpload}
						<div class="w-full flex flex-col gap-1 p-2">
							<div class="flex flex-row items-center justify-between">
								<div class="flex flex-col gap-1">
									<span class="text-xs font-bold">{fileUpload.name}</span>
									<span class="text-xs"
										>{`${Math.round((fileUpload.size / 1024 / 1024) * 100) / 100} MB`}</span
									>
								</div>
								<div class="flex flex-row gap-1 items-center">
									{#if fileUpload.errorMessage}
										<FileWarning class="w-4 h-4 text-red-500" />
									{:else if fileUpload.cancelled}
										<FileWarning class="w-4 h-4 text-yellow-500" />
									{:else if fileUpload.progress === 100}
										<CheckCheck class="w-4 h-4 text-green-500" />
									{/if}

									{#if fileUpload.cancelled || fileUpload.errorMessage !== undefined}
										<Button
											size="xs2"
											color="light"
											variant="border"
											on:click={() => {
												const file = fileUpload.file

												if (!file) {
													return
												}

												$fileUploads = $fileUploads.filter(
													(_fileUpload) => _fileUpload.name !== fileUpload.name
												)

												uploadFileToS3(file, file.name)
											}}
											startIcon={{
												icon: RefreshCcw
											}}
										>
											Retry Upload
										</Button>
										<Button
											size="xs2"
											color="light"
											variant="border"
											on:click={() => {
												const file = fileUpload.file

												if (!file) {
													return
												}

												$fileUploads = $fileUploads.filter(
													(_fileUpload) => _fileUpload.name !== fileUpload.name
												)
											}}
											startIcon={{
												icon: RefreshCcw
											}}
										>
											Remove from list
										</Button>
									{/if}
									{#if fileUpload.progress < 100 && !fileUpload.cancelled && !fileUpload.errorMessage}
										<Button
											size="xs2"
											color="light"
											variant="border"
											on:click={() => {
												fileUpload.cancelled = true
												fileUpload.progress = 0
											}}
											startIcon={{
												icon: Ban
											}}
										>
											Cancel Upload
										</Button>
									{/if}

									{#if fileUpload.progress === 100 && !fileUpload.cancelled}
										<Button
											size="xs2"
											color="red"
											variant="border"
											on:click={() => {
												$fileUploads = $fileUploads.filter(
													(_fileUpload) => _fileUpload.name !== fileUpload.name
												)

												if (fileUpload.path) {
													deleteFile(fileUpload.path)
												}
											}}
											startIcon={{
												icon: Trash
											}}
										>
											Delete
										</Button>
									{/if}
								</div>
							</div>
							<FileProgressBar
								progress={fileUpload.progress}
								color={fileUpload.errorMessage
									? '#ef4444'
									: fileUpload.cancelled
									? '#eab308'
									: fileUpload.progress === 100
									? '#22c55e'
									: '#3b82f6'}
								ended={fileUpload.cancelled || fileUpload.errorMessage !== undefined}
							>
								{#if fileUpload.errorMessage}
									<span class="text-xs text-red-600">{fileUpload.errorMessage}</span>
								{:else if fileUpload.cancelled}
									<span class="text-xs text-yellow-600">Upload cancelled</span>
								{/if}
							</FileProgressBar>
							{#if !(fileUpload.cancelled || fileUpload.errorMessage !== undefined)}
								<span class="text-xs text-gray-500 dark:text-gray-200">
									{fileUpload.progress === 100 ? 'Upload finished' : `Uploading`} to path: {fileUpload.path}
								</span>
							{/if}
						</div>
					{/each}
				</div>
				<div class="flex flex-row gap-1 items-center justify-end p-1">
					{#if !$fileUploads.every((fileUpload) => fileUpload.progress === 100 || fileUpload.cancelled)}
						<Button
							size="xs2"
							color="light"
							on:click={() => {
								$fileUploads = $fileUploads.map((fileUpload) => {
									if (fileUpload.progress === 100 || fileUpload.cancelled) {
										return fileUpload
									}

									fileUpload.cancelled = true
									fileUpload.progress = 0
									return fileUpload
								})
							}}
							startIcon={{
								icon: Ban
							}}
						>
							Cancel All Uploads
						</Button>
					{/if}
					<Button
						size="xs2"
						color="light"
						on:click={() => {
							forceDisplayUploads = true
						}}
						startIcon={{
							icon: Files
						}}
						disabled={$fileUploads.some(
							(fileUpload) => fileUpload.progress !== 100 && !fileUpload.cancelled
						)}
					>
						Upload more files
					</Button>
				</div>
			</div>
		{:else}
			<FileInput
				accept={resolvedConfigS3.acceptedFileTypes?.length
					? resolvedConfigS3.acceptedFileTypes?.join(', ')
					: undefined}
				multiple={resolvedConfigS3.allowMultiple}
				returnFileNames
				includeMimeType
				on:change={({ detail }) => {
					forceDisplayUploads = false
					handleChange(detail)
				}}
				class={twMerge('w-full h-full', css?.container?.class, 'wm-file-input')}
				style={css?.container?.style}
			>
				{resolvedConfigS3.text}
			</FileInput>
		{/if}
	</div>
{/if}
