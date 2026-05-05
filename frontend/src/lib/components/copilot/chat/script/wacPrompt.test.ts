import { describe, expect, it } from 'vitest'
import { getScriptPrompt, getWorkflowAsCodePrompt } from '$system_prompts'

describe('Workflow-as-Code prompt helpers', () => {
	it('injects only the TypeScript WAC SDK for TypeScript runtimes', () => {
		const prompt = getWorkflowAsCodePrompt('bun')

		expect(prompt).toContain('Windmill Workflow-as-Code Writing Guide')
		expect(prompt).toContain('## TypeScript Workflow-as-Code API')
		expect(prompt).not.toContain('## Python Workflow-as-Code API')
	})

	it('injects only the Python WAC SDK for Python runtimes', () => {
		const prompt = getWorkflowAsCodePrompt('python3')

		expect(prompt).toContain('Windmill Workflow-as-Code Writing Guide')
		expect(prompt).toContain('## Python Workflow-as-Code API')
		expect(prompt).not.toContain('## TypeScript Workflow-as-Code API')
	})

	it('does not support non-Bun TypeScript runtimes as WAC targets', () => {
		expect(getWorkflowAsCodePrompt('deno')).toBe('')
		expect(getWorkflowAsCodePrompt('nativets')).toBe('')
		expect(getWorkflowAsCodePrompt('bunnative')).toBe('')
	})

	it('does not change normal script prompts', () => {
		expect(getWorkflowAsCodePrompt('go')).toBe('')
		expect(getScriptPrompt('bun')).not.toContain('Windmill Workflow-as-Code Writing Guide')
	})
})
