import { describe, expect, it, vi } from 'vitest'
import type { WorkspaceItemDiff } from '$lib/gen'

const h = vi.hoisted(() => ({ prompt: '' }))

vi.mock('$lib/components/copilot/lib', () => ({
	getNonStreamingCompletion: async (messages: { content: string }[]) => {
		h.prompt = messages[1].content
		return '{"summary":"s","findings":[]}'
	}
}))
// Mirrors the shared loader, which answers `{}` instead of throwing when a read fails.
vi.mock('$lib/utils_workspace_deploy', () => ({
	getItemValue: async (_kind: string, path: string) =>
		path === 'f/a/unreadable' ? {} : { content: 'x' }
}))

const { parseReviewAnswer, reviewDeployment } = await import('./aiReview')

function diff(path: string): WorkspaceItemDiff {
	return {
		kind: 'script',
		path,
		ahead: 1,
		behind: 0,
		has_changes: true,
		exists_in_source: true,
		exists_in_fork: true
	}
}

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
		const { findings } = parseReviewAnswer(answer, [diff('f/a/b')])
		expect(findings.map((f) => [f.comment, f.kind, f.path])).toEqual([
			['real', 'script', 'f/a/b'],
			['phantom', '', '']
		])
	})
})

describe('reviewDeployment', () => {
	it('reports an item whose read failed instead of reviewing it as emptied', async () => {
		const result = await reviewDeployment(
			[diff('f/a/b'), diff('f/a/unreadable')],
			'fork',
			'parent',
			new AbortController()
		)
		expect(result.unreadable).toEqual(['script:f/a/unreadable'])
		expect(h.prompt).toContain('### script f/a/unreadable (modified)\nContents could not be read.')
	})
})
