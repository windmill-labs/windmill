import { describe, expect, it } from 'vitest'
import { withoutHeavyPayloads } from './sessionMirrorPayload'
import type { DisplayMessage } from '$lib/components/copilot/chat/shared'

// Frames go out several times a second for the whole turn, so anything left in
// them is re-cloned and re-broadcast on every tick. The caps allow megabytes per
// image and per pasted file, and a structural typecheck cannot catch a field
// name that no longer exists — the message union is cast on the way out.
describe('withoutHeavyPayloads', () => {
	it('strips the bytes and keeps what the transcript renders', () => {
		const messages = [
			{
				role: 'user',
				content: 'look at this',
				images: [
					{ dataUrl: 'data:image/png;base64,AAAA', mediaType: 'image/png', name: 'shot.png' }
				],
				files: [{ name: 'notes.md', content: 'a very long pasted file', id: 'f1' }],
				pastes: [{ id: 1, lines: 400, content: 'a very long collapsed paste' }]
			},
			{ role: 'tool', content: 'took a screenshot', imageUrl: 'data:image/png;base64,BBBB' }
		] as unknown as DisplayMessage[]

		const stripped = withoutHeavyPayloads(messages)

		expect(JSON.stringify(stripped)).not.toContain('data:image')
		expect(JSON.stringify(stripped)).not.toContain('a very long pasted file')
		expect(JSON.stringify(stripped)).not.toContain('a very long collapsed paste')
		// The chip renders from the line count, so that has to survive.
		expect((stripped[0] as any).pastes).toEqual([{ id: 1, lines: 400, content: '' }])
		// The file chip is labelled from the name, so that has to survive.
		expect((stripped[0] as any).files).toEqual([{ name: 'notes.md', content: '', id: 'f1' }])
		// Images are dropped whole: the bubble renders one <img> per entry with no
		// per-image guard, so an emptied url would show a broken image instead.
		expect((stripped[0] as any).images).toBeUndefined()
		expect((stripped[0] as any).content).toBe('look at this')
	})

	it('passes through a message with nothing heavy in it', () => {
		const messages = [{ role: 'assistant', content: 'plain reply' }] as unknown as DisplayMessage[]
		expect(withoutHeavyPayloads(messages)[0]).toBe(messages[0])
	})
})
