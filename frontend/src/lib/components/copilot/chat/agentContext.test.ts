import { describe, expect, it } from 'vitest'
import { filterTools, summarizeTools } from './agentContext'
import type { Tool } from './shared'

const tool = (name: string, description?: string) =>
	({ def: { type: 'function', function: { name, description } } }) as unknown as Tool<{}>

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
	// The modal re-derives this on every turn, so an unsorted list would reshuffle
	// under the reader as tools come and go.
	it('sorts by name and tolerates a tool with no description', () => {
		expect(summarizeTools([tool('run_script', 'Run it.'), tool('deploy')])).toEqual([
			{ name: 'deploy', description: '' },
			{ name: 'run_script', description: 'Run it.' }
		])
	})
})
