import { describe, expect, it } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import {
	fileToAttachedImage,
	MAX_IMAGE_BYTES,
	messagesHaveImageParts,
	parseImageDataUrl,
	stripImagePartsFromMessages,
	transcriptImage
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

describe('transcriptImage', () => {
	// The transcript must never hold the copy sent to the model (see THUMBNAIL_IMAGE_EDGE).
	it('uses the bounded copy and drops the model-resolution one', () => {
		const shown = transcriptImage({
			dataUrl: 'data:image/png;base64,FULLRES',
			mediaType: 'image/png',
			name: 'a.png',
			previewUrl: 'data:image/png;base64,SMALL'
		})
		expect(shown.dataUrl).toBe('data:image/png;base64,SMALL')
		expect(shown.previewUrl).toBeUndefined()
		expect(shown.name).toBe('a.png')
	})

	it('keeps the original when a downscale would not be smaller', () => {
		const only = { dataUrl: 'data:image/png;base64,X', mediaType: 'image/png' as const }
		expect(transcriptImage(only)).toBe(only)
	})
})
