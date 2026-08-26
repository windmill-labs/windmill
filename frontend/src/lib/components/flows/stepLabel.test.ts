import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import { stepLabel } from './stepLabel'

function mod(id: string, value: any, summary?: string): FlowModule {
	return { id, value, ...(summary ? { summary } : {}) } as FlowModule
}

describe('stepLabel', () => {
	it('prefers the summary over every derived label', () => {
		expect(stepLabel(mod('a', { type: 'forloopflow', parallel: true }, 'Per line item'))).toBe(
			'Per line item'
		)
		expect(stepLabel(mod('failure', { type: 'rawscript', language: 'bun' }, 'Tell Slack'))).toBe(
			'Tell Slack'
		)
	})

	// The ordering that a reorder would silently break: the id checks sit between the composite
	// types and the leaf script types, so these keep their role rather than reading as their body.
	it('names the failure and preprocessor modules by their role, not their script', () => {
		expect(stepLabel(mod('failure', { type: 'rawscript', language: 'bun' }))).toBe('Error handler')
		expect(stepLabel(mod('preprocessor', { type: 'rawscript', language: 'python3' }))).toBe(
			'Preprocessor'
		)
	})

	it('lets a composite type win over the id checks', () => {
		expect(stepLabel(mod('failure', { type: 'branchone' }))).toBe('Run one branch')
	})

	it('appends one parenthesised suffix per set flag, in order', () => {
		expect(
			stepLabel(
				mod('b', { type: 'forloopflow', parallel: true, skip_failures: true, squash: true })
			)
		).toBe('For loop (parallel) (skip failures) (squash)')
		expect(stepLabel(mod('b', { type: 'forloopflow' }))).toBe('For loop')
		expect(stepLabel(mod('c', { type: 'whileloopflow', squash: true }))).toBe('While loop (squash)')
		expect(stepLabel(mod('d', { type: 'branchall', parallel: true }))).toBe(
			'Run all branches (parallel)'
		)
	})

	it('names the leaf types', () => {
		expect(stepLabel(mod('e', { type: 'rawscript', language: 'python3' }))).toBe(
			'Inline python3 script'
		)
		expect(stepLabel(mod('f', { type: 'script' }))).toBe('Workspace script')
		expect(stepLabel(mod('g', { type: 'aiagent' }))).toBe('AI Agent')
		expect(stepLabel(mod('h', { type: 'flow' }))).toBe('Inner flow')
		expect(stepLabel(mod('i', { type: 'identity' }))).toBe('Identity')
	})

	it('returns an empty label for an unknown type rather than throwing', () => {
		expect(stepLabel(mod('j', { type: 'something_new' }))).toBe('')
		expect(stepLabel(mod('k', undefined))).toBe('')
	})
})
