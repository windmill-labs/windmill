/**
 * Message attachments kept as their original bytes, such as a PDF, for a host that
 * forwards them to object storage. Images are re-encoded and text files decoded instead,
 * so neither lane can carry what the user picked unchanged.
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
