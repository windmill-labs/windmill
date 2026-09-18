import { describe, expect, it } from 'vitest'
import { attachmentLanes } from './messageAttachments'

describe('attachmentLanes', () => {
	it('splits images into thumbnails and everything else into named file chips', () => {
		const { images, contextElements } = attachmentLanes('ws', [
			{ input: 'user_attachments', s3: 'chat/u1/shot.PNG', storage: 'secondary' },
			{ input: 'user_attachments', s3: 'chat/u1/0/notes.pdf', filename: 'Q3 notes.pdf' },
			{ input: 'report', s3: 'chat/u1/data.csv' }
		])
		expect(images.map((i) => i.name)).toEqual(['shot.PNG'])
		expect(images[0].dataUrl).toContain('/api/w/ws/job_helpers/download_s3_file?')
		expect(images[0].dataUrl).toContain('file_key=chat%2Fu1%2Fshot.PNG')
		expect(images[0].dataUrl).toContain('storage=secondary')
		expect(contextElements.map((c) => c.title)).toEqual(['Q3 notes.pdf', 'data.csv'])
	})

	it('shows an image as a file chip when there is no workspace to link it in', () => {
		const { images, contextElements } = attachmentLanes(undefined, [
			{ input: 'user_attachments', s3: 'chat/u1/shot.png' }
		])
		expect(images).toEqual([])
		expect(contextElements.map((c) => c.title)).toEqual(['shot.png'])
	})
})
