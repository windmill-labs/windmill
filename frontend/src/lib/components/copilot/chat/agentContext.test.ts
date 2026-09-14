import { describe, expect, it } from 'vitest'
import { attachmentStatusLabel, countReadyAttachments, summarizeTools } from './agentContext'
import type { Tool } from './shared'

const tool = (name: string, description?: string, parameters?: Record<string, any>) =>
	({ def: { type: 'function', function: { name, description, parameters } } }) as unknown as Tool<{}>

describe('summarizeTools', () => {
	// The modal re-derives this on every turn, so an unsorted list would reshuffle
	// under the reader as tools come and go.
	it('sorts by name and tolerates a tool with no description', () => {
		expect(summarizeTools([tool('run_script', 'Run it.'), tool('deploy')])).toEqual([
			{ name: 'deploy', description: '', parameters: { required: [] } },
			{ name: 'run_script', description: 'Run it.', parameters: { required: [] } }
		])
	})

	// `SchemaViewer` renders no argument table at all without `required`, and a tool
	// whose arguments are all optional legitimately ships without it.
	it('defaults required without dropping the declared schema', () => {
		const [summary] = summarizeTools([
			tool('open_page', 'Open it.', { type: 'object', properties: { page: { type: 'string' } } })
		])
		expect(summary.parameters).toEqual({
			required: [],
			type: 'object',
			properties: { page: { type: 'string' } }
		})
	})
})

describe('countReadyAttachments', () => {
	// A folder's own status is an aggregate, and `readyFiles()` filters out the
	// placeholder row an empty or all-binary folder keeps — so counting on the folder
	// would claim files the assistant cannot open, under a heading about what it can.
	it('counts a folder by its readable children, not its own status', () => {
		const folders = [
			{ files: [{ status: 'indexing' as const }, { status: 'ready' as const }] },
			{ files: [] },
			{ files: [{ status: 'locked' as const }] }
		]
		expect(countReadyAttachments(folders, [])).toBe(1)
	})

	it('counts loose files on their own status', () => {
		const files = [
			{ status: 'ready' as const },
			{ status: 'error' as const },
			{ status: 'ready' as const }
		]
		expect(countReadyAttachments([], files)).toBe(2)
	})
})

describe('attachmentStatusLabel', () => {
	// Every unreadable status has to say why: the file tools operate on `readyFiles()`,
	// so a row that reads like the usable ones is the one place the panel could claim
	// something the assistant cannot open.
	it('labels every status the file tools cannot read, and only those', () => {
		expect(attachmentStatusLabel('ready')).toBeUndefined()
		for (const status of ['locked', 'unavailable', 'indexing', 'error'] as const) {
			expect(attachmentStatusLabel(status)).toBeTruthy()
		}
	})
})
