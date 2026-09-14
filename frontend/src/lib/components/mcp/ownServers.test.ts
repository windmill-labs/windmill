import { describe, expect, it } from 'vitest'
import { isOwnOrSharedMcpPath } from './ownServers'

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
