import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { PENDING_TTL_MS, remoteDeployAuthorizeUrl, takePendingConnect } from './remoteDeploy'

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

	it('expires an attempt left unfinished, and drops it from storage', () => {
		vi.useFakeTimers()
		const abandoned = startConnect()
		vi.advanceTimersByTime(PENDING_TTL_MS + 60_000)
		startConnect()
		expect(localStorage.length).toBe(1)
		expect(takePendingConnect(abandoned)).toBeUndefined()
	})
})
