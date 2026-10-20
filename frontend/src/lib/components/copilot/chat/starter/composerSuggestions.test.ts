import { describe, expect, it } from 'vitest'
import { composerSuggestions, type ComposerRole } from './composerSuggestions'

const developer: ComposerRole = { operator: false, workspaceAdmin: false, instanceAdmin: false }
const operator: ComposerRole = { ...developer, operator: true }
const workspaceAdmin: ComposerRole = { ...developer, workspaceAdmin: true }

const myDraft = { kind: 'flow', path: 'f/a/sync', name: 'Sync' }
const sharedFlow = { kind: 'flow', path: 'f/billing/charge', name: 'Charge customers' }
const favorite = { kind: 'script', path: 'f/ops/check_ssl', name: 'Check SSL expiry' }
const script = favorite
const app = { kind: 'app', path: 'f/ops/console', name: 'Ops console' }
const folderItem = sharedFlow
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

	it("teaches the piece a workspace is missing first, as the reader's role allows", () => {
		const fresh = { itemCount: 0, dataTables: [], database: null, channel: null }
		expect(labels(fresh, workspaceAdmin)).toEqual([
			'Set up a data table',
			'Daily metrics to Slack',
			'Approve refunds in Slack',
			'Triage tickets to Slack'
		])
		expect(labels(fresh, developer)[0]).toBe('How do I store data?')
		expect(labels({ ...fresh, dataTables: ['main'] }, developer)[0]).toBe('Connect Slack')
		// Past its first items a workspace may keep data and Slack where these lists don't look.
		const established = { ...fresh, itemCount: 25 }
		expect(labels(established, workspaceAdmin)[0]).toBe('Set up a data table')
		expect(labels(established, developer)).not.toContain('How do I store data?')
		expect(labels(established, developer)).not.toContain('Connect Slack')
	})

	it('builds a new workspace from its own connections, naming their paths', () => {
		const s = composerSuggestions(
			{ itemCount: 0, dataTables: ['main'], database, channel: slack },
			developer
		)
		expect(s.map((x) => [x.mode, x.label])).toEqual([
			['build', 'Daily main_db summary to Slack'],
			['use', "What's in the main data table?"],
			['build', 'Build an app on the main data table'],
			['use', 'Post a Slack message from a script']
		])
		expect(s[0].prompt).toContain('f/data/main_db')
		expect(s[0].prompt).toContain('f/team/slack')
	})

	it("ranks the reader's own work first, then what the workspace holds, once per concept", () => {
		const established = {
			itemCount: 25,
			sharedFlow,
			favorite,
			script,
			app,
			folderItem,
			database,
			channel: slack,
			dataTables: ['main']
		}
		expect(labels({ ...established, myDraft }, workspaceAdmin)).toEqual([
			'Finish my draft: Sync',
			"What's in the main data table?",
			'Walk me through Charge customers',
			'Run Check SSL expiry'
		])
		expect(labels(established, developer)).toEqual([
			"What's in the main data table?",
			'Walk me through Charge customers',
			'Run Check SSL expiry',
			'Add a page to Ops console'
		])
	})

	it('keeps an operator to running and reading', () => {
		expect(
			labels(
				{ itemCount: 25, myDraft, sharedFlow, favorite, folderItem, dataTables: [], channel: null },
				operator
			)
		).toEqual([
			'Run Check SSL expiry',
			'Walk me through Charge customers',
			'What can I run here?',
			'Who can see Charge customers?'
		])
	})

	it('fills with the checks for the role', () => {
		expect(labels({ itemCount: 10 }, workspaceAdmin)).toEqual([
			'Check the schedules',
			'Check recent runs',
			'Check the run queue',
			'Use my scripts from Claude'
		])
	})
})
