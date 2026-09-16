/**
 * Message-scoped attachments that are neither an image nor readable text — a PDF
 * being the case that matters. They ride the composer next to images and files,
 * as chips cleared on send, and reach the host through `ChatSendRequestOptions`.
 *
 * The bytes are kept verbatim, unlike an image (which normalises to a bounded
 * PNG/JPEG for the model) and unlike a text file (which is decoded to a string):
 * a host that forwards these to object storage has to upload what the user
 * picked, not a re-encoding of it.
 */

/** Blobs one message may carry — the same slot cap images and text files use. */
export const MAX_ATTACHED_BLOBS = 8

/**
 * Per-blob byte cap. The data URL sits in composer state until send, so this
 * bounds what one message can hold in memory; a host uploading elsewhere pays
 * the same bytes again on the wire.
 */
export const MAX_BLOB_BYTES = 20_000_000

export type AttachedBlob = {
	name: string
	/** The file's own media type, verbatim — the upload's Content-Type depends on it. */
	mediaType: string
	/** `data:<mediaType>;base64,<...>` of the original bytes. */
	dataUrl: string
	size: number
}

/**
 * Whether a file satisfies an `accept` list — the same list the OS picker gets, applied
 * again on drop, where the browser enforces nothing.
 */
export function matchesAccept(file: File, accept: string): boolean {
	const patterns = accept
		.split(',')
		.map((p) => p.trim().toLowerCase())
		.filter(Boolean)
	if (patterns.length === 0) return true
	const type = file.type.toLowerCase()
	const name = file.name.toLowerCase()
	return patterns.some((pattern) => {
		if (pattern.startsWith('.')) return name.endsWith(pattern)
		if (pattern.endsWith('/*')) return type.startsWith(pattern.slice(0, -1))
		return type === pattern
	})
}

export async function fileToAttachedBlob(file: File): Promise<AttachedBlob> {
	const dataUrl = await new Promise<string>((resolve, reject) => {
		const reader = new FileReader()
		reader.onload = () => resolve(String(reader.result))
		reader.onerror = () => reject(reader.error ?? new Error(`Could not read ${file.name}`))
		reader.readAsDataURL(file)
	})
	return {
		name: file.name,
		mediaType: file.type || 'application/octet-stream',
		dataUrl,
		size: file.size
	}
}

/** The bytes behind a `data:` URL, for a host that has to re-upload them. */
export function dataUrlToBlob(dataUrl: string, fallbackType = 'application/octet-stream'): Blob {
	const comma = dataUrl.indexOf(',')
	const header = dataUrl.slice(5, comma)
	const isBase64 = header.endsWith(';base64')
	const mediaType = (isBase64 ? header.slice(0, -';base64'.length) : header) || fallbackType
	const payload = dataUrl.slice(comma + 1)
	if (!isBase64) {
		return new Blob([decodeURIComponent(payload)], { type: mediaType })
	}
	const binary = atob(payload)
	const bytes = new Uint8Array(binary.length)
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
	return new Blob([bytes], { type: mediaType })
}
