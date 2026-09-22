import { describe, expect, it } from 'vitest'
import {
	AgentAccessPolicy,
	filterToolsForAgentAccess,
	scopeForWorkspacePath,
	toolAccessRejection
} from './agentAccessPolicy'

function policy(
	baseline: 'selected' | 'deselected',
	overrides: Array<'personal' | 'workspace' | `folder:${string}`>
) {
	return new AgentAccessPolicy('alice', { version: 2, baseline, overrides })
}

describe('AgentAccessPolicy', () => {
	it('maps workspace paths to exact scopes', () => {
		expect(scopeForWorkspacePath('f/finance/report', 'alice')).toBe('folder:finance')
		expect(scopeForWorkspacePath('f/finance_archive/report', 'alice')).toBe(
			'folder:finance_archive'
		)
		expect(scopeForWorkspacePath('$res:u/alice/database', 'alice')).toBe('personal')
		expect(scopeForWorkspacePath('u/alice', 'alice')).toBe('personal')
		expect(scopeForWorkspacePath('u/bob/shared', 'alice')).toBe('workspace')
		expect(scopeForWorkspacePath('g/shared', 'alice')).toBe('workspace')
		expect(scopeForWorkspacePath('f/finance/../people', 'alice')).toBeUndefined()
	})

	it('supports all-except and only-selected policies', () => {
		const allExceptFinance = policy('selected', ['folder:finance'])
		expect(allExceptFinance.allows({ kind: 'workspace_path', path: 'f/finance/report' })).toBe(
			false
		)
		expect(allExceptFinance.allows({ kind: 'workspace_path', path: 'f/engineering/report' })).toBe(
			true
		)

		const financeOnly = policy('deselected', ['folder:finance'])
		expect(financeOnly.allows({ kind: 'workspace_path', path: 'f/finance/report' })).toBe(true)
		expect(financeOnly.allows({ kind: 'workspace_path', path: 'u/alice/report' })).toBe(false)
	})

	it('rejects deselected tool targets before execution', () => {
		const access = policy('deselected', ['folder:finance'])
		expect(
			toolAccessRejection(
				'read_workspace_item',
				{ path: 'f/people/payroll' },
				{
					agentAccessPolicy: access
				}
			)
		).toMatchObject({ label: 'Blocked by context selection' })
		expect(
			toolAccessRejection(
				'read_workspace_item',
				{ path: 'f/finance/report' },
				{
					agentAccessPolicy: access
				}
			)
		).toBeUndefined()
		expect(
			toolAccessRejection(
				'copy_workspace_item',
				{ path: 'f/finance/report', resource_path: 'f/people/report' },
				{ agentAccessPolicy: access }
			)
		).toMatchObject({ label: 'Blocked by context selection' })
	})

	it('withholds generic API tools unless every scope is selected', () => {
		const tools = [
			{ def: { function: { name: 'read_workspace_item' } } },
			{ def: { function: { name: 'call_api_get' } } }
		]
		expect(filterToolsForAgentAccess(tools, policy('selected', ['folder:finance']))).toEqual([
			tools[0]
		])
		expect(filterToolsForAgentAccess(tools, policy('selected', []))).toEqual(tools)
	})

	it('keeps session controls available and gates workspace-wide tools', () => {
		const access = policy('selected', ['folder:finance'])
		expect(
			toolAccessRejection('enter_plan_mode', { reason: 'Research' }, { agentAccessPolicy: access })
		).toBeUndefined()
		expect(
			toolAccessRejection('get_run', { id: 'run-id' }, { agentAccessPolicy: access })
		).toBeUndefined()

		const foldersOnly = policy('deselected', ['folder:finance'])
		expect(
			toolAccessRejection('get_run', { id: 'run-id' }, { agentAccessPolicy: foldersOnly })
		).toMatchObject({ label: 'Blocked by context selection' })
	})

	it('checks nested workspace targets and pipeline folder previews', () => {
		const access = policy('deselected', ['folder:finance'])
		expect(
			toolAccessRejection(
				'write_trigger',
				{ config: { path: '/webhook', script_path: 'f/people/run' } },
				{ agentAccessPolicy: access }
			)
		).toMatchObject({ label: 'Blocked by context selection' })
		expect(
			toolAccessRejection(
				'open_preview',
				{ kind: 'pipeline', path: 'finance' },
				{ agentAccessPolicy: access }
			)
		).toBeUndefined()
		expect(
			toolAccessRejection(
				'get_pipeline_graph',
				{},
				{
					agentAccessPolicy: access,
					pipeline: { getPipelineContext: () => ({ folder: 'finance' }) }
				}
			)
		).toBeUndefined()
	})
})
