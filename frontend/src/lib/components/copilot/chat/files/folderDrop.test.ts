import { describe, expect, it } from 'vitest'
import { isIgnoredPath, filterFolderPickerFiles, collectDroppedEntries } from './folderDrop'

describe('isIgnoredPath', () => {
	it('keeps normal source paths', () => {
		expect(isIgnoredPath('myproj/src/app.ts')).toBe(false)
		expect(isIgnoredPath('README.md')).toBe(false)
	})
	it('skips ignored directories', () => {
		expect(isIgnoredPath('myproj/node_modules/lib/index.js')).toBe(true)
		expect(isIgnoredPath('myproj/dist/bundle.js')).toBe(true)
		expect(isIgnoredPath('a/target/x')).toBe(true)
	})
	it('skips dotfiles and dotdirs', () => {
		expect(isIgnoredPath('myproj/.env')).toBe(true)
		expect(isIgnoredPath('myproj/.git/config')).toBe(true)
		expect(isIgnoredPath('.DS_Store')).toBe(true)
	})
})

function fileWithRelPath(name: string, relPath: string): File {
	const f = new File(['x'], name)
	Object.defineProperty(f, 'webkitRelativePath', { value: relPath })
	return f
}

describe('filterFolderPickerFiles', () => {
	it('keeps text-path files and drops junk, naming by relative path', () => {
		const files = [
			fileWithRelPath('app.ts', 'proj/src/app.ts'),
			fileWithRelPath('index.js', 'proj/node_modules/x/index.js'),
			fileWithRelPath('.env', 'proj/.env')
		] as unknown as FileList
		const out = filterFolderPickerFiles(files)
		expect(out.map((i: any) => i.path)).toEqual(['proj/src/app.ts'])
	})
})

// Minimal FileSystemEntry mocks mirroring the browser shape.
function fileEntry(name: string, content: string): any {
	return {
		isFile: true,
		isDirectory: false,
		name,
		file: (cb: (f: File) => void) => cb(new File([content], name))
	}
}
function dirEntry(name: string, children: any[]): any {
	let done = false
	return {
		isFile: false,
		isDirectory: true,
		name,
		createReader: () => ({
			readEntries: (cb: (e: any[]) => void) => {
				if (done) cb([])
				else {
					done = true
					cb(children)
				}
			}
		})
	}
}

describe('collectDroppedEntries', () => {
	it('recurses folders, applies relative paths, and filters junk', async () => {
		const root = dirEntry('myproj', [
			fileEntry('README.md', '# t\nhi\n'),
			dirEntry('src', [fileEntry('app.ts', 'const x=1\n')]),
			dirEntry('node_modules', [fileEntry('index.js', 'junk\n')]),
			fileEntry('.env', 'SECRET=1\n')
		])
		const out = await collectDroppedEntries([root as unknown as FileSystemEntry])
		expect(out.map((i: any) => i.path).sort()).toEqual(['myproj/README.md', 'myproj/src/app.ts'])
	})

	it('keeps a top-level dropped file even if its name is dot-prefixed', async () => {
		const out = await collectDroppedEntries([
			fileEntry('.env', 'SECRET=1\n') as unknown as FileSystemEntry
		])
		expect(out.map((i: any) => i.path)).toEqual(['.env'])
	})
})
