import { describe, expect, it } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import {
	dataUrlToImagePart,
	fileToAttachedImage,
	MAX_IMAGE_BYTES,
	parseImageDataUrl,
	stripImagePartsFromMessages
} from './imageUtils'

describe('fileToAttachedImage size bound', () => {
	// Decoding allocates ~4 bytes per pixel before the downscale can run, so an
	// oversized file has to be refused before it is read — not after.
	it('rejects a file over the byte cap without reading it', async () => {
		let read = false
		const blob = {
			size: MAX_IMAGE_BYTES + 1,
			type: 'image/png',
			get arrayBuffer() {
				read = true
				return async () => new ArrayBuffer(0)
			}
		} as unknown as Blob

		await expect(fileToAttachedImage(blob)).rejects.toThrow(/too large/i)
		expect(read).toBe(false)
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

describe('dataUrlToImagePart', () => {
	it('wraps a data URL as an OpenAI image_url content part', () => {
		expect(dataUrlToImagePart('data:image/png;base64,AAAA')).toEqual({
			type: 'image_url',
			image_url: { url: 'data:image/png;base64,AAAA' }
		})
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
