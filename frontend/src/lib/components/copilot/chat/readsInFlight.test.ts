import { describe, expect, it } from 'vitest'
import { ReadsInFlight } from './readsInFlight'

const tick = () => new Promise((resolve) => setTimeout(resolve, 0))

describe('ReadsInFlight', () => {
	it('settles at once when nothing runs', async () => {
		const reads = new ReadsInFlight()
		expect(reads.busy).toBe(false)
		await reads.settled()
	})

	it('waits for the read a routing hold starts as it ends', async () => {
		const reads = new ReadsInFlight()
		const endRouting = reads.hold()
		expect(reads.busy).toBe(true)
		let read: 'running' | 'done' = 'running'
		let settled = false
		void reads.settled().then(() => (settled = true))
		// The drop is routed, and only then does the read of the file start.
		let finishRead!: () => void
		endRouting()
		void reads.track(async () => {
			await new Promise<void>((resolve) => (finishRead = resolve))
			read = 'done'
		})
		await tick()
		expect(settled).toBe(false)
		finishRead()
		await reads.settled()
		expect(read).toBe('done')
		expect(reads.busy).toBe(false)
	})

	it('keeps counting a read that fails, and hands its rejection to the caller', async () => {
		const reads = new ReadsInFlight()
		const failed = reads.track(async () => {
			throw new Error('unreadable')
		})
		await expect(failed).rejects.toThrow('unreadable')
		await reads.settled()
		expect(reads.busy).toBe(false)
	})
})
