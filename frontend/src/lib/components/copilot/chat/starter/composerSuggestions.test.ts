import { describe, expect, it } from 'vitest'
import { composerSuggestions, type ComposerRole } from './composerSuggestions'

const developer: ComposerRole = { operator: false, workspaceAdmin: false, instanceAdmin: false }
const operator: ComposerRole = { ...developer, operator: true }
const workspaceAdmin: ComposerRole = { ...developer, workspaceAdmin: true }
const instanceAdmin: ComposerRole = { operator: false, workspaceAdmin: true, instanceAdmin: true }

const labels = (...args: Parameters<typeof composerSuggestions>) =>
	composerSuggestions(...args).map((s) => s.label)

describe('composerSuggestions', () => {
	it('offers the build examples in an empty workspace, but not to an operator', () => {
		expect(labels({ itemCount: 0 }, developer)).toEqual([
			'Sync Salesforce',
			'Weekly Slack report',
			'Ban Discord users'
		])
		expect(labels({ itemCount: 0 }, operator)).toEqual([])
	})

	it('puts incidents first, most urgent first, and caps at four', () => {
		const s = composerSuggestions(
			{
				itemCount: 10,
				liveWorkers: 0,
				failedRuns: 1,
				queue: { waiting: 12, suspended: 0 },
				myDraft: { kind: 'flow', path: 'f/a/sync', name: 'Sync' },
				failedDeploy: { kind: 'script', path: 'u/me/etl', name: 'etl' }
			},
			instanceAdmin
		)
		expect(s.map((x) => [x.mode, x.label])).toEqual([
			['use', "No worker has pinged in 5 minutes. What's wrong?"],
			['use', 'Why did a run fail today?'],
			['use', 'Why are 12 jobs waiting?'],
			['build', 'Finish my draft: Sync']
		])
	})

	it("ranks the reader's own work above setup and fillers, as build", () => {
		const s = composerSuggestions(
			{
				itemCount: 10,
				hasErrorHandler: false,
				myDraft: { kind: 'flow', path: 'f/a/sync', name: 'Sync' },
				failedDeploy: { kind: 'script', path: 'u/me/etl', name: 'etl' }
			},
			workspaceAdmin
		)
		expect(s.map((x) => [x.mode, x.label])).toEqual([
			['build', 'Finish my draft: Sync'],
			['build', 'Fix the deploy of etl'],
			['build', 'Set up an error handler for this workspace'],
			['use', 'Which schedules keep failing?']
		])
		expect(s[0].prompt).toContain('f/a/sync')
	})

	it('keeps an operator to what it can use, and admin signals to admins', () => {
		const signals = {
			itemCount: 10,
			liveWorkers: 0,
			hasErrorHandler: false,
			myDraft: { kind: 'flow', path: 'f/a/sync', name: 'Sync' }
		}
		expect(labels(signals, operator)).toEqual(['How healthy were the last runs?'])
		expect(labels(signals, developer)).toEqual([
			'Finish my draft: Sync',
			'How healthy were the last runs?'
		])
	})

	it('gives a quiet workspace fillers for the role', () => {
		expect(labels({ itemCount: 10, liveWorkers: 2, hasErrorHandler: true }, instanceAdmin)).toEqual(
			[
				'Are the workers keeping up with the queue?',
				'Which schedules keep failing?',
				'How healthy were the last runs?'
			]
		)
	})
})
