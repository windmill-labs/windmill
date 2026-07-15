/**
 * Image handling shared by the two multimodal chat features: user-attached images
 * (drag/drop/paste, GLOBAL mode) and the app agent's `take_screenshot` tool.
 *
 * Every image the model sees passes through here first so it is bounded in BOTH
 * dimensions (≤ MAX_EDGE longest side — beyond this the provider downscales anyway
 * and just bills more tokens) and bytes. Bounding bytes matters because the data URL
 * is persisted in chat history (IndexedDB, re-snapshotted on every saveChat) and
 * counts toward the context window. Everything is rasterised to PNG/JPEG so exotic
 * inputs (SVG, WebP, HEIC where the browser can decode it) become a media type all
 * providers accept.
 */
import type {
	ChatCompletionContentPartImage,
	ChatCompletionMessageParam
} from 'openai/resources/index.mjs'

/** Longest-edge cap. Matches the point past which vision models downscale server-side. */
export const MAX_IMAGE_EDGE = 1568
/**
 * Longest edge for the copy kept in `displayMessages`. Those are never compacted and
 * are re-cloned into IndexedDB on every saveChat, so the transcript must not hold a
 * model-resolution blob — the tool card only ever renders it ~192px tall.
 */
export const THUMBNAIL_IMAGE_EDGE = 384
/** Above this many bytes a PNG re-encodes to JPEG to keep history/storage bounded. */
const PNG_SIZE_CAP = 700_000
/**
 * Refuse a file this large before reading it. Decoding allocates ~4 bytes per pixel
 * — a 12MP photo is ~48MB of bitmap — and the downscale below can only run once that
 * bitmap exists, so the cap has to bite before the read, not after.
 */
export const MAX_IMAGE_BYTES = 20_000_000
/** Decoded-pixel ceiling, in case a small file expands to an absurd bitmap. */
const MAX_IMAGE_PIXELS = 40_000_000
/**
 * Images one message may carry. Enforced wherever a message is assembled, not just
 * at the composer: queuing clears the composer, so its own count would reset and let
 * repeated sends stack an unbounded batch into a single message.
 */
export const MAX_ATTACHED_IMAGES = 8

export type ImageMediaType = 'image/png' | 'image/jpeg'

/** A model-ready image: a normalised (bounded, png/jpeg) data URL plus its media type. */
export type AttachedImage = {
	dataUrl: string
	mediaType: ImageMediaType
	/** Original filename when it came from a user file; absent for screenshots. */
	name?: string
	/**
	 * Smaller copy for the chips and the message bubble. `displayMessages` are never
	 * compacted and are re-cloned into IndexedDB on every save, so the transcript
	 * must not keep the model's copy. Absent when a downscale would not be smaller.
	 */
	previewUrl?: string
}

/** The copy to show and persist in the transcript: bounded when that helps. */
export function transcriptImage(image: AttachedImage): AttachedImage {
	if (!image.previewUrl) return image
	return { dataUrl: image.previewUrl, mediaType: image.mediaType, name: image.name }
}

/**
 * Recover the model's own images from an API message's content parts. Resending a
 * turn must read them from here, never from the transcript: that copy is a bounded
 * thumbnail, and re-sending it would silently downgrade the model's input.
 */
export function imagesFromContent(content: unknown): AttachedImage[] | undefined {
	if (!Array.isArray(content)) return undefined
	const images: AttachedImage[] = (content as any[])
		.filter((part) => part?.type === 'image_url' && typeof part?.image_url?.url === 'string')
		.map((part) => {
			const dataUrl = part.image_url.url as string
			return {
				dataUrl,
				mediaType:
					parseImageDataUrl(dataUrl).mediaType === 'image/jpeg' ? 'image/jpeg' : 'image/png'
			}
		})
	return images.length > 0 ? images : undefined
}

export function isImageFile(file: File | Blob): boolean {
	return typeof file.type === 'string' && file.type.startsWith('image/')
}

/** Byte size of a base64 data URL's payload (4 base64 chars → 3 bytes). */
function base64Bytes(dataUrl: string): number {
	const comma = dataUrl.indexOf(',')
	const b64 = comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl
	const padding = b64.endsWith('==') ? 2 : b64.endsWith('=') ? 1 : 0
	return Math.max(0, Math.floor((b64.length * 3) / 4) - padding)
}

function loadImage(src: string): Promise<HTMLImageElement> {
	return new Promise((resolve, reject) => {
		const img = new Image()
		img.onload = () => resolve(img)
		img.onerror = () => reject(new Error('Could not decode image'))
		img.src = src
	})
}

function blobToDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader()
		reader.onload = () => resolve(reader.result as string)
		reader.onerror = () => reject(reader.error ?? new Error('Could not read file'))
		reader.readAsDataURL(blob)
	})
}

/** PNG by default (lossless — crisp for the common UI-screenshot/diagram case); fall
 *  back to JPEG only when the PNG would blow the size cap (photographic content). */
function encodeCanvas(canvas: HTMLCanvasElement): { dataUrl: string; mediaType: ImageMediaType } {
	const png = canvas.toDataURL('image/png')
	if (base64Bytes(png) <= PNG_SIZE_CAP) {
		return { dataUrl: png, mediaType: 'image/png' }
	}
	return { dataUrl: canvas.toDataURL('image/jpeg', 0.82), mediaType: 'image/jpeg' }
}

/**
 * Downscale a data URL to ≤ MAX_IMAGE_EDGE on its longest side and re-encode to
 * png/jpeg. Used by both the file-attach path and the screenshot tool.
 */
export async function normalizeImageDataUrl(
	dataUrl: string,
	name?: string,
	maxEdge: number = MAX_IMAGE_EDGE
): Promise<AttachedImage> {
	const img = await loadImage(dataUrl)
	const srcW = img.naturalWidth || img.width
	const srcH = img.naturalHeight || img.height
	if (!srcW || !srcH) throw new Error('Image has no dimensions')
	if (srcW * srcH > MAX_IMAGE_PIXELS) throw new Error('Image resolution is too large')
	const scale = Math.min(1, maxEdge / Math.max(srcW, srcH))
	const w = Math.max(1, Math.round(srcW * scale))
	const h = Math.max(1, Math.round(srcH * scale))
	const canvas = document.createElement('canvas')
	canvas.width = w
	canvas.height = h
	const ctx = canvas.getContext('2d')
	if (!ctx) throw new Error('Canvas 2D context unavailable')
	ctx.drawImage(img, 0, 0, w, h)
	return { ...encodeCanvas(canvas), name }
}

/** Read a user-provided image file and produce a bounded, model-ready AttachedImage. */
export async function fileToAttachedImage(file: File | Blob): Promise<AttachedImage> {
	if (file.size > MAX_IMAGE_BYTES) throw new Error('Image file is too large')
	const name = file instanceof File ? file.name : undefined
	const dataUrl = await blobToDataUrl(file)
	const image = await normalizeImageDataUrl(dataUrl, name)
	// Derive the preview from the already-bounded copy, not the source: re-decoding
	// the original would allocate its full bitmap (~4 bytes/pixel) a second time.
	// Built here rather than at send time because this path already awaits behind a
	// spinner, whereas the send path shows its bubble optimistically.
	const preview = await normalizeImageDataUrl(image.dataUrl, name, THUMBNAIL_IMAGE_EDGE)
	// A downscale of flat UI colours can encode larger than the original, so only
	// keep it when it actually saves something.
	return preview.dataUrl.length < image.dataUrl.length
		? { ...image, previewUrl: preview.dataUrl }
		: image
}

/** Split a data URL into its media type and base64 payload (for the Anthropic converter). */
export function parseImageDataUrl(url: string): { mediaType: string; base64: string } {
	const match = /^data:([^;,]+)?(;base64)?,(.*)$/s.exec(url)
	if (!match) return { mediaType: 'image/png', base64: '' }
	return { mediaType: match[1] || 'image/png', base64: match[2] ? match[3] : '' }
}

/** Build the OpenAI-format image content part that all three provider paths convert from. */
export function dataUrlToImagePart(dataUrl: string): ChatCompletionContentPartImage {
	return { type: 'image_url', image_url: { url: dataUrl } }
}

/**
 * Replace image_url content parts with a short text placeholder, collapsing the
 * remaining parts back to a plain string. Used to keep base64 blobs out of the
 * summarizer request during compaction (the summary text then stands in for them).
 */
export function stripImagePartsFromMessages(
	messages: ChatCompletionMessageParam[]
): ChatCompletionMessageParam[] {
	return messages.map((message) => {
		if (!Array.isArray(message.content)) return message
		let hadImage = false
		const text = (message.content as any[])
			.map((part) => {
				if (part?.type === 'text') return part.text ?? ''
				if (part?.type === 'image_url') {
					hadImage = true
					return '[image omitted]'
				}
				return ''
			})
			.filter(Boolean)
			.join('\n')
		if (!hadImage) return message
		return { ...message, content: text } as ChatCompletionMessageParam
	})
}
