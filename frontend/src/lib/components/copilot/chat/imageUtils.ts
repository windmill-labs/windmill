/**
 * Image handling shared by the two multimodal chat features: user-attached images
 * (drag/drop/paste, GLOBAL mode) and the app agent's `take_screenshot` tool.
 *
 * Every image the model sees passes through here first so it is bounded in BOTH
 * dimensions (≤ MAX_EDGE longest side — beyond this the provider downscales anyway
 * and just bills more tokens) and bytes. Bounding bytes matters because the data URL
 * rides every request (stateless APIs resend the whole history) and is persisted in
 * the chat history's blob store. Everything is rasterised to PNG/JPEG so exotic
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
}

/** Stands in for a stripped or evicted image part in message content. */
export const IMAGE_OMITTED_PLACEHOLDER = '[image omitted]'

/**
 * Recover the model's own images from an API message's content parts. Anything
 * resending a turn (retry, edit) must read images from here, never from the
 * transcript bubble: a provider rejection strips them from history while the
 * bubble keeps its copy so the user can still see what they sent — resending
 * that copy would re-attach the image the provider just refused.
 */
export function imagesFromContent(content: unknown): AttachedImage[] | undefined {
	if (!Array.isArray(content)) return undefined
	const images = (content as any[]).flatMap((part): AttachedImage[] => {
		if (part?.type !== 'image_url' || typeof part?.image_url?.url !== 'string') return []
		const dataUrl = part.image_url.url as string
		return [
			{
				dataUrl,
				mediaType:
					parseImageDataUrl(dataUrl).mediaType === 'image/jpeg' ? 'image/jpeg' : 'image/png'
			}
		]
	})
	return images.length > 0 ? images : undefined
}

/**
 * Raster scale for a DOM screenshot of a target whose longest CSS edge is
 * `cssEdge`. Above CSS resolution (up to 2×) for small targets — the SVG
 * re-render is vector, so the extra scale is real detail, not interpolation —
 * but never a raster larger than MAX_IMAGE_EDGE: normalize would downscale the
 * excess away, and rasterising an oversized body (a tall scrolling app) at ≥1×
 * first can allocate a tab-freezing canvas. Sub-1× output is deliberate.
 */
export function captureScale(cssEdge: number): number {
	return Math.min(2, MAX_IMAGE_EDGE / Math.max(1, cssEdge))
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
	// JPEG has no alpha channel and canvas encoders composite transparent pixels
	// onto black, which hides dark strokes in a transparent diagram. Flatten onto
	// white before encoding.
	const flat = document.createElement('canvas')
	flat.width = canvas.width
	flat.height = canvas.height
	const ctx = flat.getContext('2d')
	if (ctx) {
		ctx.fillStyle = '#ffffff'
		ctx.fillRect(0, 0, flat.width, flat.height)
		ctx.drawImage(canvas, 0, 0)
	}
	return {
		dataUrl: (ctx ? flat : canvas).toDataURL('image/jpeg', 0.82),
		mediaType: 'image/jpeg'
	}
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
	return await normalizeImageDataUrl(dataUrl, name)
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

/** Whether any message still carries an image_url content part. */
export function messagesHaveImageParts(messages: ChatCompletionMessageParam[]): boolean {
	return messages.some(
		(message) =>
			Array.isArray(message.content) &&
			(message.content as any[]).some((part) => part?.type === 'image_url')
	)
}

/**
 * Total decoded image bytes one request may carry. Providers reject the whole
 * request body over a size limit (20MB on Bedrock, 32MB direct Anthropic), and
 * that 413 never mentions images, so the vision-rejection fallback cannot
 * recover it — each request must stay under the limit in the first place.
 * Compaction cannot be relied on for this: it triggers on estimated tokens,
 * and images are cheap in tokens relative to their bytes. 12MB decoded is
 * ~16MB of base64 on the wire, safely under the tightest limit with text.
 */
export const MAX_TOTAL_IMAGE_BYTES = 12_000_000

/**
 * Keep the request's cumulative image bytes under the cap by stripping the
 * OLDEST image parts first (the newest images are the ones the conversation
 * is about). Part-granular so a single over-cap batch keeps the subset that
 * fits — the newest message never silently loses all its images (one bounded
 * image alone cannot exceed the cap). Returns the input array unchanged when
 * everything fits.
 */
export function boundImagePartBytes(
	messages: ChatCompletionMessageParam[],
	cap: number = MAX_TOTAL_IMAGE_BYTES
): ChatCompletionMessageParam[] {
	let total = 0
	const drops = new Map<number, Set<number>>()
	for (let i = messages.length - 1; i >= 0; i--) {
		const content = messages[i].content
		if (!Array.isArray(content)) continue
		// Parts walk in reverse too: within a message they are in attachment order,
		// and for screenshot follow-ups the last one is the app's current state.
		for (let j = (content as any[]).length - 1; j >= 0; j--) {
			const part = (content as any[])[j]
			if (part?.type !== 'image_url' || typeof part?.image_url?.url !== 'string') continue
			total += base64Bytes(part.image_url.url)
			if (total > cap) {
				if (!drops.has(i)) drops.set(i, new Set())
				drops.get(i)!.add(j)
			}
		}
	}
	if (drops.size === 0) return messages
	return messages.map((message, i) => {
		const drop = drops.get(i)
		if (!drop) return message
		return {
			...message,
			content: (message.content as any[]).map((part, j) =>
				drop.has(j) ? { type: 'text', text: IMAGE_OMITTED_PLACEHOLDER } : part
			)
		} as ChatCompletionMessageParam
	})
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
					return IMAGE_OMITTED_PLACEHOLDER
				}
				return ''
			})
			.filter(Boolean)
			.join('\n')
		if (!hadImage) return message
		return { ...message, content: text } as ChatCompletionMessageParam
	})
}
