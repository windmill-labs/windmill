import { describe, expect, it } from 'vitest'
import {
	attachmentStatusLabel,
	attachmentValue,
	contextGlanceLine,
	filterTools
} from './agentContext'

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
