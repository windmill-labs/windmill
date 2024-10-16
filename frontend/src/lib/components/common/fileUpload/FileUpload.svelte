<script lang="ts">
	import { FileInput } from '../'
	import FileProgressBar from '../FileProgressBar.svelte'

	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService } from '$lib/gen'
	import { writable, type Writable } from 'svelte/store'
	import { Ban, CheckCheck, FileWarning, Files, RefreshCcw, Trash } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { emptyString } from '$lib/utils'

	export let acceptedFileTypes: string[] | undefined = ['*']
	export let allowMultiple: boolean = true
	export let containerText: string = 'Drag and drop files here or click to browse'
	export let customResourcePath: string | undefined = undefined
	export let customResourceType: 's3' | 'azure_blob' | undefined = undefined // when customResourcePath is provided, this should be provided as well. Will default to S3 if not
	export let customClass: string = ''
	export let customStyle: string = ''
	export let randomFileKey: boolean = false
	export let pathTransformer: any = undefined // function taking as input {file: File} and returning a string
	export let forceDisplayUploads: boolean = false
	export let defaultValue: string | undefined = undefined
	export let workspace: string | undefined = undefined
	export let fileUploads: Writable<FileUploadData[]> = writable([])

	const dispatch = createEventDispatcher()

	type FileUploadData = {
		name: string
		size: number
		progress: number
		cancelled?: boolean
		errorMessage?: string
		path?: string
		file?: File
	}

	async function handleChange(files: File[] | undefined) {
		for (const file of files ?? []) {
			uploadFileToS3(file, file.name)
		}
	}

	let activeUploads: { xhr: XMLHttpRequest; fileName: string }[] = []

	async function uploadFileToS3(fileToUpload: File, fileToUploadKey: string) {
		if (fileToUpload === undefined || fileToUploadKey === undefined) {
			return
		}
		let path: string | undefined = undefined
		let fileExtension: string | undefined = undefined
		if (randomFileKey) {
			fileExtension = fileToUpload.name.split('.').pop()
			if (emptyString(fileExtension)) {
				fileExtension = undefined
			}
		} else {
			path =
				typeof pathTransformer == 'function'
					? (await pathTransformer?.({
							file: fileToUpload
					  })) ?? fileToUploadKey
					: fileToUploadKey
		}
		const uploadData: FileUploadData = {
			name: fileToUpload.name,
			size: fileToUpload.size,
			progress: 1, // We set it to 1 so that the progress bar is visible
			cancelled: false,
			path: path,
			file: fileToUpload
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
					`/api/w/${workspace ?? $workspaceStore}/job_helpers/upload_s3_file?${params.toString()}`,
					true
				)
				xhr?.setRequestHeader('Content-Type', 'application/octet-stream')
				xhr?.send(fileToUpload)
			})) as any

			uploadData.path = response.file_key
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
		dispatch('addition', { path: uploadData.path, filename: fileToUpload.name })
		sendUserToast('File upload finished!')

		uploadData.progress = 100
		$fileUploads = $fileUploads.map((fileUpload) => {
			if (fileUpload.name === uploadData.name) {
				return uploadData
			}
			return fileUpload
		})
		return
	}

	async function deleteFile(fileKey: string) {
		await HelpersService.deleteS3File({
			workspace: workspace ?? $workspaceStore!,
			fileKey: fileKey
		})
		dispatch('deletion', { path: fileKey })
		sendUserToast('File deleted!')
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

<div class="w-full h-full p-0 flex flex-col">
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
											abortUpload(fileUpload.name)
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
								{fileUpload.progress === 100 ? 'Upload finished' : `Uploading`} to path: {fileUpload.path ??
									'N/A'}
							</span>
						{/if}
					</div>
				{/each}
			</div>
			{#if allowMultiple}
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
			{/if}
		</div>
	{:else}
		<FileInput
			accept={acceptedFileTypes?.join(',')}
			multiple={allowMultiple}
			returnFileNames
			includeMimeType
			on:change={({ detail }) => {
				forceDisplayUploads = false
				handleChange(detail)
			}}
			class={twMerge('w-full h-full', customClass, 'wm-file-input')}
			style={customStyle}
			defaultFile={defaultValue}
		>
			{containerText}
		</FileInput>
	{/if}
</div>
