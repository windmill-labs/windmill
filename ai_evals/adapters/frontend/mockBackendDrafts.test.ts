import { afterEach, beforeEach, describe, expect, it } from 'bun:test'
import {
	clearBenchmarkDrafts,
	getBenchmarkDraftForUser,
	listBenchmarkDrafts,
	resetBenchmarkMockBackend,
	seedBenchmarkDraft,
	updateBenchmarkDraft
} from './mockBackend'

const WORKSPACE = 'benchmark-drafts-ws'

// Drives the in-memory stand-in for the per-user draft backend (`DraftService`)
// that the global AI-chat eval round-trips its drafts through. Mirrors the
// production-unit-test mock in
// `frontend/src/lib/components/copilot/chat/global/core.test.ts`.
describe('mockBackend drafts', () => {
	beforeEach(() => resetBenchmarkMockBackend())
	afterEach(() => resetBenchmarkMockBackend())

	it('round-trips a saved draft through update / get / list', () => {
		const value = { summary: 'Greet a user', content: 'export async function main() {}' }
		const res = updateBenchmarkDraft({
			workspace: WORKSPACE,
			kind: 'script',
			path: 'f/evals/greet',
			requestBody: { value }
		})
		expect(res.status).toBe('saved')

		expect(getBenchmarkDraftForUser({ workspace: WORKSPACE, kind: 'script', path: 'f/evals/greet' }).value).toEqual(
			value
		)

		const rows = listBenchmarkDrafts(WORKSPACE)
		expect(rows).toHaveLength(1)
		expect(rows[0]).toMatchObject({ kind: 'script', path: 'f/evals/greet', summary: 'Greet a user', draft_only: true })
	})

	it('treats a null value as a delete', () => {
		updateBenchmarkDraft({
			workspace: WORKSPACE,
			kind: 'variable',
			path: 'f/evals/token',
			requestBody: { value: { summary: 'token' } }
		})
		updateBenchmarkDraft({
			workspace: WORKSPACE,
			kind: 'variable',
			path: 'f/evals/token',
			requestBody: { value: null }
		})

		expect(listBenchmarkDrafts(WORKSPACE)).toHaveLength(0)
		expect(() => getBenchmarkDraftForUser({ workspace: WORKSPACE, kind: 'variable', path: 'f/evals/token' })).toThrow()
	})

	it('throws a 404-shaped error when no draft exists', () => {
		try {
			getBenchmarkDraftForUser({ workspace: WORKSPACE, kind: 'script', path: 'f/evals/missing' })
			throw new Error('expected a throw')
		} catch (e) {
			expect((e as { status?: number }).status).toBe(404)
		}
	})

	it('seeds a draft as a backend row that a later edit overwrites', () => {
		seedBenchmarkDraft(WORKSPACE, 'script', 'f/evals/current', { content: 'seed' })
		expect(getBenchmarkDraftForUser({ workspace: WORKSPACE, kind: 'script', path: 'f/evals/current' }).value).toEqual({
			content: 'seed'
		})

		// A model edit persists the same path and must win over the seed.
		updateBenchmarkDraft({
			workspace: WORKSPACE,
			kind: 'script',
			path: 'f/evals/current',
			requestBody: { value: { content: 'edited' } }
		})
		expect(getBenchmarkDraftForUser({ workspace: WORKSPACE, kind: 'script', path: 'f/evals/current' }).value).toEqual({
			content: 'edited'
		})
	})

	it('clears only the targeted workspace', () => {
		seedBenchmarkDraft(WORKSPACE, 'script', 'f/a', { content: 'a' })
		seedBenchmarkDraft('other-ws', 'script', 'f/b', { content: 'b' })

		clearBenchmarkDrafts(WORKSPACE)

		expect(listBenchmarkDrafts(WORKSPACE)).toHaveLength(0)
		expect(listBenchmarkDrafts('other-ws')).toHaveLength(1)
	})
})
