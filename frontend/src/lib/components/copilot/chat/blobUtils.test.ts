import { describe, expect, it } from 'vitest'
import { matchesAccept } from './blobUtils'

function file(name: string, type: string): File {
	return new File(['x'], name, { type })
}

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
