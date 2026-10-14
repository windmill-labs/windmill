import { beforeEach, describe, expect, it, vi } from 'vitest'

const { session, roleMock } = vi.hoisted(() => ({
	session: { workspace_id: 'browsed', username: 'alice', pgroups: ['g/browsed-only'] },
	roleMock: vi.fn()
}))

vi.mock('$lib/stores', () => ({
	userStore: { subscribe: (run: (v: unknown) => void) => (run({ ...session }), () => {}) }
}))
vi.mock('$lib/user', () => ({ getWorkspaceRole: roleMock }))

import { isOwnOrSharedMcpPath, mcpViewer } from './ownServers'

const alice = { username: 'alice', pgroups: ['g/eng'] }

describe('isOwnOrSharedMcpPath', () => {
	it('keeps own, folder, and explicitly shared servers; drops the rest of u/', () => {
		expect(isOwnOrSharedMcpPath('u/alice/notion', {}, alice)).toBe(true)
		expect(isOwnOrSharedMcpPath('f/team/notion', {}, alice)).toBe(true)
		expect(isOwnOrSharedMcpPath('u/bob/notion', {}, alice)).toBe(false)
		expect(isOwnOrSharedMcpPath('u/alicex/notion', {}, alice)).toBe(false)
		expect(isOwnOrSharedMcpPath('u/bob/notion', { 'u/alice': false }, alice)).toBe(true)
		expect(isOwnOrSharedMcpPath('u/bob/notion', { 'g/eng': false }, alice)).toBe(true)
		expect(isOwnOrSharedMcpPath('u/bob/notion', { 'g/ops': true }, alice)).toBe(false)
		expect(isOwnOrSharedMcpPath('u/alice/notion', {}, { username: undefined, pgroups: [] })).toBe(
			false
		)
	})
})

describe('mcpViewer', () => {
	beforeEach(() => roleMock.mockReset())

	it('answers from the store only for the browsed workspace', async () => {
		expect(await mcpViewer('browsed')).toEqual({ username: 'alice', pgroups: ['g/browsed-only'] })
		expect(roleMock).not.toHaveBeenCalled()
	})

	it('looks the identity up for another workspace instead of reusing the browsed one', async () => {
		roleMock.mockResolvedValue({
			kind: 'resolved',
			user: { username: 'alice_other', pgroups: ['g/eng'] }
		})
		expect(await mcpViewer('other')).toEqual({ username: 'alice_other', pgroups: ['g/eng'] })
		expect(roleMock).toHaveBeenCalledWith('other')
	})

	it('yields no identity when the lookup fails', async () => {
		roleMock.mockResolvedValue({ kind: 'lookup_failed' })
		expect(await mcpViewer('other')).toEqual({ username: undefined, pgroups: [] })
	})
})
