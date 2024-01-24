<script lang="ts">
	import { FileInput } from '../'
	import FileProgressBar from '../FileProgressBar.svelte'

	import Button from '$lib/components/common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import { HelpersService, type UploadFilePart } from '$lib/gen'
	import { writable, type Writable } from 'svelte/store'
	import { Ban, CheckCheck, FileWarning, Files, RefreshCcw, Trash } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createEventDispatcher } from 'svelte'
	import { emptyString } from '$lib/utils'

	export let acceptedFileTypes: string[] | undefined = ['*']
	export let allowMultiple: boolean = true
	export let containerText: string = 'Drag and drop files here or click to browse'
	export let customS3ResourcePath: string | undefined = undefined
	export let customClass: string = ''
	export let customStyle: string = ''
	export let randomFileKey: boolean = false
	export let pathTransformer: any = undefined // function taking as input {file: File} and returning a string
	export let forceDisplayUploads: boolean = false

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

	let fileUploads: Writable<FileUploadData[]> = writable([])

	async function handleChange(files: File[] | undefined) {
		for (const file of files ?? []) {
			uploadFileToS3(file, file.name)
		}
	}

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

			try {
				let response = await HelpersService.multipartFileUpload({
					workspace: $workspaceStore!,
					requestBody: {
						file_key: path,
						file_extension: fileExtension,
						part_content: Array.from(chunk),
						upload_id: upload_id,
						parts: parts,
						is_final: readerDone,
						cancel_upload: currentFileUpload.cancelled ?? false,
						s3_resource_path:
							customS3ResourcePath !== undefined ? customS3ResourcePath.split(':')[1] : undefined
					}
				})
				uploadData.path = response.file_key
				path = response.file_key
				upload_id = response.upload_id
				parts = response.parts

				// update upload progress
				fileUploadProgress += (chunk.length * 100) / fileToUpload.size
				uploadData.progress = fileUploadProgress
				$fileUploads = $fileUploads.map((fileUpload) => {
					if (fileUpload.name === uploadData.name) {
						return uploadData
					}
					return fileUpload
				})

				if (response.is_done) {
					if (currentFileUpload.cancelled) {
						sendUserToast('File upload cancelled!')
					} else {
						dispatch('addition', { path: path })
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

	async function deleteFile(fileKey: string) {
		await HelpersService.deleteS3File({
			workspace: $workspaceStore!,
			fileKey: fileKey
		})
		dispatch('deletion', { path: fileKey })
		sendUserToast('File deleted!')
	}
</script>

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
		>
			{containerText}
		</FileInput>
	{/if}
</div>
