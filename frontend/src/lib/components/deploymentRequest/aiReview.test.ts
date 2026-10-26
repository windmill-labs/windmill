import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { WorkspaceItemDiff } from '$lib/gen'

const h = vi.hoisted(() => ({
	prompt: '',
	system: '',
	workspace: '' as string | undefined,
	reads: 0,
	instructionReads: [] as string[]
}))

vi.mock('$lib/components/copilot/lib', () => ({
	getNonStreamingCompletion: async (
		messages: { content: string }[],
		_abort: AbortController,
		options?: { workspace?: string }
	) => {
		h.system = messages[0].content
		h.prompt = messages[1].content
		h.workspace = options?.workspace
		return '{"summary":"s","findings":[]}'
	}
}))
// Mirrors the shared loader, which answers `{}` instead of throwing when a read fails.
vi.mock('$lib/utils_workspace_deploy', () => ({
	getItemValue: async (_kind: string, path: string) => {
		h.reads++
		if (path === 'f/a/unreadable') return {}
		return { content: path.startsWith('f/big/') ? 'x'.repeat(30_000) : 'x' }
	}
}))

vi.mock('$lib/gen', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/gen')>()),
	ResourceService: {
		listResource: async ({ workspace }: { workspace: string }) =>
			workspace === 'parent'
				? [{ path: 'f/a/REVIEW' }, { path: 'f/other/REVIEW' }, { path: 'f/a/gone' }]
				: [{ path: 'f/a/FORK_REVIEW' }],
		getResourceValue: async ({ workspace, path }: { workspace: string; path: string }) => {
			h.instructionReads.push(`${workspace}:${path}`)
			if (path === 'f/a/gone') throw new Error('not found')
			return { content: `body of ${path}` }
		}
	}
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

beforeEach(() => {
	h.reads = 0
	h.instructionReads = []
})

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
		expect(h.workspace).toBe('fork')
	})

	it('stops reading items once a section no longer fits the budget', async () => {
		const diffs = Array.from({ length: 30 }, (_, i) => diff(`f/big/${i}`))
		const result = await reviewDeployment(diffs, 'fork', 'parent', new AbortController())
		// Only the first batch of 6 is read, both sides each.
		expect(h.reads).toBe(12)
		expect(result.omitted).toContain('script:f/big/29')
	})

	it("follows the parent's instructions covering a changed item, and no others", async () => {
		const result = await reviewDeployment([diff('f/a/b')], 'fork', 'parent', new AbortController())
		expect(result.instructions).toEqual(['f/a/REVIEW'])
		expect(result.skippedInstructions).toEqual(['f/a/gone'])
		expect(h.system).toContain('<folder_instructions path="f/a/REVIEW" scope="f/a/">')
		expect(h.system).toContain('body of f/a/REVIEW')
		expect(h.system).not.toContain('f/other/REVIEW')
		expect(h.instructionReads.every((r) => r.startsWith('parent:'))).toBe(true)
	})
})
