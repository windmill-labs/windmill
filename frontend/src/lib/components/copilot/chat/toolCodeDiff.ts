// This is Monaco's advanced diff engine; it supplies the same multi-line inner ranges.
import { DefaultLinesDiffComputer } from '@codingame/monaco-vscode-api/vscode/vs/editor/common/diff/defaultLinesDiffComputer/defaultLinesDiffComputer'
import type { ToolCodeDiff, ToolDisplayMessage } from './shared'

// Builds a diff from a call's own arguments, for the moments the saved `codeDiff` does not
// exist: while the arguments stream, when the call failed, and for transcripts saved before
// calls recorded one. Argument diffs have no language or surrounding lines.
const ARGS_DIFF_BY_TOOL: Record<string, (params: unknown) => ToolCodeDiff | undefined> = {
	edit_script: (params) => argumentDiff(streamingEditArguments(params)),
	write_script: (params) => fullContentArgumentDiff(params, 'content'),
	edit_code: (params) => {
		if (
			params &&
			typeof params === 'object' &&
			Array.isArray((params as { diffs?: unknown }).diffs)
		) {
			const diffs = (params as { diffs: unknown[] }).diffs
			return argumentDiffs(diffs.map(streamingEditArguments))
		}
		if (typeof params === 'string') {
			return (
				argumentDiffs(streamingEditArgumentsArray(params)) ??
				fullContentArgumentDiff(params, 'code')
			)
		}
		return argumentDiff(streamingEditArguments(params)) ?? fullContentArgumentDiff(params, 'code')
	}
}

function fullContentArgumentDiff(params: unknown, key: string): ToolCodeDiff | undefined {
	const content =
		params && typeof params === 'object'
			? (params as Record<string, unknown>)[key]
			: typeof params === 'string'
				? partialJsonString(params, key)
				: undefined
	return typeof content === 'string' ? { before: '', after: content, lang: 'plaintext' } : undefined
}

function argumentDiff(
	edit: { old_string?: string; new_string?: string } | undefined
): ToolCodeDiff | undefined {
	return typeof edit?.old_string === 'string' || typeof edit?.new_string === 'string'
		? {
				before: typeof edit.old_string === 'string' ? edit.old_string : '',
				after: typeof edit.new_string === 'string' ? edit.new_string : '',
				lang: 'plaintext'
			}
		: undefined
}

function argumentDiffs(
	edits: ({ old_string?: string; new_string?: string } | undefined)[]
): ToolCodeDiff | undefined {
	const diffs = edits.map(argumentDiff).filter((diff): diff is ToolCodeDiff => diff !== undefined)
	if (diffs.length === 0) return undefined
	return {
		before: diffs.map((diff) => diff.before).join('\n'),
		after: diffs.map((diff) => diff.after).join('\n'),
		lang: 'plaintext'
	}
}

function streamingEditArguments(
	params: unknown
): { old_string?: string; new_string?: string } | undefined {
	if (params && typeof params === 'object')
		return params as { old_string?: string; new_string?: string }
	if (typeof params !== 'string') return undefined

	return {
		old_string: partialJsonString(params, 'old_string'),
		new_string: partialJsonString(params, 'new_string')
	}
}

function streamingEditArgumentsArray(
	partialJson: string
): { old_string?: string; new_string?: string }[] {
	const oldStrings = partialJsonStrings(partialJson, 'old_string')
	const newStrings = partialJsonStrings(partialJson, 'new_string')
	return Array.from({ length: Math.max(oldStrings.length, newStrings.length) }, (_, index) => ({
		old_string: oldStrings[index],
		new_string: newStrings[index]
	}))
}

function partialJsonString(partialJson: string, key: string): string | undefined {
	return partialJsonStrings(partialJson, key)[0]
}

function partialJsonStrings(partialJson: string, key: string): string[] {
	const matches = partialJson.matchAll(new RegExp(`"${key}"\\s*:\\s*"((?:[^"\\\\]|\\\\.)*)`, 'g'))
	const result: string[] = []
	// An empty string is a real value (a deletion): skipping it would pair the next edit's
	// `new_string` with this edit's `old_string`.
	for (const match of matches) {
		try {
			result.push(JSON.parse(`"${match[1]}"`))
		} catch {
			continue
		}
	}
	return result
}

export function hasToolCodeDiff(toolName: string | undefined): boolean {
	return toolName !== undefined && Object.hasOwn(ARGS_DIFF_BY_TOOL, toolName)
}

export function toolCodeDiff(message: ToolDisplayMessage): ToolCodeDiff | undefined {
	if (message.codeDiff) return message.codeDiff
	if (!message.toolName) return undefined
	return Object.hasOwn(ARGS_DIFF_BY_TOOL, message.toolName)
		? ARGS_DIFF_BY_TOOL[message.toolName](message.parameters)
		: undefined
}

// Decided by tool and argument shape, never by the saved `codeDiff`: an overwrite's saved diff
// has a before side, and clearing a script leaves both argument sides empty, yet both calls sent
// a whole file.
export function argumentsCarryWholeFile(message: ToolDisplayMessage): boolean {
	if (message.toolName === 'write_script') return true
	if (message.toolName !== 'edit_code') return false
	const params = message.parameters
	if (typeof params === 'string') return /"code"\s*:/.test(params)
	return Boolean(params && typeof params === 'object' && 'code' in params)
}

export function diffLineCounts(
	diff: ToolCodeDiff,
	streaming = false
): { added: number; removed: number } {
	if (streaming) {
		return { added: lines(diff.after).length, removed: lines(diff.before).length }
	}

	return toolDiffLineCounts(toolDiffLines(diff))
}

export function toolDiffLineCounts(lines: ToolDiffLine[]): { added: number; removed: number } {
	let added = 0
	let removed = 0
	for (const line of lines) {
		if (line.kind === 'added') added++
		else if (line.kind === 'removed') removed++
	}
	return { added, removed }
}

export type ToolDiffLine = {
	kind: 'context' | 'added' | 'removed'
	content: string
	oldLine?: number
	newLine?: number
	changedRanges?: CharacterRange[]
}

export type CharacterRange = { start: number; length: number; extendsToEnd?: boolean }

const STREAMING_PREVIEW_LINES = 200

function lines(value: string): string[] {
	const result = value.split('\n')
	if (result.at(-1) === '') result.pop()
	return result
}

function hasFinalNewline(value: string): boolean {
	return value.endsWith('\n')
}

export function toolDiffLines(diff: ToolCodeDiff, streaming = false): ToolDiffLine[] {
	const before = lines(diff.before)
	const after = lines(diff.after)
	if (streaming) return streamingDiffLines(before, after)

	const result: ToolDiffLine[] = []
	const monacoBefore = monacoLines(diff.before)
	const monacoAfter = monacoLines(diff.after)
	// Runs on the UI thread. Cost follows the amount changed, not file size: only a large
	// rewrite reaches the cap, and Monaco then reports the whole file as one change.
	const changes = new DefaultLinesDiffComputer().computeDiff(monacoBefore, monacoAfter, {
		ignoreTrimWhitespace: false,
		maxComputationTimeMs: 200,
		computeMoves: false,
		extendToSubwords: false
	}).changes
	const removedRanges = new Map<number, CharacterRange[]>()
	const addedRanges = new Map<number, CharacterRange[]>()
	for (const change of changes) {
		for (const innerChange of change.innerChanges ?? []) {
			addCharacterRanges(removedRanges, monacoBefore, innerChange.originalRange)
			addCharacterRanges(addedRanges, monacoAfter, innerChange.modifiedRange)
		}
	}

	let oldIndex = 0
	let newIndex = 0
	for (const change of changes) {
		const oldStart = change.original.startLineNumber - 1
		const newStart = change.modified.startLineNumber - 1
		appendContextLines(
			result,
			before,
			after,
			oldIndex,
			newIndex,
			Math.min(oldStart - oldIndex, newStart - newIndex)
		)
		appendChangedLines(
			result,
			'removed',
			before,
			oldStart,
			change.original.endLineNumberExclusive - 1,
			removedRanges
		)
		appendChangedLines(
			result,
			'added',
			after,
			newStart,
			change.modified.endLineNumberExclusive - 1,
			addedRanges
		)
		oldIndex = Math.min(change.original.endLineNumberExclusive - 1, before.length)
		newIndex = Math.min(change.modified.endLineNumberExclusive - 1, after.length)
	}
	const trailingContext = Math.max(0, Math.min(before.length - oldIndex, after.length - newIndex))
	appendContextLines(result, before, after, oldIndex, newIndex, trailingContext)
	oldIndex += trailingContext
	newIndex += trailingContext
	appendChangedLines(result, 'removed', before, oldIndex, before.length, removedRanges)
	appendChangedLines(result, 'added', after, newIndex, after.length, addedRanges)
	if (
		diff.before !== '' &&
		diff.after !== '' &&
		hasFinalNewline(diff.before) !== hasFinalNewline(diff.after)
	) {
		result.push(
			hasFinalNewline(diff.before)
				? { kind: 'removed', content: '', oldLine: before.length + 1 }
				: { kind: 'added', content: '', newLine: after.length + 1 }
		)
	}

	return result
}

export type VisibleToolDiffRow =
	| ToolDiffLine
	| { kind: 'collapsed'; key: string; count: number }
	| { kind: 'omitted'; key: string; count: number }

const CONTEXT_LINES = 3
// Every visible row is a DOM row with no virtualization, and only context collapses: a whole-file
// rewrite would otherwise render every line of both sides. Two capped runs fit under the overall
// cap, so a rewrite still shows its added side. Each expansion of an omitted row reveals one more
// cap's worth, so no single click renders an unbounded number of rows.
const MAX_CHANGED_RUN_ROWS = 400
const MAX_VISIBLE_ROWS = 1_000

// `expansions` counts the clicks on each collapsed or omitted row, by its key.
export function visibleToolDiffRows(
	lines: ToolDiffLine[],
	expansions: ReadonlyMap<string, number>
): VisibleToolDiffRow[] {
	const result: VisibleToolDiffRow[] = []
	for (let index = 0; index < lines.length; ) {
		const start = index
		const kind = lines[start].kind
		while (index < lines.length && lines[index].kind === kind) index++
		const count = index - start

		if (kind !== 'context') {
			const key = `${kind}:${start}`
			const shown = MAX_CHANGED_RUN_ROWS * (1 + (expansions.get(key) ?? 0))
			result.push(...lines.slice(start, Math.min(index, start + shown)))
			if (count > shown) result.push({ kind: 'omitted', key, count: count - shown })
			continue
		}

		const key = `${lines[start].oldLine}:${lines[start].newLine}:${count}`
		if (count <= CONTEXT_LINES * 2 + 1 || expansions.has(key)) {
			result.push(...lines.slice(start, index))
			continue
		}
		result.push(...lines.slice(start, start + CONTEXT_LINES))
		result.push({ kind: 'collapsed', key, count: count - CONTEXT_LINES * 2 })
		result.push(...lines.slice(index - CONTEXT_LINES, index))
	}

	const maxRows = MAX_VISIBLE_ROWS * (1 + (expansions.get('end') ?? 0))
	if (result.length <= maxRows) return result
	const hidden = result
		.slice(maxRows)
		.reduce((total, row) => total + ('count' in row ? row.count : 1), 0)
	return [...result.slice(0, maxRows), { kind: 'omitted', key: 'end', count: hidden }]
}

function streamingDiffLines(before: string[], after: string[]): ToolDiffLine[] {
	return [
		...before
			.slice(0, STREAMING_PREVIEW_LINES)
			.map((content, index) => ({ kind: 'removed' as const, content, oldLine: index + 1 })),
		...after
			.slice(0, STREAMING_PREVIEW_LINES)
			.map((content, index) => ({ kind: 'added' as const, content, newLine: index + 1 }))
	]
}

function monacoLines(value: string): string[] {
	const result = lines(value)
	return result.length ? result : ['']
}

function appendContextLines(
	result: ToolDiffLine[],
	before: string[],
	after: string[],
	oldStart: number,
	newStart: number,
	count: number
): void {
	for (let index = 0; index < count; index++) {
		result.push({
			kind: 'context',
			content: before[oldStart + index],
			oldLine: oldStart + index + 1,
			newLine: newStart + index + 1
		})
	}
}

function appendChangedLines(
	result: ToolDiffLine[],
	kind: 'added' | 'removed',
	source: string[],
	start: number,
	end: number,
	ranges: Map<number, CharacterRange[]>
): void {
	for (let index = start; index < Math.min(end, source.length); index++) {
		const changedRanges = ranges.get(index + 1)
		result.push({
			kind,
			content: source[index],
			...(kind === 'added' ? { newLine: index + 1 } : { oldLine: index + 1 }),
			...(changedRanges?.length ? { changedRanges } : {})
		})
	}
}

function addCharacterRanges(
	ranges: Map<number, CharacterRange[]>,
	lines: string[],
	range: {
		startLineNumber: number
		startColumn: number
		endLineNumber: number
		endColumn: number
	}
): void {
	for (
		let lineNumber = range.startLineNumber;
		lineNumber <= Math.min(range.endLineNumber, lines.length);
		lineNumber++
	) {
		const content = lines[lineNumber - 1]
		const start = lineNumber === range.startLineNumber ? range.startColumn - 1 : 0
		const extendsToEnd = lineNumber < range.endLineNumber
		const end = extendsToEnd ? content.length : range.endColumn - 1
		if (end > start) {
			ranges.set(lineNumber, [
				...(ranges.get(lineNumber) ?? []),
				{ start, length: end - start, ...(extendsToEnd ? { extendsToEnd } : {}) }
			])
		}
	}
}
