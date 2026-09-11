import { describe, expect, it } from 'vitest'
import { dataUrlToBlob, matchesAccept } from './blobUtils'

function file(name: string, type: string): File {
	return new File(['x'], name, { type })
}

describe('dataUrlToBlob', () => {
	// The bytes are re-uploaded verbatim, so a decode that drops or shifts one is a
	// corrupted file the reader only discovers downstream.
	it('decodes base64 back to the exact bytes', async () => {
		const bytes = new Uint8Array([0x00, 0xff, 0x10, 0x89, 0x50])
		const b64 = btoa(String.fromCharCode(...bytes))
		const blob = dataUrlToBlob(`data:application/pdf;base64,${b64}`)
		expect(blob.type).toBe('application/pdf')
		expect(new Uint8Array(await blob.arrayBuffer())).toEqual(bytes)
	})

	it('percent-decodes a url that is not base64', async () => {
		const blob = dataUrlToBlob('data:text/plain,hello%20world')
		expect(blob.type).toBe('text/plain')
		expect(await blob.text()).toBe('hello world')
	})

	it('falls back to a media type when the url names none', async () => {
		expect(dataUrlToBlob('data:;base64,QQ==').type).toBe('application/octet-stream')
		expect(dataUrlToBlob('data:;base64,QQ==', 'image/png').type).toBe('image/png')
	})
})

describe('matchesAccept', () => {
	it('matches an extension, a type wildcard and an exact media type', () => {
		expect(matchesAccept(file('report.PDF', ''), '.pdf')).toBe(true)
		expect(matchesAccept(file('shot.png', 'image/png'), 'image/*')).toBe(true)
		expect(matchesAccept(file('shot.png', 'image/png'), 'image/png')).toBe(true)
	})

	it('refuses a file no pattern covers, and allows everything when the list is empty', () => {
		expect(matchesAccept(file('notes.txt', 'text/plain'), '.pdf, image/*')).toBe(false)
		expect(matchesAccept(file('notes.txt', 'text/plain'), '')).toBe(true)
	})
})
