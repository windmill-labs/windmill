import { describe, expect, it } from 'vitest'
import { storedAttachmentName } from './attachmentNames'

/**
 * The worker reads an attachment's media type from the object key and nothing else, so a
 * key whose extension disagrees with the bytes reaches the provider mislabelled.
 */
describe('storedAttachmentName', () => {
	// The composer re-encodes images, so the picked extension is the one that lies.
	it('renames a re-encoded image to the type it was encoded as', () => {
		expect(storedAttachmentName('photo.webp', 'image/png')).toBe('photo.png')
		expect(storedAttachmentName('holiday.png', 'image/jpeg')).toBe('holiday.jpg')
	})

	it('gives an extension to a name that has none', () => {
		expect(storedAttachmentName('attachment-1', 'image/png')).toBe('attachment-1.png')
		expect(storedAttachmentName('contract', 'application/pdf')).toBe('contract.pdf')
	})

	it('replaces only the last extension', () => {
		expect(storedAttachmentName('report.2026.final.webp', 'image/png')).toBe(
			'report.2026.final.png'
		)
	})

	// Blobs upload byte for byte, so a type we do not re-encode keeps the name as picked.
	it('leaves a type it does not re-encode alone', () => {
		expect(storedAttachmentName('notes.csv', 'text/csv')).toBe('notes.csv')
		expect(storedAttachmentName('archive.zip', 'application/zip')).toBe('archive.zip')
	})
})
