import { describe, it, expect } from 'vitest'
import { parentFolderKey } from './forkDiffNav'

describe('parentFolderKey', () => {
	describe('folders', () => {
		it('a scope folder (2 segments) has no parent', () => {
			expect(parentFolderKey('folder', 'f/foo')).toBeUndefined()
			expect(parentFolderKey('folder', 'u/alice')).toBeUndefined()
		})
		it('a nested folder belongs to its parent folder', () => {
			expect(parentFolderKey('folder', 'f/foo/sub')).toBe('folder:f/foo')
			expect(parentFolderKey('folder', 'f/foo/sub/deep')).toBe('folder:f/foo/sub')
		})
	})

	describe('files', () => {
		it('a single-segment leaf has no scope folder', () => {
			expect(parentFolderKey('file', 'orphan')).toBeUndefined()
		})
		it('a 2-segment file maps to its scope folder (the cubic fix)', () => {
			// Regression: previously returned `folder:f` (nonexistent) → ArrowLeft broke.
			expect(parentFolderKey('file', 'f/foo')).toBe('folder:f/foo')
		})
		it('a 3-segment file (directly under scope) maps to the scope folder', () => {
			expect(parentFolderKey('file', 'f/foo/bar')).toBe('folder:f/foo')
			expect(parentFolderKey('file', 'u/alice/script')).toBe('folder:u/alice')
		})
		it('a deeper file belongs to its immediate folder', () => {
			expect(parentFolderKey('file', 'f/foo/sub/bar')).toBe('folder:f/foo/sub')
		})
	})
})
