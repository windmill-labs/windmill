import { describe, expect, it } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'
import {
	dataUrlToImagePart,
	fileToAttachedImage,
	MAX_IMAGE_BYTES,
	parseImageDataUrl,
	stripImagePartsFromMessages,
	transcriptImage
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

describe('transcriptImage', () => {
	// displayMessages are never compacted and are re-cloned into IndexedDB on every
	// save, so the transcript must never hold the copy sent to the model.
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
