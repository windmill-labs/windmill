import { describe, expect, it } from 'vitest'
import { composerSuggestions, type ComposerRole } from './composerSuggestions'

const developer: ComposerRole = { operator: false, workspaceAdmin: false, instanceAdmin: false }
const operator: ComposerRole = { ...developer, operator: true }
const workspaceAdmin: ComposerRole = { ...developer, workspaceAdmin: true }
const instanceAdmin: ComposerRole = { operator: false, workspaceAdmin: true, instanceAdmin: true }

const myDraft = { kind: 'flow', path: 'f/a/sync', name: 'Sync' }
const failedDeploy = { kind: 'script', path: 'u/me/etl', name: 'etl' }
const sharedFlow = { kind: 'flow', path: 'f/billing/charge', name: 'Charge customers' }
const favorite = { kind: 'script', path: 'f/ops/check_ssl', name: 'Check SSL expiry' }
const database = { type: 'postgresql', path: 'f/data/main_db', name: 'main_db' }
const slack = { type: 'slack', path: 'f/team/slack', name: 'slack' }

const labels = (...args: Parameters<typeof composerSuggestions>) =>
	composerSuggestions(...args).map((s) => s.label)

describe('composerSuggestions', () => {
	it('offers the build examples in an empty workspace, but not to an operator', () => {
		expect(labels({ itemCount: 0 }, developer)).toEqual([
			'Daily metrics to Slack',
			'Approve refunds in Slack',
			'Triage tickets to Slack'
		])
		expect(labels({ itemCount: 0 }, operator)).toEqual([])
	})

	it('builds a new workspace from its own connections, naming their paths', () => {
		const s = composerSuggestions({ itemCount: 0, database, channel: slack }, developer)
		expect(s.map((x) => [x.mode, x.label])).toEqual([
			['build', 'Daily main_db summary to Slack'],
			['use', "What's in main_db?"],
			['build', 'Approve refunds in Slack'],
			['build', 'Triage tickets to Slack']
		])
		expect(s[0].prompt).toContain('f/data/main_db')
		expect(s[0].prompt).toContain('f/team/slack')
	})

	it("ranks the reader's own work first, then what the workspace holds, and caps at four", () => {
		expect(
			labels(
				{ itemCount: 25, myDraft, failedDeploy, sharedFlow, favorite, database },
				instanceAdmin
			)
		).toEqual([
			'Finish my draft: Sync',
			'Fix the deploy of etl',
			'Walk me through Charge customers',
			'Run Check SSL expiry'
		])
		expect(labels({ itemCount: 25, sharedFlow, database }, developer)).toEqual([
			'Walk me through Charge customers',
			"What's in main_db?",
			'Check recent runs',
			'Check the run queue'
		])
	})

	it('keeps an operator to running and reading', () => {
		expect(
			labels({ itemCount: 25, myDraft, sharedFlow, favorite, database, channel: slack }, operator)
		).toEqual([
			'Run Check SSL expiry',
			'Walk me through Charge customers',
			'Check recent runs',
			'Check the run queue'
		])
	})

	it('gives each role its checks', () => {
		expect(labels({ itemCount: 10 }, developer)).toEqual([
			'Check recent runs',
			'Check the run queue'
		])
		expect(labels({ itemCount: 10 }, workspaceAdmin)).toEqual([
			'Check the schedules',
			'Check recent runs',
			'Check the run queue'
		])
	})
})
