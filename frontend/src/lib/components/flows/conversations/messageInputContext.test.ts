import { describe, expect, it } from 'vitest'
import { argsToMessageInputs } from './messageInputContext.svelte'

const SCHEMA = {
	properties: {
		token: { type: 'string', password: true },
		city: { type: 'string' },
		report: { type: 'object', format: 'resource-s3_object' }
	}
}

function summaries(elements: { title?: string; content?: string }[]) {
	return elements.map((e) => e.content)
}

describe('argsToMessageInputs', () => {
	it('never puts a secret input on screen', () => {
		const { contextElements } = argsToMessageInputs(
			'ws',
			{ token: 'hunter2', city: 'Paris' },
			SCHEMA
		)
		expect(summaries(contextElements as any)).toEqual(['<hidden>', 'Paris'])
	})

	// Only images get a thumbnail lane; every other s3 file is a chip naming its key.
	it('splits attachments into thumbnails and file chips', () => {
		const { images, contextElements } = argsToMessageInputs(
			'ws',
			{ report: [{ s3: 'a/shot.png' }, { s3: 'a/notes.pdf' }] },
			SCHEMA
		)
		expect(images.map((i) => i.name)).toEqual(['shot.png'])
		expect(images[0].dataUrl).toContain('file_key=a%2Fshot.png')
		expect(contextElements).toHaveLength(1)
	})

	it('leaves out the message and anything the composer already shows', () => {
		const { contextElements } = argsToMessageInputs(
			'ws',
			{ user_message: 'hi', city: 'Paris' },
			SCHEMA,
			new Set(['city'])
		)
		expect(contextElements).toEqual([])
	})
})
