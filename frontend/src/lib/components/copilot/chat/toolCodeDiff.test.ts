import { describe, expect, it } from 'vitest'
import hljs from 'highlight.js/lib/core'
import {
	diffLineCounts,
	hasToolCodeDiff,
	toolCodeDiff,
	toolDiffLines,
	visibleToolDiffRows
} from './toolCodeDiff'
import { highlightedSourceLines } from './toolCodeDiffHighlight'
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

	it('uses each replacement from an in-editor edit while its arguments stream', () => {
		expect(
			toolCodeDiff(
				message({
					toolName: 'edit_code',
					parameters: {
						diffs: [
							{ old_string: 'first old', new_string: 'first new' },
							{ old_string: 'second old', new_string: 'second new' }
						]
					}
				})
			)
		).toEqual({
			before: 'first old\nsecond old',
			after: 'first new\nsecond new',
			lang: 'plaintext'
		})
	})

	it('uses each replacement while in-editor edit arguments are incomplete', () => {
		expect(
			toolCodeDiff(
				message({
					toolName: 'edit_code',
					parameters:
						'{"diffs":[{"old_string":"first old","new_string":"first new"},{"old_string":"second old","new_string":"second new'
				})
			)
		).toEqual({
			before: 'first old\nsecond old',
			after: 'first new\nsecond new',
			lang: 'plaintext'
		})
	})

	it('shows a streamed full-code editor update without replacements', () => {
		expect(
			toolCodeDiff(message({ toolName: 'edit_code', parameters: '{"code":"console.log(1)' }))
		).toEqual({ before: '', after: 'console.log(1)', lang: 'plaintext' })
	})

	it('shows streamed full-script updates before the saved diff is available', () => {
		expect(
			toolCodeDiff(
				message({ toolName: 'write_script', parameters: '{"content":"print(\\\"hello\\\")' })
			)
		).toEqual({ before: '', after: 'print("hello")', lang: 'plaintext' })
	})

	it('only marks tools with a defined argument diff as diff-capable', () => {
		expect(hasToolCodeDiff('edit_script')).toBe(true)
		expect(hasToolCodeDiff('edit_code')).toBe(true)
		expect(hasToolCodeDiff('write_script')).toBe(true)
		expect(hasToolCodeDiff('__proto__')).toBe(false)
		expect(hasToolCodeDiff('toString')).toBe(false)
		expect(hasToolCodeDiff(undefined)).toBe(false)
	})

	it('ignores tool names inherited from Object.prototype', () => {
		expect(toolCodeDiff(message({ toolName: 'constructor', parameters: {} }))).toBeUndefined()
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

	it('keeps a final newline as a diffable line', () => {
		expect(toolDiffLines({ before: 'one', after: 'one\n', lang: 'plaintext' })).toEqual([
			{ kind: 'context', content: 'one', oldLine: 1, newLine: 1 },
			{ kind: 'added', content: '', newLine: 2 }
		])
	})

	it('does not add a final-newline row when creating or deleting a file', () => {
		expect(toolDiffLines({ before: '', after: 'one\n', lang: 'plaintext' })).toEqual([
			{ kind: 'added', content: 'one', newLine: 1, changedRanges: [{ start: 0, length: 3 }] }
		])
		expect(toolDiffLines({ before: 'one\n', after: '', lang: 'plaintext' })).toEqual([
			{ kind: 'removed', content: 'one', oldLine: 1, changedRanges: [{ start: 0, length: 3 }] }
		])
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

	it('keeps unchanged trailing lines out of the change counts', () => {
		const diff = { before: 'one\ntwo\nthree', after: 'one\nnew\nthree', lang: 'plaintext' }

		expect(diffLineCounts(diff)).toEqual({ added: 1, removed: 1 })
		expect(toolDiffLines(diff).filter((line) => line.content === 'three')).toEqual([
			{ kind: 'context', content: 'three', oldLine: 3, newLine: 3 }
		])
	})

	it('does not report changes for identical files', () => {
		const diff = { before: 'one\ntwo\nthree', after: 'one\ntwo\nthree', lang: 'plaintext' }

		expect(diffLineCounts(diff)).toEqual({ added: 0, removed: 0 })
		expect(toolDiffLines(diff)).toEqual([
			{ kind: 'context', content: 'one', oldLine: 1, newLine: 1 },
			{ kind: 'context', content: 'two', oldLine: 2, newLine: 2 },
			{ kind: 'context', content: 'three', oldLine: 3, newLine: 3 }
		])
	})

	it('renders a newly created file', () => {
		expect(toolDiffLines({ before: '', after: 'print("hello")', lang: 'python' })).toEqual([
			{
				kind: 'added',
				content: 'print("hello")',
				oldLine: undefined,
				newLine: 1,
				changedRanges: [{ start: 0, length: 14 }]
			}
		])
	})

	it('renders a deleted file', () => {
		expect(toolDiffLines({ before: 'print("goodbye")', after: '', lang: 'python' })).toEqual([
			{
				kind: 'removed',
				content: 'print("goodbye")',
				oldLine: 1,
				newLine: undefined,
				changedRanges: [{ start: 0, length: 16 }]
			}
		])
	})

	it('uses Monaco line alignment for reordered lines', () => {
		expect(
			toolDiffLines({
				before: 'const a = 1\nconst b = 2',
				after: 'const b = 2\nconst a = 1',
				lang: 'typescript'
			})
		).toEqual([
			{
				kind: 'added',
				content: 'const b = 2',
				newLine: 1,
				changedRanges: [{ start: 0, length: 11, extendsToEnd: true }]
			},
			{ kind: 'context', content: 'const a = 1', oldLine: 1, newLine: 2 },
			{
				kind: 'removed',
				content: 'const b = 2',
				oldLine: 2,
				changedRanges: [{ start: 0, length: 11 }]
			}
		])
	})

	it('avoids expensive alignment while arguments stream', () => {
		const before = Array.from({ length: 1_000 }, (_, index) => `before ${index}`).join('\n')
		const after = Array.from({ length: 1_000 }, (_, index) => `after ${index}`).join('\n')

		expect(toolDiffLines({ before, after, lang: 'plaintext' }, true)).toHaveLength(400)
		expect(diffLineCounts({ before, after, lang: 'plaintext' }, true)).toEqual({
			added: 1_000,
			removed: 1_000
		})
	})

	it('diffs a small edit in a large file', () => {
		const beforeLines = Array.from({ length: 600 }, (_, index) => `line ${index}`)
		const afterLines = beforeLines.map((line, index) => (index === 400 ? 'changed' : line))
		const diff = { before: beforeLines.join('\n'), after: afterLines.join('\n'), lang: 'plaintext' }
		const lines = toolDiffLines(diff)

		expect(lines.filter((line) => line.kind !== 'context')).toMatchObject([
			{ kind: 'removed', content: 'line 400', oldLine: 401 },
			{ kind: 'added', content: 'changed', newLine: 401 }
		])
		expect(lines[399]).toEqual({ kind: 'context', content: 'line 399', oldLine: 400, newLine: 400 })
		expect(diffLineCounts(diff)).toEqual({ added: 1, removed: 1 })
	})

	it('reports a large rewrite as a whole-file change', () => {
		const before = Array.from({ length: 1_000 }, (_, index) => `before ${index}`).join('\n')
		const after = Array.from({ length: 1_000 }, (_, index) => `after ${index}`).join('\n')

		expect(diffLineCounts({ before, after, lang: 'plaintext' })).toEqual({
			added: 1_000,
			removed: 1_000
		})
	})

	it('shows each row of a small edit once', () => {
		const lines = toolDiffLines({ before: 'a\nb\nc\n', after: 'a\nB\nc\n', lang: 'plaintext' })

		expect(visibleToolDiffRows(lines, new Set())).toEqual(lines)
		expect(lines.map((line) => line.kind)).toEqual(['context', 'removed', 'added', 'context'])
	})

	it('bounds the rows of a large rewrite', () => {
		const before = Array.from({ length: 1_000 }, (_, index) => `before ${index}`).join('\n')
		const after = Array.from({ length: 1_000 }, (_, index) => `after ${index}`).join('\n')
		const rows = visibleToolDiffRows(toolDiffLines({ before, after, lang: 'plaintext' }), new Set())

		expect(rows).toHaveLength(802)
		expect(rows[400]).toEqual({ kind: 'omitted', key: 'removed:0', count: 600 })
		expect(rows[401]).toMatchObject({ kind: 'added', newLine: 1 })
		expect(rows[801]).toEqual({ kind: 'omitted', key: 'added:1000', count: 600 })

		const expanded = visibleToolDiffRows(
			toolDiffLines({ before, after, lang: 'plaintext' }),
			new Set(['removed:0'])
		)
		expect(expanded).toHaveLength(1_001)
		expect(expanded[1_000]).toEqual({ kind: 'omitted', key: 'end', count: 1_000 })
	})

	it('highlights each line within the scope of the whole source', () => {
		const typescript = toolCodeDiffLanguage('typescript')
		hljs.registerLanguage(typescript.name, typescript.register)
		const code = 'const s = `a\nb`\nconst n = 1\n'

		const lines = highlightedSourceLines(code, typescript.name)
		expect(lines).toHaveLength(3)
		expect(lines[0]).toBe(
			'<span class="hljs-keyword">const</span> s = <span class="hljs-string">`a</span>'
		)
		expect(lines[1]).toMatch(/^<span class="hljs-string">b`<\/span>/)
		expect(highlightedSourceLines(code, typescript.name, 2)).toEqual(lines.slice(0, 2))
		expect(highlightedSourceLines('a\nb', typescript.name)).toEqual(['a', 'b'])
		expect(highlightedSourceLines('a\nb\n', typescript.name)).toEqual(['a', 'b'])
		expect(highlightedSourceLines('', typescript.name)).toEqual([])

		const large = Array.from({ length: 2_001 }, () => 'const a = 1 < 2').join('\n') + '\n'
		const plain = highlightedSourceLines(large, typescript.name)
		expect(plain).toHaveLength(2_001)
		expect(plain[0]).toBe('const a = 1 &lt; 2')
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
