import { describe, expect, it } from 'vitest'
import { hasFileSystemAccess, isIgnoredPath } from './fsAccess'

describe('hasFileSystemAccess', () => {
	it('is false when the File System Access API is absent (node / Firefox / Safari today)', () => {
		// The test env exposes none of showOpenFilePicker / showDirectoryPicker /
		// DataTransferItem.getAsFileSystemHandle, so the gate must report false.
		// The positive path (all three present) is exercised via browser verification.
		expect(hasFileSystemAccess()).toBe(false)
	})
})

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
