import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
	buildLineIndex,
	readFile,
	searchFiles,
	searchFilesInWorker,
	isTextFile,
	numberLines,
	type FileEntry
} from './fileEngine'

function makeFile(content: string | Uint8Array, name = 'f.txt'): File {
	return new File([content as BlobPart], name)
}

async function makeEntry(content: string, name = 'f.txt'): Promise<FileEntry> {
	const file = makeFile(content, name)
	const { lineIndex, lineCount } = await buildLineIndex(file)
	return { name, file, lineIndex, lineCount }
}

describe('buildLineIndex', () => {
	it('counts lines without a trailing newline', async () => {
		const { lineIndex, lineCount } = await buildLineIndex(makeFile('a\nb'))
		expect(lineCount).toBe(2)
		expect(lineIndex).toEqual([0, 2])
	})

	it('does not count a single trailing newline as an extra line', async () => {
		const { lineIndex, lineCount } = await buildLineIndex(makeFile('a\nb\n'))
		expect(lineCount).toBe(2)
		expect(lineIndex).toEqual([0, 2])
	})

	it('handles an empty file', async () => {
		const { lineIndex, lineCount } = await buildLineIndex(makeFile(''))
		expect(lineCount).toBe(0)
		expect(lineIndex).toEqual([])
	})

	it('handles CRLF line endings (offsets by byte)', async () => {
		// bytes: a=0 \r=1 \n=2 b=3 → line starts at 0 and 3
		const { lineIndex, lineCount } = await buildLineIndex(makeFile('a\r\nb'))
		expect(lineCount).toBe(2)
		expect(lineIndex).toEqual([0, 3])
	})

	it('counts lines correctly across stream chunk boundaries', async () => {
		const lines = Array.from({ length: 5000 }, (_, i) => `line ${i}`)
		const { lineCount } = await buildLineIndex(makeFile(lines.join('\n')))
		expect(lineCount).toBe(5000)
	})
})

describe('readFile', () => {
	it('reads a bounded window and reports pagination', async () => {
		const entry = await makeEntry('l1\nl2\nl3')
		const res = await readFile(entry, { startLine: 1, endLine: 2 })
		expect(res.text).toBe('l1\nl2\n')
		expect(res.startLine).toBe(1)
		expect(res.endLine).toBe(2)
		expect(res.totalLines).toBe(3)
		expect(res.truncated).toBe(true)
		expect(res.note).toContain('start_line=3')
	})

	it('reads the final line to end of file', async () => {
		const entry = await makeEntry('l1\nl2\nl3')
		const res = await readFile(entry, { startLine: 3 })
		expect(res.text).toBe('l3')
		expect(res.truncated).toBe(false)
	})

	it('clamps the window to maxLines', async () => {
		const entry = await makeEntry(Array.from({ length: 100 }, (_, i) => `l${i}`).join('\n'))
		const res = await readFile(entry, { startLine: 1, maxLines: 10 })
		expect(res.endLine).toBe(10)
		expect(res.truncated).toBe(true)
	})

	it('char-caps a degenerate single long line', async () => {
		const entry = await makeEntry('x'.repeat(10000))
		const res = await readFile(entry, { maxChars: 8000 })
		expect(res.text.length).toBe(8000)
		expect(res.truncated).toBe(true)
		expect(res.note).toContain('8000 characters')
	})

	it('bounds the byte decode on a newline-sparse window (never reads past maxChars*4 bytes)', async () => {
		// One 200k-char line; with maxChars=1000 only ≤4000 bytes are sliced before decode.
		const entry = await makeEntry('y'.repeat(200_000))
		const res = await readFile(entry, { maxChars: 1000 })
		expect(res.text).toBe('y'.repeat(1000))
		expect(res.truncated).toBe(true)
	})

	it('clamps a start line beyond the end', async () => {
		const entry = await makeEntry('l1\nl2')
		const res = await readFile(entry, { startLine: 99 })
		expect(res.startLine).toBe(2)
		expect(res.text).toBe('l2')
	})

	it('returns empty for an empty file', async () => {
		const entry = await makeEntry('')
		const res = await readFile(entry)
		expect(res.text).toBe('')
		expect(res.totalLines).toBe(0)
	})

	it('char-truncation inside the first line resumes the note at the next line', async () => {
		// Line 1 exceeds maxChars; lines 2-3 follow. The window only returns line 1's prefix,
		// so the note must report line 1 and point the model at line 2 (not claim lines 1-3).
		const entry = await makeEntry('x'.repeat(20000) + '\nl2\nl3')
		const res = await readFile(entry, { startLine: 1, endLine: 3, maxChars: 5000 })
		expect(res.endLine).toBe(1)
		expect(res.totalLines).toBe(3)
		expect(res.truncated).toBe(true)
		expect(res.note).toContain('start_line=2')
	})

	it('char-truncation after some whole lines resumes at the truncated line', async () => {
		const entry = await makeEntry('a\nb\n' + 'x'.repeat(20000) + '\nd')
		const res = await readFile(entry, { startLine: 1, endLine: 4, maxChars: 5000 })
		expect(res.endLine).toBe(2) // a, b whole; line 3 (xxx) cut
		expect(res.note).toContain('start_line=3')
		// the partial line 3 must NOT leak into the body — it would contradict the note
		expect(res.text).toBe('a\nb\n')
		expect(numberLines(res.text, res.startLine)).toBe('1→a\n2→b')
	})
})

describe('searchFiles', () => {
	it('finds matches with 1-based line numbers', async () => {
		const entry = await makeEntry('alpha\nbeta\ngamma beta')
		const res = await searchFiles([entry], 'beta')
		expect(res.error).toBeUndefined()
		expect(res.hits).toEqual([
			{ file: 'f.txt', line: 2, text: 'beta' },
			{ file: 'f.txt', line: 3, text: 'gamma beta' }
		])
	})

	it('searches across multiple files', async () => {
		const a = await makeEntry('needle here', 'a.txt')
		const b = await makeEntry('nope\nneedle', 'b.txt')
		const res = await searchFiles([a, b], 'needle')
		expect(res.hits.map((h) => `${h.file}:${h.line}`)).toEqual(['a.txt:1', 'b.txt:2'])
	})

	it('restricts to a single file with pathFilter', async () => {
		const a = await makeEntry('needle', 'a.txt')
		const b = await makeEntry('needle', 'b.txt')
		const res = await searchFiles([a, b], 'needle', { pathFilter: 'b.txt' })
		expect(res.hits).toEqual([{ file: 'b.txt', line: 1, text: 'needle' }])
	})

	it('reports an unknown pathFilter as an error', async () => {
		const a = await makeEntry('needle', 'a.txt')
		const res = await searchFiles([a], 'needle', { pathFilter: 'missing.txt' })
		expect(res.error).toContain('missing.txt')
	})

	it('truncates at maxHits', async () => {
		const entry = await makeEntry(Array.from({ length: 10 }, () => 'match').join('\n'))
		const res = await searchFiles([entry], 'match', { maxHits: 3 })
		expect(res.hits.length).toBe(3)
		expect(res.truncated).toBe(true)
	})

	it('supports case-insensitive flags', async () => {
		const entry = await makeEntry('Hello\nworld')
		const res = await searchFiles([entry], 'hello', { flags: 'i' })
		expect(res.hits).toEqual([{ file: 'f.txt', line: 1, text: 'Hello' }])
	})

	it('strips trailing CR from matched CRLF lines', async () => {
		const entry = await makeEntry('foo\r\nbar')
		const res = await searchFiles([entry], 'foo')
		expect(res.hits).toEqual([{ file: 'f.txt', line: 1, text: 'foo' }])
	})

	it('returns a friendly error for an invalid regex', async () => {
		const entry = await makeEntry('anything')
		const res = await searchFiles([entry], '(')
		expect(res.error).toContain('Invalid regex')
		expect(res.hits).toEqual([])
	})

	it('matches across stream chunk boundaries', async () => {
		const lines = Array.from({ length: 5000 }, (_, i) => (i === 4999 ? 'TARGET' : `line ${i}`))
		const entry = await makeEntry(lines.join('\n'))
		const res = await searchFiles([entry], 'TARGET')
		expect(res.hits).toEqual([{ file: 'f.txt', line: 5000, text: 'TARGET' }])
	})

	it('a global flag does not drop matches via a stale lastIndex', async () => {
		// `.test()` is stateful under the `g` flag; without resetting lastIndex, lines after
		// the first match would be tested from a stale offset and silently miss.
		const entry = await makeEntry('match\nmatch\nmatch')
		const res = await searchFiles([entry], 'match', { flags: 'g' })
		expect(res.hits.map((h) => h.line)).toEqual([1, 2, 3])
	})
})

describe('searchFilesInWorker', () => {
	// A controllable stand-in for the search Worker — `reply`, `error`, or `hang` (never responds).
	class MockWorker {
		onmessage: ((e: MessageEvent) => void) | null = null
		onerror: ((e: unknown) => void) | null = null
		static mode: 'reply' | 'error' | 'hang' = 'reply'
		static reply: unknown = { hits: [], truncated: false }
		static terminated = false
		constructor(_url: URL | string, _opts?: unknown) {}
		postMessage(): void {
			if (MockWorker.mode === 'reply')
				queueMicrotask(() => this.onmessage?.({ data: MockWorker.reply } as MessageEvent))
			else if (MockWorker.mode === 'error') queueMicrotask(() => this.onerror?.({}))
			// 'hang' → never responds, exercising the timeout path.
		}
		terminate(): void {
			MockWorker.terminated = true
		}
	}

	beforeEach(() => {
		MockWorker.terminated = false
		vi.stubGlobal('Worker', MockWorker)
	})
	afterEach(() => vi.unstubAllGlobals())

	const entry: FileEntry = { name: 'a.txt', file: makeFile('x'), lineIndex: [], lineCount: 0 }

	it('resolves with the worker result and terminates the worker', async () => {
		MockWorker.mode = 'reply'
		MockWorker.reply = { hits: [{ file: 'a.txt', line: 1, text: 'x' }], truncated: false }
		const res = await searchFilesInWorker([entry], 'x')
		expect(res.hits).toEqual([{ file: 'a.txt', line: 1, text: 'x' }])
		expect(MockWorker.terminated).toBe(true)
	})

	it('times out on a non-responding (pathological) pattern and terminates the worker', async () => {
		MockWorker.mode = 'hang'
		const res = await searchFilesInWorker([entry], '^(a+)+$', {}, 20)
		expect(res.error).toContain('timed out')
		expect(MockWorker.terminated).toBe(true)
	})
})

describe('isTextFile', () => {
	it('accepts UTF-8 text', async () => {
		expect(await isTextFile(makeFile('hello © world'))).toBe(true)
	})

	it('accepts an empty file', async () => {
		expect(await isTextFile(makeFile(''))).toBe(true)
	})

	it('rejects content with NUL bytes', async () => {
		expect(await isTextFile(makeFile(new Uint8Array([104, 0, 105]), 'b.bin'))).toBe(false)
	})
})

describe('numberLines', () => {
	it('prefixes each line with its absolute 1-based number', () => {
		expect(numberLines('l3\nl4\n', 3)).toBe('3→l3\n4→l4')
	})

	it('right-aligns numbers to a common width', () => {
		expect(numberLines('a\nb', 9)).toBe(' 9→a\n10→b')
	})

	it('numbers a final line that has no trailing newline', () => {
		expect(numberLines('only', 5)).toBe('5→only')
	})

	it('matches a readFile window (numbers the returned lines, no phantom line)', async () => {
		const file = makeFile('l1\nl2\nl3')
		const { lineIndex, lineCount } = await buildLineIndex(file)
		const res = await readFile(
			{ name: 'f.txt', file, lineIndex, lineCount },
			{ startLine: 1, endLine: 2 }
		)
		expect(numberLines(res.text, res.startLine)).toBe('1→l1\n2→l2')
	})
})
