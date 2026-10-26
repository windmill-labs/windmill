import { describe, expect, it, vi } from 'vitest'
import type { WorkspaceItemDiff } from '$lib/gen'

vi.mock('$lib/components/copilot/lib', () => ({}))
vi.mock('$lib/utils_workspace_deploy', () => ({}))

const { parseReviewAnswer } = await import('./aiReview')

const diff = {
	kind: 'script',
	path: 'f/a/b',
	ahead: 1,
	behind: 0,
	has_changes: true,
	exists_in_source: true,
	exists_in_fork: true
} satisfies WorkspaceItemDiff

describe('parseReviewAnswer', () => {
	it('anchors only findings on items in the diff, and ranks by severity', () => {
		const answer =
			'```json\n' +
			JSON.stringify({
				summary: 's',
				findings: [
					{ kind: 'flow', path: 'f/not/changed', severity: 'low', comment: 'phantom' },
					{ kind: 'script', path: 'f/a/b', severity: 'high', comment: 'real' }
				]
			}) +
			'\n```'
		const { findings } = parseReviewAnswer(answer, [diff])
		expect(findings.map((f) => [f.comment, f.kind, f.path])).toEqual([
			['real', 'script', 'f/a/b'],
			['phantom', '', '']
		])
	})
})
