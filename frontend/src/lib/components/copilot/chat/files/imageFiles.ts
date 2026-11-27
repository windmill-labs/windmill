/**
 * Which linked files the session's image viewer can show. Decided by extension: a
 * folder's files come off disk through the File System Access API, which leaves
 * `File.type` empty for many of them, and a snapshot Blob restored from IndexedDB
 * keeps whatever type it had at link time.
 */

/** Binary image formats: linked as image rows, never line-indexed or read as text. */
const RASTER_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'bmp', 'ico'])

function extensionOf(name: string): string {
	const dot = name.lastIndexOf('.')
	return dot >= 0 ? name.slice(dot + 1).toLowerCase() : ''
}

/** A binary image the store keeps as an image row instead of rejecting it as non-text. */
export function isRasterImageName(name: string): boolean {
	return RASTER_EXTENSIONS.has(extensionOf(name))
}

/** Whether the image viewer can show the file. SVG is text, so it stays readable by the
 * file tools as well. */
export function isViewableImageName(name: string): boolean {
	return isRasterImageName(name) || extensionOf(name) === 'svg'
}

const MIME_BY_EXTENSION: Record<string, string> = {
	png: 'image/png',
	jpg: 'image/jpeg',
	jpeg: 'image/jpeg',
	gif: 'image/gif',
	webp: 'image/webp',
	avif: 'image/avif',
	bmp: 'image/bmp',
	ico: 'image/x-icon',
	svg: 'image/svg+xml'
}

/** The bytes as a Blob the browser decodes as an image. An SVG served without
 * `image/svg+xml` does not render in an `<img>`, and a folder file often has no type. */
export function asImageBlob(file: Blob, name: string): Blob {
	const type = MIME_BY_EXTENSION[extensionOf(name)]
	if (!type || file.type === type) return file
	return new Blob([file], { type })
}

/**
 * Stable short key for an image sent in a chat message, which has no id of its own:
 * FNV-1a over the data URL, plus its length. A tab persists this key and finds the
 * image again by recomputing it over the transcript's images.
 */
export function imageContentKey(dataUrl: string, owner?: object): string {
	// An image tab re-resolves on every transcript change, so it passes the message or
	// image object as `owner`: the hash is then computed once per image and released with it.
	if (!owner) return fnv1a(dataUrl)
	const hit = keyByOwner.get(owner)
	if (hit?.dataUrl === dataUrl) return hit.key
	const key = fnv1a(dataUrl)
	keyByOwner.set(owner, { dataUrl, key })
	return key
}

const keyByOwner = new WeakMap<object, { dataUrl: string; key: string }>()

function fnv1a(dataUrl: string): string {
	let h = 0x811c9dc5
	for (let i = 0; i < dataUrl.length; i++) {
		h ^= dataUrl.charCodeAt(i)
		h = Math.imul(h, 0x01000193)
	}
	return `${(h >>> 0).toString(16)}-${dataUrl.length.toString(36)}`
}
