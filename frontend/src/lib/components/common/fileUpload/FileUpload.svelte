<script lang="ts">
	import { FileInput } from '../'
	import FileProgressBar from '../FileProgressBar.svelte'

	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { AppService, HelpersService } from '$lib/gen'
	import { writable, type Writable } from 'svelte/store'
	import { Ban, CheckCheck, FileWarning, Files, RefreshCcw, Trash, XIcon } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { emptyString } from '$lib/utils'

	interface Props {
		acceptedFileTypes?: string[] | undefined
		allowMultiple?: boolean
		allowDelete?: boolean
		folderOnly?: boolean
		containerText?: string
		customResourcePath?: string | undefined
		customResourceType?: 's3' | 'azure_blob' | undefined // when customResourcePath is provided, this should be provided as well. Will default to S3 if not
		customClass?: string
		customStyle?: string
		randomFileKey?: boolean
		pathTransformer?: any // function taking as input {file: File} and returning a string
		forceDisplayUploads?: boolean
		defaultValue?: string | string[] | undefined
		workspace?: string | undefined
		fileUploads?: Writable<FileUploadData[]>
		appPath?: string | undefined
		disabled?: boolean
		iconSize?: number | undefined
		initialValue?:
			| {
					s3: string
					filename: string
			  }
			| {
					s3: string
					filename: string
			  }[]
			| undefined
		computeForceViewerPolicies?:
			| (() =>
					| {
							allowed_resources: string[]
							allow_user_resources: boolean
							allow_workspace_resource: boolean
							file_key_regex: string
					  }
					| undefined)
			| undefined
	}

	let {
		acceptedFileTypes = ['*'],
		allowMultiple = true,
		allowDelete = true,
		folderOnly = false,
		containerText = folderOnly
			? 'Drag and drop a folder here or click to browse'
			: allowMultiple
				? 'Drag and drop files here or click to browse'
				: 'Drag and drop a file here or click to browse',
		customResourcePath = undefined,
		customResourceType = undefined,
		customClass = '',
		customStyle = '',
		randomFileKey = false,
		pathTransformer = undefined,
		forceDisplayUploads = $bindable(false),
		defaultValue = undefined,
		workspace = undefined,
		fileUploads = writable([]),
		appPath = undefined,
		disabled = false,
		iconSize = undefined,
		initialValue = undefined,
		computeForceViewerPolicies = undefined
	}: Props = $props()

	const dispatch = createEventDispatcher<{
		addition: { path?: string; filename?: string }
		deletion: { path: string }
	}>()

	let initialS3 = $derived(
		Array.isArray(initialValue)
			? initialValue?.map((v) => v.s3)
			: initialValue?.s3
				? [initialValue?.s3]
				: undefined
	)
	init()

	function transform(initialValue: { s3: string; filename?: string }) {
		return {
			name: initialValue?.filename ?? initialValue?.s3 ?? 'Unknown file',
			path: initialValue?.s3,
			fromBucket: true
		}
	}
	function init() {
		if (initialS3) {
			for (const s3 of initialS3) {
				if (!$fileUploads.find((fileUpload) => fileUpload.path === s3)) {
					let initialFileUploads = initialValue
						? Array.isArray(initialValue)
							? initialValue.map(transform)
							: [transform(initialValue)]
						: []
					$fileUploads = [...$fileUploads, ...initialFileUploads]
				}
			}
		}
	}

	export function addUpload(upload: { s3: string; filename?: string }) {
		$fileUploads = [...$fileUploads, transform(upload)]
	}

	export function setUpload(upload: { s3: string; filename?: string }) {
		$fileUploads = [transform(upload)]
	}

	type FileUploadData = {
		name: string
		size?: number
		progress?: number
		cancelled?: boolean
		errorMessage?: string
		path?: string
		file?: File
		deleteToken?: string
		fromBucket?: boolean
	}

	async function handleChange(files: File[] | undefined) {
		if (folderOnly) {
			uniqueFolderPrefix = getRandomFolderPrefix()
			uploadedFolderRoot = undefined
			await Promise.all(
				files?.map(async (file) => {
					await uploadFileToS3(file, file.name)
				}) ?? []
			)
			if (uploadedFolderRoot) {
				dispatch('addition', {
					path: uniqueFolderPrefix + uploadedFolderRoot,
					filename: undefined
				})
				sendUserToast('Folder uploaded!')
			}
		} else {
			for (const file of files ?? []) {
				uploadFileToS3(file, file.name)
			}
		}
	}

	let activeUploads: { xhr: XMLHttpRequest; fileName: string }[] = []

	function getRandomFolderPrefix(): string {
		const now = Date.now()
		const randomNum = Math.floor(Math.random() * 65536)
		return `windmill_uploads/upload_${now}_${randomNum}/`
	}

	let uniqueFolderPrefix = getRandomFolderPrefix()
	let uploadedFolderRoot: string | undefined = undefined

	async function uploadFileToS3(fileToUpload: File & { path?: string }, fileToUploadKey: string) {
		if (fileToUpload === undefined || fileToUploadKey === undefined) {
			return
		}
		let path: string | undefined = undefined
		let fileExtension: string | undefined = undefined
		if (folderOnly) {
			const relativePath = fileToUpload.webkitRelativePath || fileToUpload.path
			if (!relativePath) {
				throw new Error('Missing file relative path')
			}
			const rootFolder = relativePath.split('/')[0]
			if (!rootFolder) {
				throw new Error('Missing root folder')
			}
			if (uploadedFolderRoot && rootFolder !== uploadedFolderRoot) {
				throw new Error('Uploading a file in a different root folder than the previous files')
			} else {
				uploadedFolderRoot = rootFolder
			}
			path = uniqueFolderPrefix + relativePath
		} else if (randomFileKey) {
			fileExtension = fileToUpload.name.split('.').pop()
			if (emptyString(fileExtension)) {
				fileExtension = undefined
			}
		} else {
			path =
				typeof pathTransformer == 'function'
					? ((await pathTransformer?.({
							file: fileToUpload
						})) ?? fileToUploadKey)
					: fileToUploadKey
		}
		const uploadData: FileUploadData = {
			name: fileToUpload.name,
			size: fileToUpload.size,
			progress: 1, // We set it to 1 so that the progress bar is visible
			cancelled: false,
			path: path,
			file: fileToUpload,
			deleteToken: undefined
		}

		$fileUploads = [...$fileUploads, uploadData]

		// // Use a custom TransformStream to track upload progress
		// const progressTrackingStream = new TransformStream({
		// 	transform(chunk, controller) {
		// 		controller.enqueue(chunk)
		// 		bytesUploaded += chunk.byteLength
		// 		console.log('upload progress:', bytesUploaded / totalBytes)
		// 		uploadData.progress = (bytesUploaded / totalBytes) * 100
		// 	},
		// 	flush(controller) {
		// 		console.log('completed stream')
		// 	}
		// })

		try {
			// const response = await HelpersService.multipartFileUpload({
			// 	workspace: $workspaceStore!,
			// 	fileKey: path,
			// 	fileExtension: fileExtension,
			// 	s3ResourcePath: customS3ResourcePath?.split(':')[1],
			// 	requestBody: fileToUpload.stream().pipeThrough(progressTrackingStream, {})
			// })

			const params = new URLSearchParams()
			if (path) {
				params.append('file_key', path)
			}
			if (customResourcePath?.split(':')[1]) {
				params.append('s3_resource_path', customResourcePath?.split(':')[1])
				params.append('resource_type', customResourceType === 'azure_blob' ? 'AzureBlob' : 'S3')
			}
			if (fileExtension) {
				params.append('file_extension', fileExtension)
			}
			if (fileToUpload.type) {
				params.append('content_type', fileToUpload.type)
			}

			if (computeForceViewerPolicies !== undefined) {
				const forceViewerPolicies = computeForceViewerPolicies()

				if (forceViewerPolicies) {
					params.append(
						'force_viewer_allowed_resources',
						forceViewerPolicies.allowed_resources.join(',')
					)
					params.append(
						'force_viewer_allow_user_resources',
						JSON.stringify(forceViewerPolicies.allow_user_resources)
					)
					params.append(
						'force_viewer_allow_workspace_resource',
						JSON.stringify(forceViewerPolicies.allow_workspace_resource)
					)
					params.append('force_viewer_file_key_regex', forceViewerPolicies.file_key_regex)
				}
			}

			// let response = await fetch(
			// 	`/api/w/${$workspaceStore}/job_helpers/multipart_upload_s3_file?${params.toString()}`,
			// 	{
			// 		method: 'POST',
			// 		headers: {
			// 			'Content-Type': 'application/octet-stream'
			// 		},
			// 		body: fileToUpload.stream().pipeThrough(progressTrackingStream, {}),
			// 		duplex: 'half'
			// 	}
			// )
			let xhr = new XMLHttpRequest()

			activeUploads.push({ xhr, fileName: fileToUpload.name })

			const response = (await new Promise((resolve, reject) => {
				xhr?.upload.addEventListener('progress', (event) => {
					if (event.lengthComputable) {
						let progress = (event.loaded / event.total) * 100
						if (progress == 100) {
							progress = 99
						}
						console.log('upload progress:', progress)
						uploadData.progress = progress
						$fileUploads = $fileUploads
					}
				})

				xhr?.addEventListener('loadend', () => {
					activeUploads = activeUploads.filter((x) => x.fileName !== fileToUpload.name)
					let response = xhr?.responseText
					if (xhr?.readyState === 4 && xhr?.status === 200 && response) {
						uploadData.progress = 100
						resolve(JSON.parse(response))
					} else {
						xhr?.abort()
						if (response) {
							reject(response)
						} else {
							reject('An error occurred while uploading the file, see server logs')
						}
					}
				})

				xhr?.open(
					'POST',
					appPath
						? `/api/w/${
								workspace ?? $workspaceStore
							}/apps_u/upload_s3_file/${appPath}?${params.toString()}`
						: `/api/w/${
								workspace ?? $workspaceStore
							}/job_helpers/upload_s3_file?${params.toString()}`,
					true
				)
				xhr?.setRequestHeader('Content-Type', 'application/octet-stream')
				xhr?.send(fileToUpload)
			})) as any

			uploadData.path = response.file_key
			if (appPath && 'delete_token' in response) {
				uploadData.deleteToken = response.delete_token
			}
		} catch (e) {
			console.error(e)
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
		if (!folderOnly) {
			dispatch('addition', { path: uploadData.path, filename: fileToUpload.name })
			sendUserToast('File uploaded!')
		}

		uploadData.progress = 100
		$fileUploads = $fileUploads.map((fileUpload) => {
			if (fileUpload.name === uploadData.name) {
				return uploadData
			}
			return fileUpload
		})
		return
	}

	async function deleteFile(fileKey: string, deleteToken?: string) {
		try {
			if (deleteToken) {
				await AppService.deleteS3FileFromApp({
					workspace: workspace ?? $workspaceStore!,
					deleteToken: deleteToken
				})
			} else {
				await HelpersService.deleteS3File({
					workspace: workspace ?? $workspaceStore!,
					fileKey: fileKey
				})
			}
			dispatch('deletion', { path: fileKey })
			sendUserToast('File deleted!')
		} catch (err) {
			console.error(err)
			sendUserToast('Could not delete file', true)
		}
	}

	function clearRequests() {
		activeUploads.forEach(({ xhr }) => xhr.abort())
		activeUploads = []
	}

	function abortUpload(fileName: string) {
		const upload = activeUploads.find((x) => x.fileName === fileName)
		if (upload) {
			upload.xhr.abort()
			activeUploads = activeUploads.filter((x) => x.fileName !== fileName)
		}
	}

	onDestroy(() => {
		clearRequests()
	})
</script>

{#snippet fileInput()}
	<FileInput
		{folderOnly}
		{disabled}
		accept={acceptedFileTypes?.join(',')}
		multiple={allowMultiple}
		returnFileNames
		{iconSize}
		on:change={({ detail }) => {
			forceDisplayUploads = false
			handleChange(detail)
		}}
		class={twMerge('w-full h-full', customClass, 'wm-file-input')}
		style={customStyle}
		defaultFile={defaultValue}
	>
		{containerText}{#if disabled}<br />(Disabled){/if}
	</FileInput>
{/snippet}

<div class="w-full h-full p-0 flex flex-col">
	{#if $fileUploads.length > 0}
		<div class="border rounded-md flex flex-col gap-1 divide-y h-full w-full p-1">
			<div class="flex h-full overflow-y-auto flex-col">
				{#each $fileUploads as fileUpload}
					<div class="w-full flex flex-col gap-1 p-2">
						<div class="flex flex-row items-center justify-between">
							<div class="flex flex-col gap-1">
								<span class="text-xs font-bold">{fileUpload.name ?? ''}</span>
								{#if fileUpload.size}
									<span class="text-xs"
										>{`${Math.round((fileUpload.size / 1024 / 1024) * 100) / 100} MB`}</span
									>
								{/if}
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

											abortUpload(fileUpload.name)

											$fileUploads = $fileUploads.filter(
												(_fileUpload) => _fileUpload.name !== fileUpload.name
											)

											uploadFileToS3(file, file.name)
										}}
										startIcon={{
											icon: RefreshCcw
										}}
									>
										Retry upload
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

											clearRequests()

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
								{#if fileUpload.progress !== undefined && fileUpload.progress < 100 && !fileUpload.cancelled && !fileUpload.errorMessage}
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
										Cancel upload
									</Button>
								{/if}

								{#if fileUpload.progress === 100 && !fileUpload.cancelled && allowDelete && (!appPath || fileUpload.deleteToken != undefined)}
									<Button
										size="xs2"
										color="red"
										variant="border"
										on:click={() => {
											$fileUploads = $fileUploads.filter(
												(_fileUpload) => _fileUpload.name !== fileUpload.name
											)

											if (fileUpload.path) {
												deleteFile(fileUpload.path, fileUpload.deleteToken)
											}
											abortUpload(fileUpload.name)
										}}
										startIcon={{
											icon: Trash
										}}
									>
										Delete
									</Button>
								{:else if fileUpload.fromBucket}
									<Button
										size="xs2"
										color="red"
										variant="border"
										on:click={() => {
											$fileUploads = $fileUploads.filter(
												(_fileUpload) => _fileUpload.name !== fileUpload.name
											)

											if (fileUpload.path) {
												dispatch('deletion', { path: fileUpload.path })
											}
										}}
										startIcon={{
											icon: XIcon
										}}
									>
										Remove
									</Button>
								{/if}
							</div>
						</div>
						{#if !fileUpload.fromBucket}
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
						{/if}
						{#if !(fileUpload.cancelled || fileUpload.errorMessage !== undefined)}
							<span class="text-xs text-gray-500 dark:text-gray-200">
								{#if fileUpload.fromBucket}
									{fileUpload.path ?? 'N/A'}
								{:else}
									{fileUpload.progress === 100 ? 'Upload finished' : `Uploading`} to path: {fileUpload.path ??
										'N/A'}
								{/if}
							</span>
						{/if}
					</div>
				{/each}
			</div>
			<div class="flex flex-row gap-1 items-center justify-end p-1">
				{#if !forceDisplayUploads && (allowMultiple || folderOnly) && !$fileUploads.every((fileUpload) => fileUpload.progress === 100 || fileUpload.cancelled || fileUpload.fromBucket)}
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
						Cancel all uploads
					</Button>
				{/if}
				{#if allowMultiple}
					{#if forceDisplayUploads}
						{@render fileInput()}
					{:else}
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
								(fileUpload) =>
									fileUpload.progress !== 100 && !fileUpload.cancelled && !fileUpload.fromBucket
							)}
						>
							Upload more files
						</Button>
					{/if}
				{/if}
			</div>
		</div>
	{:else}
		{@render fileInput()}
	{/if}
	{#if initialS3 && initialS3.length > 0 && $fileUploads.length == 0}
		<div class="flex flex-row gap-1 items-center p-1">
			<span class="text-sm">
				File{initialS3.length > 1 ? 's' : ''} currently selected: {initialS3?.join(', ')}
			</span>
		</div>
	{/if}
</div>
