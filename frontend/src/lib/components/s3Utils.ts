import { HelpersService, type UploadFilePart } from '$lib/gen'
import { sendUserToast } from '$lib/utils'

type FileType = {
	type: 'folder' | 'leaf'
	full_key: string
	display_name: string
	collapsed: boolean
	parentPath: string | undefined
	nestingLevel: number
}

export async function uploadFileToS3(workspace: string) {
	let fileToUpload: File | undefined
	let fileToUploadKey: string | undefined
	let fileUploadCancelled: boolean = false
	let fileUploadProgress: number = 0

	let allFilesByKey: Record<string, FileType> = {}

	if (!fileToUpload || !fileToUploadKey) {
		return
	}

	if (allFilesByKey[fileToUploadKey]) {
		throw new Error(
			'A file with this name already exists in the S3 bucket. Delete it first to replace.'
		)
	}

	let uploadId: string | undefined
	let parts: UploadFilePart[] = []
	let reader = fileToUpload.stream().getReader()

	try {
		let chunk: Uint8Array | null = null
		let totalSize = fileToUpload.size
		let uploadedSize = 0

		while (true) {
			const { value, done } = await reader.read()
			if (done) {
				if (chunk) {
					await uploadChunk(chunk, true) // Final chunk upload if any data is left
				}
				break
			}

			chunk = chunk ? new Uint8Array([...chunk, ...value]) : new Uint8Array(value)
			if (chunk.length > 5 * 1024 * 1024) {
				await uploadChunk(chunk)
				chunk = null
			}

			uploadedSize += value.length
			fileUploadProgress = (uploadedSize * 100) / totalSize
		}
	} catch (error) {
		sendUserToast(`Error during file upload: ${error}`, true)
	}

	async function uploadChunk(chunk: Uint8Array, isFinal: boolean = false) {
		if (!fileToUploadKey) {
			throw new Error('File key is undefined')
		}

		const response = await HelpersService.multipartFileUpload({
			workspace,
			requestBody: {
				file_key: fileToUploadKey,
				part_content: Array.from(chunk),
				upload_id: uploadId,
				parts: parts,
				is_final: isFinal,
				cancel_upload: fileUploadCancelled
			}
		})

		uploadId = response.upload_id
		parts = response.parts

		if (response.is_done) {
			sendUserToast(fileUploadCancelled ? 'File upload cancelled!' : 'File upload finished!')
		}
	}
}
