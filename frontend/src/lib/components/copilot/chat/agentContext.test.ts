import { describe, expect, it } from 'vitest'
import {
	attachmentStatusLabel,
	attachmentValue,
	contextGlanceLine,
	countReadyAttachments,
	filterTools,
	summarizeTools
} from './agentContext'
import type { Tool } from './shared'

const tool = (name: string, description?: string) =>
	({ def: { type: 'function', function: { name, description } } }) as unknown as Tool<{}>

const NOTHING = {
	tools: 0,
	skills: 0,
	mcpServers: 0,
	attachments: 0,
	instructions: false
}

describe('contextGlanceLine', () => {
	it('drops empty categories and pluralizes the rest', () => {
		expect(
			contextGlanceLine({ ...NOTHING, tools: 64, skills: 1, mcpServers: 2, instructions: true })
		).toBe('64 tools · 1 skill · 2 MCP servers · custom instructions')
	})

	it('says so rather than rendering a row of zeroes', () => {
		expect(contextGlanceLine(NOTHING)).toBe('Nothing yet')
	})
})

describe('filterTools', () => {
	const tools = [
		{ name: 'take_screenshot', description: 'Capture the app preview.' },
		{ name: 'get_db_schema', description: 'Fetch a database resource’s tables.' }
	]

	it('matches descriptions, not just names', () => {
		expect(filterTools(tools, 'database').map((t) => t.name)).toEqual(['get_db_schema'])
	})

	it('returns everything for a blank query', () => {
		expect(filterTools(tools, '  ')).toHaveLength(2)
	})
})

describe('summarizeTools', () => {
	// The panel re-derives this on every turn, so an unsorted list would reshuffle
	// under the reader as tools come and go.
	it('sorts by name and tolerates a tool with no description', () => {
		expect(summarizeTools([tool('run_script', 'Run it.'), tool('deploy')])).toEqual([
			{ name: 'deploy', description: '' },
			{ name: 'run_script', description: 'Run it.' }
		])
	})
})

describe('countReadyAttachments', () => {
	// A folder's own status is an aggregate, and its placeholder row is filtered out
	// of readyFiles() — both directions of that have been got wrong here before.
	it('counts a folder by its readable children, not by its aggregate status', () => {
		const folders = [
			{ files: [{ status: 'ready' }, { status: 'indexing' }] }, // partly readable
			{ files: [] }, // empty or all-binary
			{ files: [{ status: 'error' }] } // nothing readable
		] as const
		expect(countReadyAttachments(folders, [])).toBe(1)
	})

	it('counts standalone files only when ready', () => {
		expect(countReadyAttachments([], [{ status: 'ready' }, { status: 'locked' }])).toBe(1)
	})
})

describe('attachments', () => {
	it('separates what is usable from what is merely attached', () => {
		expect(attachmentValue(0, 0)).toBe('None')
		expect(attachmentValue(3, 3)).toBe('3')
		expect(attachmentValue(1, 3)).toBe('1 of 3')
	})

	it('labels every status the file tools cannot read, and only those', () => {
		expect(attachmentStatusLabel('ready')).toBeUndefined()
		for (const status of ['locked', 'unavailable', 'indexing', 'error'] as const) {
			expect(attachmentStatusLabel(status)).toBeTruthy()
		}
	})
})
