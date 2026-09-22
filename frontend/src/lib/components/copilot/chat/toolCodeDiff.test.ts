import { describe, expect, it } from 'vitest'
import hljs from 'highlight.js/lib/core'
import { diffLineCounts, hasToolCodeDiff, toolCodeDiff, toolDiffLines } from './toolCodeDiff'
import { TOOL_CODE_DIFF_LANGUAGES, toolCodeDiffLanguage } from './toolCodeDiffLanguage'
import type { ToolDisplayMessage } from './shared'

function message(overrides: Partial<ToolDisplayMessage>): ToolDisplayMessage {
	return {
		role: 'tool',
		tool_call_id: 'call-1',
		content: 'Updated script "f/example" as a draft',
		...overrides
	}
}

describe('toolCodeDiff', () => {
	it('uses the saved full script sides after an edit succeeds', () => {
		const codeDiff = {
			before: 'const before = true\n',
			after: 'const after = true\n',
			lang: 'typescript'
		}

		expect(
			toolCodeDiff(
				message({
					toolName: 'edit_script',
					parameters: { old_string: 'before', new_string: 'after' },
					codeDiff
				})
			)
		).toBe(codeDiff)
	})

	it('uses edit arguments until the full script sides are available', () => {
		expect(
			toolCodeDiff(
				message({
					toolName: 'edit_script',
					parameters: { old_string: 'before', new_string: 'after' }
				})
			)
		).toEqual({ before: 'before', after: 'after', lang: 'plaintext' })
	})

	it('renders streamed edit arguments before their JSON object is complete', () => {
		expect(
			toolCodeDiff(
				message({
					toolName: 'edit_script',
					parameters: '{"old_string":"before\\nline","new_string":"after'
				})
			)
		).toEqual({ before: 'before\nline', after: 'after', lang: 'plaintext' })
	})

	it('only marks tools with a defined argument diff as diff-capable', () => {
		expect(hasToolCodeDiff('edit_script')).toBe(true)
		expect(hasToolCodeDiff('write_script')).toBe(false)
		expect(hasToolCodeDiff(undefined)).toBe(false)
	})

	it('counts added and removed lines independently', () => {
		expect(
			diffLineCounts({
				before: 'one\ntwo\nthree\n',
				after: 'one\nnew\nthree\nfour\n',
				lang: 'plaintext'
			})
		).toEqual({
			added: 2,
			removed: 1
		})
	})

	it('keeps both gutters correct across removed and added lines', () => {
		expect(
			toolDiffLines({
				before: 'one\ntwo\nthree\n',
				after: 'one\nnew\nthree\nfour\n',
				lang: 'plaintext'
			})
		).toEqual([
			{ kind: 'context', content: 'one', oldLine: 1, newLine: 1 },
			{
				kind: 'removed',
				content: 'two',
				oldLine: 2,
				newLine: undefined,
				changedRanges: [{ start: 0, length: 3 }]
			},
			{
				kind: 'added',
				content: 'new',
				oldLine: undefined,
				newLine: 2,
				changedRanges: [{ start: 0, length: 3 }]
			},
			{ kind: 'context', content: 'three', oldLine: 3, newLine: 3 },
			{
				kind: 'added',
				content: 'four',
				oldLine: undefined,
				newLine: 4,
				changedRanges: [{ start: 0, length: 4 }]
			}
		])
	})

	it('has a highlighter for every supported editor language', () => {
		expect(TOOL_CODE_DIFF_LANGUAGES).toEqual(
			expect.arrayContaining([
				'typescript',
				'javascript',
				'python',
				'json',
				'yaml',
				'sql',
				'shell',
				'powershell',
				'php',
				'rust',
				'graphql',
				'csharp',
				'nu',
				'java',
				'r',
				'go',
				'ruby',
				'plaintext'
			])
		)
		for (const language of TOOL_CODE_DIFF_LANGUAGES) {
			const highlighter = toolCodeDiffLanguage(language)
			hljs.registerLanguage(highlighter.name, highlighter.register)
			expect(() =>
				hljs.highlight('const changed = true', { language: highlighter.name })
			).not.toThrow()
		}
	})
})
