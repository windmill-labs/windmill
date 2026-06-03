import { describe, expect, it } from 'vitest'
import type { Flow, NewScript, Script } from '$lib/gen/types.gen'
import { buildFlowDeployRequestBody, buildScriptDeployRequestBody } from './deployRequests'
import type { WorkspaceItem } from './workspaceItems'

describe('global AI deploy request builders', () => {
	it('preserves existing script metadata while replacing draft-controlled fields', () => {
		const existing = {
			hash: 'parent-hash',
			path: 'f/demo/script',
			summary: 'existing summary',
			description: 'existing description',
			content: 'old content',
			schema: { properties: { name: { type: 'string' } } },
			is_template: true,
			language: 'bun',
			kind: 'script',
			tag: 'node',
			envs: ['ENV_A'],
			concurrent_limit: 3,
			concurrency_time_window_s: 60,
			concurrency_key: 'key',
			debounce_key: 'debounce',
			debounce_delay_s: 5,
			debounce_args_to_accumulate: ['ids'],
			max_total_debouncing_time: 120,
			max_total_debounces_amount: 4,
			cache_ttl: 30,
			cache_ignore_s3_path: true,
			dedicated_worker: true,
			ws_error_handler_muted: true,
			priority: 9,
			restart_unless_cancelled: true,
			timeout: 300,
			delete_after_secs: 600,
			visible_to_runner_only: true,
			auto_kind: 'script',
			codebase: 'repo',
			has_preprocessor: true,
			on_behalf_of_email: 'deployer@example.com',
			assets: [{ path: 's3://bucket/key', kind: 's3object' }],
			modules: { 'helper.ts': { content: 'export const helper = 1', language: 'bun' } },
			labels: ['prod'],
			lock: 'stale lock'
		} as unknown as Script & Partial<NewScript>
		const draft: WorkspaceItem = {
			type: 'script',
			path: 'f/demo/script',
			summary: 'draft summary',
			language: 'bun',
			value: 'new content',
			isDraft: true
		}

		const requestBody = buildScriptDeployRequestBody('f/demo/script', draft, existing, 'ai deploy')

		expect(requestBody).toMatchObject({
			path: 'f/demo/script',
			parent_hash: 'parent-hash',
			summary: 'draft summary',
			description: 'existing description',
			content: 'new content',
			schema: existing.schema,
			tag: 'node',
			envs: ['ENV_A'],
			concurrent_limit: 3,
			concurrency_time_window_s: 60,
			concurrency_key: 'key',
			debounce_key: 'debounce',
			debounce_delay_s: 5,
			debounce_args_to_accumulate: ['ids'],
			max_total_debouncing_time: 120,
			max_total_debounces_amount: 4,
			cache_ttl: 30,
			cache_ignore_s3_path: true,
			dedicated_worker: true,
			ws_error_handler_muted: true,
			priority: 9,
			restart_unless_cancelled: true,
			timeout: 300,
			delete_after_secs: 600,
			visible_to_runner_only: true,
			auto_kind: 'script',
			codebase: 'repo',
			has_preprocessor: true,
			on_behalf_of_email: 'deployer@example.com',
			preserve_on_behalf_of: true,
			assets: existing.assets,
			modules: existing.modules,
			labels: ['prod'],
			deployment_message: 'ai deploy'
		})
		expect(requestBody.lock).toBeUndefined()
	})

	it('preserves existing flow metadata and uses draft value/schema overrides', () => {
		const existing = {
			path: 'f/demo/flow',
			summary: 'existing summary',
			description: 'existing description',
			value: { modules: [] },
			schema: { required: ['name'] },
			tag: 'python',
			ws_error_handler_muted: true,
			priority: 7,
			dedicated_worker: true,
			timeout: 60,
			visible_to_runner_only: true,
			on_behalf_of_email: 'deployer@example.com',
			labels: ['critical']
		} as unknown as Flow
		const draftValue = {
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] },
			schema: { properties: { name: { type: 'string' } } },
			groups: [{ start_id: 'step', end_id: 'step', summary: 'Group' }]
		}

		const requestBody = buildFlowDeployRequestBody(
			'f/demo/flow',
			undefined,
			draftValue as any,
			existing,
			'ai deploy'
		)

		expect(requestBody).toMatchObject({
			path: 'f/demo/flow',
			summary: 'existing summary',
			description: 'existing description',
			schema: draftValue.schema,
			tag: 'python',
			ws_error_handler_muted: true,
			priority: 7,
			dedicated_worker: true,
			timeout: 60,
			visible_to_runner_only: true,
			on_behalf_of_email: 'deployer@example.com',
			preserve_on_behalf_of: true,
			labels: ['critical'],
			deployment_message: 'ai deploy'
		})
		expect(requestBody.value.modules).toHaveLength(1)
		expect(requestBody.value.groups).toEqual(draftValue.groups)
	})

	it('falls back to existing flow schema when the draft has no schema', () => {
		const existing = {
			path: 'f/demo/flow',
			summary: 'existing summary',
			value: { modules: [] },
			schema: { properties: { existing: { type: 'boolean' } } }
		} as unknown as Flow

		const requestBody = buildFlowDeployRequestBody(
			'f/demo/flow',
			'draft summary',
			{ value: { modules: [] }, schema: null, groups: null },
			existing,
			undefined
		)

		expect(requestBody.summary).toBe('draft summary')
		expect(requestBody.schema).toBe(existing.schema)
	})
})
