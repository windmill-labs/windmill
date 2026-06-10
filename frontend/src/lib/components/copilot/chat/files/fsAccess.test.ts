import { describe, expect, it } from 'vitest'
import { hasFileSystemAccess } from './fsAccess'

describe('hasFileSystemAccess', () => {
	it('is false when the File System Access API is absent (node / Firefox / Safari today)', () => {
		// The test env exposes none of showOpenFilePicker / showDirectoryPicker /
		// DataTransferItem.getAsFileSystemHandle, so the gate must report false.
		// The positive path (all three present) is exercised via browser verification.
		expect(hasFileSystemAccess()).toBe(false)
	})
})
