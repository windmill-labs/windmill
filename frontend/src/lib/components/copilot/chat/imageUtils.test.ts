import { describe, expect, it } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import {
	boundImagePartBytes,
	captureScale,
	fileToAttachedImage,
	imagesFromContent,
	MAX_IMAGE_BYTES,
	messagesHaveImageParts,
	parseImageDataUrl,
	stripImagePartsFromMessages
} from './imageUtils'

describe('fileToAttachedImage size bound', () => {
	// Decoding allocates ~4 bytes per pixel before the downscale can run, so an
	// oversized file must be refused up front.
	it('rejects a file over the byte cap', async () => {
		const blob = { size: MAX_IMAGE_BYTES + 1, type: 'image/png' } as unknown as Blob
		await expect(fileToAttachedImage(blob)).rejects.toThrow(/too large/i)
	})
})

describe('parseImageDataUrl', () => {
	it('splits media type and base64 payload', () => {
		expect(parseImageDataUrl('data:image/png;base64,AAAA')).toEqual({
			mediaType: 'image/png',
			base64: 'AAAA'
		})
		expect(parseImageDataUrl('data:image/jpeg;base64,ZZ==')).toEqual({
			mediaType: 'image/jpeg',
			base64: 'ZZ=='
		})
	})

	it('defaults to png and empty payload on a malformed url', () => {
		expect(parseImageDataUrl('not-a-data-url')).toEqual({ mediaType: 'image/png', base64: '' })
	})
})

describe('stripImagePartsFromMessages', () => {
	it('replaces image parts with a placeholder and collapses to a string', () => {
		const messages: ChatCompletionMessageParam[] = [
			{
				role: 'user',
				content: [
					{ type: 'text', text: 'look at this' },
					{ type: 'image_url', image_url: { url: 'data:image/png;base64,HUGEBLOB' } }
				]
			} as any
		]
		const out = stripImagePartsFromMessages(messages)
		expect(out[0].content).toBe('look at this\n[image omitted]')
	})

	it('leaves image-free messages untouched (same reference)', () => {
		const messages: ChatCompletionMessageParam[] = [{ role: 'user', content: 'plain' }]
		const out = stripImagePartsFromMessages(messages)
		expect(out[0]).toBe(messages[0])
	})
})

describe('boundImagePartBytes', () => {
	const imgMsg = (payloadChars: number, text: string): ChatCompletionMessageParam =>
		({
			role: 'user',
			content: [
				{ type: 'text', text },
				{
					type: 'image_url',
					image_url: { url: 'data:image/png;base64,' + 'A'.repeat(payloadChars) }
				}
			]
		}) as any

	it('returns the same array when everything fits', () => {
		const messages = [imgMsg(100, 'a')]
		expect(boundImagePartBytes(messages, 1000)).toBe(messages)
	})

	const imageParts = (m: ChatCompletionMessageParam) =>
		(m.content as any[]).filter((p) => p?.type === 'image_url').length

	it('strips the oldest images first once the cap is exceeded', () => {
		// 1000 base64 chars ≈ 750 bytes each: the newest fits alone, both together don't
		const messages = [
			imgMsg(1000, 'old'),
			{ role: 'assistant', content: 'ok' } as ChatCompletionMessageParam,
			imgMsg(1000, 'new')
		]
		const out = boundImagePartBytes(messages, 1000)
		expect(imageParts(out[0])).toBe(0)
		expect(JSON.stringify(out[0].content)).toContain('[image omitted]')
		expect(out[1]).toBe(messages[1])
		expect(imageParts(out[2])).toBe(1)
	})

	// An over-cap batch on the CURRENT turn must keep the subset that fits, not
	// silently send a text-only message while the composer showed attached images.
	// The newest parts win: for screenshot follow-ups the last image is the app's
	// current state.
	it('keeps the newest fitting subset when the newest message alone exceeds the cap', () => {
		const url = (marker: string) => ({
			type: 'image_url',
			image_url: { url: 'data:image/png;base64,' + marker.repeat(1000) }
		})
		const messages = [
			{
				role: 'user',
				content: [{ type: 'text', text: 'batch' }, url('A'), url('B'), url('C')]
			} as any
		]
		const out = boundImagePartBytes(messages, 1600)
		// 750 bytes each against a 1600-byte cap: the two NEWEST fit, the oldest drops
		const content = out[0].content as any[]
		expect(content[1]).toEqual({ type: 'text', text: '[image omitted]' })
		expect(content[2].image_url.url).toContain('B')
		expect(content[3].image_url.url).toContain('C')
	})
})

describe('messagesHaveImageParts', () => {
	it('detects an image part anywhere in the history', () => {
		const messages: ChatCompletionMessageParam[] = [
			{ role: 'user', content: 'plain' },
			{
				role: 'user',
				content: [{ type: 'image_url', image_url: { url: 'data:image/png;base64,A' } }]
			} as any
		]
		expect(messagesHaveImageParts(messages)).toBe(true)
	})

	it('is false for string content and image-free part arrays', () => {
		const messages: ChatCompletionMessageParam[] = [
			{ role: 'user', content: 'plain' },
			{ role: 'user', content: [{ type: 'text', text: 'also plain' }] } as any
		]
		expect(messagesHaveImageParts(messages)).toBe(false)
	})
})

describe('imagesFromContent', () => {
	it('recovers image parts and skips text (including the omitted placeholder)', () => {
		const content = [
			{ type: 'text', text: 'look' },
			{ type: 'image_url', image_url: { url: 'data:image/jpeg;base64,AAAA' } },
			{ type: 'text', text: '[image omitted]' },
			{ type: 'image_url', image_url: { url: 'data:image/png;base64,BBBB' } }
		]
		expect(imagesFromContent(content)).toEqual([
			{ dataUrl: 'data:image/jpeg;base64,AAAA', mediaType: 'image/jpeg' },
			{ dataUrl: 'data:image/png;base64,BBBB', mediaType: 'image/png' }
		])
	})

	it('is undefined for string content and image-free part arrays', () => {
		expect(imagesFromContent('plain')).toBeUndefined()
		expect(imagesFromContent([{ type: 'text', text: 'plain' }])).toBeUndefined()
	})
})

describe('captureScale', () => {
	it('captures small targets above CSS resolution, capped at 2x', () => {
		expect(captureScale(400)).toBe(2)
	})

	it('never yields a raster larger than MAX_IMAGE_EDGE, even below 1x', () => {
		// A tall scrolling app body: rasterising at >=1x would allocate an
		// unbounded canvas only for normalize to shrink or reject it.
		expect(captureScale(10_000) * 10_000).toBe(1568)
		expect(captureScale(10_000)).toBeLessThan(1)
	})
})
