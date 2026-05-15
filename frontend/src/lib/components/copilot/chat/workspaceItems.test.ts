import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/gen', () => ({
	ScriptService: { listScripts: vi.fn() },
	FlowService: { listFlows: vi.fn() },
	AppService: { listApps: vi.fn() }
}))

import { extractCandidatePaths, WINDMILL_PATH_REGEX } from './workspaceItems.svelte'

describe('WINDMILL_PATH_REGEX', () => {
	it('matches simple folder and user paths', () => {
		expect(extractCandidatePaths('Use f/marketing/send_email today')).toEqual([
			'f/marketing/send_email'
		])
		expect(extractCandidatePaths('Check u/admin/cleanup')).toEqual(['u/admin/cleanup'])
	})

	it('matches paths with sub-folders and dotted segments', () => {
		expect(extractCandidatePaths('Pipeline f/etl/jobs/ingest_users.v2 runs nightly')).toEqual([
			'f/etl/jobs/ingest_users.v2'
		])
	})

	it('matches usernames containing dots (e.g. firstname.lastname)', () => {
		expect(extractCandidatePaths('Owned by u/jane.doe/report')).toEqual(['u/jane.doe/report'])
	})

	it('strips trailing sentence punctuation', () => {
		expect(extractCandidatePaths('Look at f/foo/bar.')).toEqual(['f/foo/bar'])
		expect(extractCandidatePaths('Try f/foo/bar, please')).toEqual(['f/foo/bar'])
		expect(extractCandidatePaths('Is f/foo/bar?')).toEqual(['f/foo/bar'])
	})

	it('skips matches inside URLs', () => {
		expect(extractCandidatePaths('See https://example.com/u/me/script for context')).toEqual([])
	})

	it('does not match incomplete paths', () => {
		expect(extractCandidatePaths('I tried f/folder/ but nothing')).toEqual([])
		expect(extractCandidatePaths('Just u/me')).toEqual([])
	})

	it('returns multiple unique paths from one string', () => {
		const paths = extractCandidatePaths('Run f/a/one then f/b/two and again f/a/one')
		expect(paths.sort()).toEqual(['f/a/one', 'f/b/two'])
	})

	it('exposes a global regex', () => {
		expect(WINDMILL_PATH_REGEX.global).toBe(true)
	})
})
