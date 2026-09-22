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

function partialJsonString(partialJson: string, key: string): string | undefined {
	const match = partialJson.match(new RegExp(`"${key}"\\s*:\\s*"((?:[^"\\\\]|\\\\.)*)`))
	if (!match?.[1]) return undefined

	try {
		return JSON.parse(`"${match[1]}"`)
	} catch {
		return undefined
	}
}

export function hasToolCodeDiff(toolName: string | undefined): boolean {
	return toolName !== undefined && toolName in ARGS_DIFF_BY_TOOL
}

export function toolCodeDiff(message: ToolDisplayMessage): ToolCodeDiff | undefined {
	if (message.codeDiff) return message.codeDiff
	if (!message.toolName) return undefined
	return ARGS_DIFF_BY_TOOL[message.toolName]?.(message.parameters)
}

export function diffLineCounts(diff: ToolCodeDiff, streaming = false): { added: number; removed: number } {
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

export function toolDiffLines(diff: ToolCodeDiff, streaming = false): ToolDiffLine[] {
	const before = lines(diff.before)
	const after = lines(diff.after)
	if (streaming) return streamingDiffLines(before, after)

	const result: ToolDiffLine[] = []
	const monacoBefore = monacoLines(diff.before)
	const monacoAfter = monacoLines(diff.after)
	const changes = new DefaultLinesDiffComputer().computeDiff(monacoBefore, monacoAfter, {
		ignoreTrimWhitespace: false,
		maxComputationTimeMs: 1000,
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
		appendContextLines(result, before, after, oldIndex, newIndex, Math.min(oldStart - oldIndex, newStart - newIndex))
		appendChangedLines(result, 'removed', before, oldStart, change.original.endLineNumberExclusive - 1, removedRanges)
		appendChangedLines(result, 'added', after, newStart, change.modified.endLineNumberExclusive - 1, addedRanges)
		oldIndex = Math.min(change.original.endLineNumberExclusive - 1, before.length)
		newIndex = Math.min(change.modified.endLineNumberExclusive - 1, after.length)
	}
	const trailingContext = Math.max(
		0,
		Math.min(before.length - oldIndex, after.length - newIndex)
	)
	appendContextLines(result, before, after, oldIndex, newIndex, trailingContext)
	oldIndex += trailingContext
	newIndex += trailingContext
	appendChangedLines(result, 'removed', before, oldIndex, before.length, removedRanges)
	appendChangedLines(result, 'added', after, newIndex, after.length, addedRanges)

	return result
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
