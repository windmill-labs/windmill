import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { remoteDeployAuthorizeUrl, takePendingConnect } from './remoteDeploy'

const target = { base_url: 'https://prod.example.com', workspace_id: 'prod' }

function startConnect(workspace = 'dev'): string {
	const url = new URL(remoteDeployAuthorizeUrl(target, workspace, '/deploy/flow/f/a'))
	return url.searchParams.get('state')!
}

describe('remote deploy connect state', () => {
	beforeEach(() => {
		localStorage.clear()
		vi.stubGlobal('window', { location: { origin: 'https://dev.example.com' } })
	})
	afterEach(() => {
		vi.useRealTimers()
		vi.unstubAllGlobals()
	})

	// The state is what keeps any other page from planting a token as the user's.
	it('matches only the state it issued, and only once', () => {
		const state = startConnect()
		expect(takePendingConnect('forged')).toBeUndefined()
		expect(takePendingConnect(state)).toMatchObject({ workspace: 'dev', target })
		expect(takePendingConnect(state)).toBeUndefined()
	})

	it('keeps concurrent attempts apart', () => {
		const first = startConnect('dev')
		const second = startConnect('staging')
		expect(takePendingConnect(first)?.workspace).toBe('dev')
		expect(takePendingConnect(second)?.workspace).toBe('staging')
	})

	it('expires an attempt left unfinished', () => {
		vi.useFakeTimers()
		const state = startConnect()
		vi.advanceTimersByTime(16 * 60_000)
		expect(takePendingConnect(state)).toBeUndefined()
	})
})
