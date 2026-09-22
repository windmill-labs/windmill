import { diffLines } from 'diff'
// This is Monaco's advanced diff engine; it supplies the same multi-line inner ranges.
import { DefaultLinesDiffComputer } from '@codingame/monaco-vscode-api/vscode/vs/editor/common/diff/defaultLinesDiffComputer/defaultLinesDiffComputer'
import type { ToolCodeDiff, ToolDisplayMessage } from './shared'

// Builds a diff from a call's own arguments, for the moments the saved `codeDiff` does not
// exist: while the arguments stream, when the call failed, and for transcripts saved before
// calls recorded one. The replaced snippet alone has no language or surrounding lines.
const ARGS_DIFF_BY_TOOL: Record<string, (params: unknown) => ToolCodeDiff | undefined> = {
	edit_script: (params) => argumentDiff(streamingEditArguments(params)),
	edit_code: (params) => {
		if (
			params &&
			typeof params === 'object' &&
			Array.isArray((params as { diffs?: unknown }).diffs)
		) {
			const diffs = (params as { diffs: unknown[] }).diffs
			return argumentDiffs(diffs.map(streamingEditArguments))
		}
		return argumentDiff(streamingEditArguments(params))
	}
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

export function diffLineCounts(diff: ToolCodeDiff): { added: number; removed: number } {
	let added = 0
	let removed = 0
	for (const change of diffLines(diff.before, diff.after)) {
		if (change.added) added += change.count ?? 0
		else if (change.removed) removed += change.count ?? 0
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

function lines(value: string): string[] {
	const result = value.split('\n')
	if (result.at(-1) === '') result.pop()
	return result
}

export function toolDiffLines(diff: ToolCodeDiff): ToolDiffLine[] {
	const result: ToolDiffLine[] = []
	let oldLine = 1
	let newLine = 1
	const { added, removed } = monacoCharacterRanges(diff)

	for (const change of diffLines(diff.before, diff.after)) {
		const kind = change.added ? 'added' : change.removed ? 'removed' : 'context'
		for (const content of lines(change.value)) {
			const lineNumber = kind === 'removed' ? oldLine : newLine
			const changedRanges = kind === 'removed' ? removed.get(lineNumber) : added.get(lineNumber)
			result.push({
				kind,
				content,
				oldLine: kind === 'added' ? undefined : oldLine++,
				newLine: kind === 'removed' ? undefined : newLine++,
				...(changedRanges?.length ? { changedRanges } : {})
			})
		}
	}

	return result
}

function monacoCharacterRanges(diff: ToolCodeDiff): {
	removed: Map<number, CharacterRange[]>
	added: Map<number, CharacterRange[]>
} {
	const before = lines(diff.before)
	const after = lines(diff.after)
	const result = {
		removed: new Map<number, CharacterRange[]>(),
		added: new Map<number, CharacterRange[]>()
	}
	const changes = new DefaultLinesDiffComputer().computeDiff(before, after, {
		ignoreTrimWhitespace: false,
		maxComputationTimeMs: 0,
		computeMoves: false,
		extendToSubwords: false
	})

	for (const change of changes.changes) {
		for (const innerChange of change.innerChanges ?? []) {
			addCharacterRanges(result.removed, before, innerChange.originalRange)
			addCharacterRanges(result.added, after, innerChange.modifiedRange)
		}
	}

	return result
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
