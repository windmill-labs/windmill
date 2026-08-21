import { beforeEach, describe, expect, it } from 'vitest'

import { clearParkedImport, parkImport, readParkedImport, resumableImport } from './parking'

// A run that skips its create when it should not have imports a project into a workspace
// somebody else owns, so the match has to be exact and a damaged entry has to read as
// nothing parked rather than as a partial match.

describe('resumableImport', () => {
	beforeEach(() => clearParkedImport())

	it('resumes the run that parked it', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		expect(resumableImport('calendly', 'calendly-7')).toBe(true)
	})

	it('does not resume another project parked at the same workspace', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		expect(resumableImport('bitly', 'calendly-7')).toBe(false)
	})

	it('does not resume the same project aimed at another workspace', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		expect(resumableImport('calendly', 'calendly-8')).toBe(false)
	})

	it('does not resume once cleared', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		clearParkedImport()
		expect(resumableImport('calendly', 'calendly-7')).toBe(false)
	})

	it('reads a damaged entry as nothing parked', () => {
		sessionStorage.setItem('import_wizard_parked', '{"slug":"calendly"')
		expect(readParkedImport()).toBeUndefined()
		sessionStorage.setItem('import_wizard_parked', '{"slug":"calendly"}')
		expect(readParkedImport()).toBeUndefined()
	})
})
