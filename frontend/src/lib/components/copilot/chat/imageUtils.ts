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
/** Above this many bytes a PNG re-encodes to JPEG to keep history/storage bounded. */
const PNG_SIZE_CAP = 700_000

export type ImageMediaType = 'image/png' | 'image/jpeg'

/** A model-ready image: a normalised (bounded, png/jpeg) data URL plus its media type. */
export type AttachedImage = {
	dataUrl: string
	mediaType: ImageMediaType
	/** Original filename when it came from a user file; absent for screenshots. */
	name?: string
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
	name?: string
): Promise<AttachedImage> {
	const img = await loadImage(dataUrl)
	const srcW = img.naturalWidth || img.width
	const srcH = img.naturalHeight || img.height
	if (!srcW || !srcH) throw new Error('Image has no dimensions')
	const scale = Math.min(1, MAX_IMAGE_EDGE / Math.max(srcW, srcH))
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
	const name = file instanceof File ? file.name : undefined
	return normalizeImageDataUrl(await blobToDataUrl(file), name)
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
