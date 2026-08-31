import { describe, it, expect, vi } from 'vitest'
import { createLatestWins } from './latestWins'

function deferred() {
	let resolve!: () => void
	const promise = new Promise<void>((res) => {
		resolve = res
	})
	return { promise, resolve }
}

const tick = () => new Promise((r) => setTimeout(r))

describe('createLatestWins', () => {
	it('runs tasks for one key strictly in order', async () => {
		const q = createLatestWins()
		const first = deferred()
		const order: string[] = []
		q.run('s', async () => {
			order.push('first:start')
			await first.promise
			order.push('first:end')
		})
		await tick()
		q.run('s', () => {
			order.push('second')
		})
		await tick()
		expect(order).toEqual(['first:start'])
		first.resolve()
		await tick()
		expect(order).toEqual(['first:start', 'first:end', 'second'])
	})

	it('skips superseded tasks and runs only the newest', async () => {
		const q = createLatestWins()
		const ran: string[] = []
		q.run('s', () => {
			ran.push('first')
		})
		q.run('s', () => {
			ran.push('second')
		})
		q.run('s', () => {
			ran.push('third')
		})
		await tick()
		expect(ran).toEqual(['third'])
	})

	it('flips the running task probe when a newer task is queued', async () => {
		const q = createLatestWins()
		const gate = deferred()
		const seen: boolean[] = []
		q.run('s', async (superseded) => {
			seen.push(superseded())
			await gate.promise
			seen.push(superseded())
		})
		await tick()
		q.run('s', () => {})
		gate.resolve()
		await tick()
		expect(seen).toEqual([false, true])
	})

	it('runs the next task after one fails', async () => {
		const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
		try {
			const q = createLatestWins()
			const ran: string[] = []
			q.run('s', () => {
				throw new Error('boom')
			})
			q.run('s', () => {
				ran.push('after')
			})
			await tick()
			expect(ran).toEqual(['after'])
		} finally {
			errorSpy.mockRestore()
		}
	})
})
