<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { CheckCircle, Loader2, Upload, XCircle } from 'lucide-svelte'
	import Button from '../button/Button.svelte'
	import { HelpersService, type UploadFilePart } from '$lib/gen'
	import { emptyString, sendUserToast } from '$lib/utils'

	export let uploadedFile:
		| {
				s3: string
		  }
		| undefined = undefined

	let localFileToUpload: File | undefined = undefined
	let fileUploadProgress: number | undefined = undefined
	let fileUploadCancelled: boolean = false
	let fileUploadCancelInProgress: boolean = false
	let fileUploadErrorMsg: string | undefined = undefined

	async function uploadFileToS3() {
		fileUploadErrorMsg = undefined
		if (localFileToUpload === undefined) {
			return
		}

		uploadedFile = undefined
		let uploadId: string | undefined = undefined
		let parts: UploadFilePart[] = []

		let reader = localFileToUpload.stream().getReader()
		let { value: chunk, done: readerDone } = await reader.read()
		if (chunk === undefined || readerDone) {
			sendUserToast('Error reading file, no data read', true)
			return
		}
		let fileExtension: string | undefined = localFileToUpload.name.split('.').pop()
		if (emptyString(fileExtension)) {
			fileExtension = undefined
		}
		let s3FileKey: string | undefined = undefined
		let fileUploadProgressExact = 0
		fileUploadProgress = 0
		while (true) {
			let { value: chunk_2, done: readerDone } = await reader.read()
			if (!readerDone && chunk_2 !== undefined && chunk.length <= 5 * 1024 * 1024) {
				// AWS enforces part to be bigger than 5MB, so we accumulate bytes until we reach that limit before triggering the request to the BE
				chunk = new Uint8Array([...chunk, ...chunk_2])
				continue
			}
			fileUploadProgressExact += (chunk.length * 100) / localFileToUpload.size
			let response = await HelpersService.multipartFileUpload({
				workspace: $workspaceStore!,
				requestBody: {
					file_key: s3FileKey,
					file_extension: fileExtension,
					part_content: Array.from(chunk),
					upload_id: uploadId,
					parts: parts,
					is_final: readerDone,
					cancel_upload: fileUploadCancelled
				}
			})
			s3FileKey = response.file_key
			uploadId = response.upload_id
			parts = response.parts
			if (response.is_done) {
				if (fileUploadCancelled) {
					sendUserToast('File upload cancelled!')
				} else {
					console.log('File upload finished: ', s3FileKey)
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
			fileUploadProgress = fileUploadProgressExact | 0
		}

		if (!fileUploadCancelled) {
			uploadedFile = { s3: s3FileKey }
		}

		fileUploadProgress = undefined
		fileUploadCancelled = false
		fileUploadCancelInProgress = false
		fileUploadErrorMsg = undefined
	}
</script>

<div class="flex flex-col gap-1">
	<input
		type="file"
		title={localFileToUpload ? `${localFileToUpload.name}` : 'No file chosen '}
		on:change={({ currentTarget }) => {
			if (
				currentTarget.files === undefined ||
				currentTarget.files === null ||
				currentTarget.files.length === 0
			) {
				localFileToUpload = undefined
			} else {
				localFileToUpload = currentTarget.files[0]
			}
			uploadedFile = undefined
		}}
		accept="*"
		multiple={false}
	/>
	<div class="flex w-full gap-1 justify-evenly">
		<div class="grow">
			<Button
				disabled={localFileToUpload === undefined}
				variant="border"
				color="light"
				size="xs"
				on:click={() => {
					console.log('Starting file upload: ', localFileToUpload?.name)
					uploadFileToS3()
				}}
				startIcon={fileUploadProgress === undefined
					? uploadedFile === undefined || uploadedFile === null
						? { icon: Upload }
						: { icon: CheckCircle }
					: { icon: Loader2, classes: 'animate-spin' }}
			>
				{`Upload${fileUploadProgress !== undefined ? ` - ${fileUploadProgress}%` : ''}`}
			</Button>
		</div>
		<div class="grow">
			<Button
				disabled={fileUploadProgress === undefined}
				variant="border"
				color="red"
				size="xs"
				on:click={() => {
					console.log('Cancelling file upload for: ', localFileToUpload?.name)
					fileUploadCancelInProgress = true
					fileUploadCancelled = true
				}}
				startIcon={fileUploadCancelInProgress
					? { icon: Loader2, classes: 'animate-spin' }
					: { icon: XCircle }}
			>
				Cancel
			</Button>
		</div>
	</div>
</div>
