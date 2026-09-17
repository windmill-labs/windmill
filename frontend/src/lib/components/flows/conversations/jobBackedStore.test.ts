import { describe, expect, it, vi } from 'vitest'
import { JobBackedStore } from './jobBackedStore.svelte'

const tick = (ms = 0) => new Promise((resolve) => setTimeout(resolve, ms))

describe('JobBackedStore', () => {
	it('keeps every answer when reads finish out of order, and reads each job once', async () => {
		const load = vi.fn((_ws: string, jobId: string) =>
			tick(jobId === 'a' ? 20 : 5).then(() => `details of ${jobId}`)
		)
		const store = new JobBackedStore(() => 'ws', '', load)
		expect(store.get('a')).toBe('')
		expect(store.get('b')).toBe('')
		await tick(40)
		expect(store.get('a')).toBe('details of a')
		expect(store.get('b')).toBe('details of b')
		expect(load).toHaveBeenCalledTimes(2)
	})
})
