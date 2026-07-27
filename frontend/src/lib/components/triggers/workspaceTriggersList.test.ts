import { describe, it, expect } from 'vitest'
import {
	portableTriggerConfig,
	triggerHandlerRefs,
	type WorkspaceTrigger
} from './workspaceTriggersList'

const trigger = (kind: string, config: Record<string, unknown>): WorkspaceTrigger =>
	({ kind, path: 'f/p/t', script_path: 'f/p/s', is_flow: false, config }) as WorkspaceTrigger

describe('portableTriggerConfig', () => {
	it('is an allowlist: unknown and instance-side fields never cross the boundary', () => {
		const out = portableTriggerConfig('schedule', {
			// portable
			schedule: '0 0 * * * *',
			cron_version: 'v2',
			no_flow_overlap: true,
			on_failure: 'script/f/p/handler',
			dynamic_skip: 'f/p/skip',
			retry: { constant: { attempts: 1 } },
			// instance-side / identity — must all be dropped
			email: 'owner@corp.com',
			edited_by: 'admin',
			is_draft: false,
			paused_until: '2026-01-01',
			enabled: true,
			workspace_id: 'ws',
			extra_perms: {},
			permissioned_as: 'u/admin',
			tag: 'gpu-worker',
			// a field upstream might add tomorrow — dropped until admitted
			some_future_field: 'x'
		})
		expect(Object.keys(out).sort()).toEqual([
			'cron_version',
			'dynamic_skip',
			'no_flow_overlap',
			'on_failure',
			'retry',
			'schedule'
		])
	})

	it('appends shared behavior fields for non-schedule kinds', () => {
		const out = portableTriggerConfig('mqtt', {
			mqtt_resource_path: 'f/p/broker',
			error_handler_path: 'f/p/handler',
			retry: {},
			permissioned_as: 'u/admin',
			server_id: 'srv-1'
		})
		expect(Object.keys(out).sort()).toEqual(['error_handler_path', 'mqtt_resource_path', 'retry'])
	})

	it('returns empty for unknown kinds or missing config', () => {
		expect(portableTriggerConfig('nope', { a: 1 })).toEqual({})
		expect(portableTriggerConfig('mqtt', null)).toEqual({})
	})
})

describe('triggerHandlerRefs', () => {
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

	it('collects websocket runnable url and initial-message runnables', () => {
		expect(
			triggerHandlerRefs(
				trigger('websocket', {
					url: '$script:f/p/url_builder',
					initial_messages: [
						{ raw_message: 'hi' },
						{ runnable_result: { path: 'f/p/greeter', args: {}, is_flow: true } }
					]
				})
			)
		).toEqual([
			{ kind: 'script', path: 'f/p/url_builder' },
			{ kind: 'flow', path: 'f/p/greeter' }
		])
	})
})

describe('triggerHandlerRefs edge cases', () => {
	it('returns empty when no handlers are configured', () => {
		expect(triggerHandlerRefs(trigger('schedule', { schedule: '0 0 * * * *', timezone: 'UTC' }))).toEqual([])
		expect(triggerHandlerRefs(trigger('mqtt', { mqtt_resource_path: 'f/p/broker' }))).toEqual([])
	})

	it('collects on_success handlers', () => {
		expect(triggerHandlerRefs(trigger('schedule', { on_success: 'script/f/p/notify' }))).toEqual([
			{ kind: 'script', path: 'f/p/notify' }
		])
	})
})
