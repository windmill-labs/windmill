import { describe, expect, it } from 'vitest'
import { summarizeTools } from './agentContext'
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
