import { describe, it, expect } from 'vitest'
import {
	stripTriggerConfig,
	triggerHandlerRefs,
	type WorkspaceTrigger
} from './workspaceTriggersList'

describe('stripTriggerConfig', () => {
	it('drops instance-side ownership and runtime state but keeps behavior', () => {
		const out = stripTriggerConfig({
			email: 'owner@corp.com',
			edited_by: 'admin',
			is_draft: false,
			paused_until: '2026-01-01',
			enabled: true,
			workspace_id: 'ws',
			extra_perms: {},
			permissioned_as: 'u/admin',
			schedule: '0 0 * * * *',
			cron_version: 'v2',
			no_flow_overlap: true,
			on_failure: 'script/f/p/handler',
			dynamic_skip: 'f/p/skip',
			error_handler_path: 'f/p/handler',
			retry: { constant: { attempts: 1 } }
		})
		expect(Object.keys(out).sort()).toEqual([
			'cron_version',
			'dynamic_skip',
			'error_handler_path',
			'no_flow_overlap',
			'on_failure',
			'retry',
			'schedule'
		])
	})
})

describe('triggerHandlerRefs', () => {
	const trigger = (kind: string, config: Record<string, unknown>): WorkspaceTrigger =>
		({ kind, path: 'f/p/t', script_path: 'f/p/s', is_flow: false, config }) as WorkspaceTrigger

	it('collects schedule handlers including dynamic_skip', () => {
		expect(
			triggerHandlerRefs(
				trigger('schedule', {
					on_failure: 'script/f/p/fail',
					on_recovery: 'flow/f/p/recover',
					dynamic_skip: 'f/p/skip'
				})
			)
		).toEqual([
			{ kind: 'script', path: 'f/p/fail' },
			{ kind: 'flow', path: 'f/p/recover' },
			{ kind: 'script', path: 'f/p/skip' }
		])
	})

	it('collects bare error_handler_path for non-schedule kinds', () => {
		expect(triggerHandlerRefs(trigger('mqtt', { error_handler_path: 'u/admin/handler' }))).toEqual([
			{ kind: 'script', path: 'u/admin/handler' }
		])
	})
})
