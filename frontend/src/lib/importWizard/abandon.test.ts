import { beforeEach, describe, expect, it } from 'vitest'

import { clearParkedImport, parkImport, resumableImport } from './parking'

/**
 * Leaving mid-run is the one case where finishing and *stopping* disagree about the parked
 * workspace. A run that clears parking on its way out makes the link the user was told to
 * come back to create the workspace a second time — and fail, because it already exists.
 *
 * `ImportExecution` needs a live API to construct, so this covers the decision itself rather
 * than the class: what `#import`'s tail does with parking, given whether the user left.
 */
function finish(opts: { abandoned: boolean }): void {
	if (!opts.abandoned) clearParkedImport()
}

describe('parking across the end of a run', () => {
	beforeEach(() => clearParkedImport())

	it('clears the parked workspace when the run finishes normally', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		finish({ abandoned: false })
		// A later import of the same project must reach its own create rather than adopt
		// the workspace this run made.
		expect(resumableImport('calendly', 'calendly-7')).toBe(false)
	})

	it('keeps it when the user left mid-run, so the link still resumes', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		finish({ abandoned: true })
		expect(resumableImport('calendly', 'calendly-7')).toBe(true)
	})

	it('still scopes the resume to the project that parked it', () => {
		parkImport({ slug: 'calendly', workspaceId: 'calendly-7' })
		finish({ abandoned: true })
		// Abandoning must not turn the parked entry into a workspace any project can adopt.
		expect(resumableImport('bitly', 'calendly-7')).toBe(false)
		expect(resumableImport('calendly', 'bitly-1')).toBe(false)
	})
})
