/**
 * What a chat attachment is stored under in object storage.
 *
 * The worker reads an attachment's media type from the object key and nothing else —
 * `mime_guess::from_path` in `windmill-ai/src/image_handler.rs`, falling back to
 * `image/png` when it can read no extension — and never from the content type stored
 * beside it. So the key's extension is a claim about the bytes, and it has to be true.
 */

/** The extension each type the composer can send must be stored under. */
const EXTENSION_BY_MEDIA_TYPE: Record<string, string> = {
	'image/png': 'png',
	'image/jpeg': 'jpg',
	'application/pdf': 'pdf'
}

/**
 * The composer re-encodes every image to PNG or JPEG, so keeping the picked `photo.webp`
 * would hand the provider PNG bytes labelled webp, which Anthropic rejects outright; and a
 * PDF picked without an extension would be read back as the `image/png` fallback. A type
 * not listed is left as picked — blobs upload byte for byte, so their name is already true.
 */
export function storedAttachmentName(filename: string, mediaType: string): string {
	const extension = EXTENSION_BY_MEDIA_TYPE[mediaType]
	if (!extension) return filename
	const stem = filename.replace(/\.[^./]+$/, '')
	return `${stem || filename}.${extension}`
}
