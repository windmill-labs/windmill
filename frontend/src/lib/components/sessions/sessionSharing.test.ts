import { describe, expect, it } from 'vitest'

import { readSessionSnapshot } from './sessionSharing'

describe('readSessionSnapshot', () => {
	it('keeps only inline images and the rows and artifacts the viewer can render', () => {
		const snapshot = readSessionSnapshot({
			messages: [
				{
					role: 'user',
					content: 'hi',
					images: [
						{ dataUrl: 'https://tracker.example/u.png', mediaType: 'image/png' },
						{ dataUrl: 'data:image/png;base64,AAAA', mediaType: 'image/png' }
					]
				},
				{ role: 'tool', content: 'shot', imageUrl: 'https://tracker.example/t.png' },
				{ role: 'tool', content: 'shot', imageUrl: 'data:image/png;base64,BBBB' },
				{
					role: 'tool',
					content: 'Run',
					isLoading: true,
					needsConfirmation: true,
					runForm: { kind: 'test', code: 'export async function main() {}' },
					userQuestion: { question: 'ok?' },
					actions: [{ label: 'Open' }],
					inspectedRun: { jobId: 'j', workspace: 'w', runId: 'j' }
				},
				{ role: 'system', content: 'not a chat row' },
				'not an object'
			],
			artifacts: [
				{ id: 'a', name: 'Plan', kind: 'md', content: '# Plan' },
				{ id: 'b', name: 'Bad', kind: 'svg', content: '<svg/>' }
			]
		})

		expect(snapshot.messages.map((m) => m.role)).toEqual(['user', 'tool', 'tool', 'tool'])
		expect(snapshot.messages[0]).toMatchObject({
			images: [{ dataUrl: 'data:image/png;base64,AAAA' }]
		})
		expect((snapshot.messages[1] as { imageUrl?: string }).imageUrl).toBeUndefined()
		expect(snapshot.messages[2]).toMatchObject({ imageUrl: 'data:image/png;base64,BBBB' })
		// Every affordance that would act as the reader is gone, pending state included.
		expect(snapshot.messages[3]).toEqual({
			role: 'tool',
			tool_call_id: 'shared-3',
			content: 'Run',
			showDetails: false,
			blockedByPlanMode: false,
			declinedByUser: false
		})
		expect(snapshot.artifacts.map((a) => a.id)).toEqual(['a'])
	})

	it('reads a malformed snapshot as an empty one', () => {
		expect(readSessionSnapshot('nope')).toEqual({ messages: [], artifacts: [] })
	})
})
