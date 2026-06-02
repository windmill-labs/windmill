import { describe, expect, it } from 'vitest'
import type { Flow, NewScript, Script } from '$lib/gen/types.gen'
import {
	buildFlowDeployRequestBody,
	buildScriptDeployRequestBody,
	type FlowDeployMetadata,
	type ScriptDeployMetadata
} from './deployRequests'
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

	it('uses script draft metadata over existing deployed metadata', () => {
		const existing = {
			hash: 'parent-hash',
			path: 'f/demo/script',
			summary: 'existing summary',
			description: 'existing description',
			content: 'old content',
			schema: { properties: { existing: { type: 'string' } } },
			is_template: true,
			language: 'bun',
			kind: 'script',
			tag: 'old-tag',
			envs: ['OLD_ENV'],
			concurrent_limit: 5,
			cache_ttl: 300,
			cache_ignore_s3_path: false,
			dedicated_worker: true,
			ws_error_handler_muted: true,
			priority: 10,
			timeout: 120,
			visible_to_runner_only: true,
			on_behalf_of_email: 'existing@example.com',
			assets: [{ path: 's3://old/key', kind: 's3object' }],
			modules: { 'old.ts': { content: 'export const old = 1', language: 'bun' } },
			labels: ['old']
		} as unknown as Script & Partial<NewScript>
		const draft: WorkspaceItem = {
			type: 'script',
			path: 'f/demo/script',
			summary: 'draft summary',
			language: 'bun',
			value: 'new content',
			isDraft: true
		}
		const draftMetadata: ScriptDeployMetadata = {
			description: 'draft description',
			schema: { properties: { draft: { type: 'boolean' } } },
			tag: 'draft-tag',
			envs: [],
			concurrent_limit: 0,
			cache_ttl: 0,
			cache_ignore_s3_path: true,
			dedicated_worker: false,
			ws_error_handler_muted: false,
			priority: 0,
			timeout: 0,
			visible_to_runner_only: false,
			on_behalf_of_email: 'draft@example.com',
			assets: [{ path: 's3://draft/key', kind: 's3object' }],
			modules: null,
			labels: []
		}

		const requestBody = buildScriptDeployRequestBody(
			'f/demo/script',
			draft,
			existing,
			'ai deploy',
			draftMetadata
		)

		expect(requestBody).toMatchObject({
			path: 'f/demo/script',
			parent_hash: 'parent-hash',
			summary: 'draft summary',
			description: 'draft description',
			content: 'new content',
			schema: draftMetadata.schema,
			tag: 'draft-tag',
			envs: [],
			concurrent_limit: 0,
			cache_ttl: 0,
			cache_ignore_s3_path: true,
			dedicated_worker: false,
			ws_error_handler_muted: false,
			priority: 0,
			timeout: 0,
			visible_to_runner_only: false,
			on_behalf_of_email: 'draft@example.com',
			preserve_on_behalf_of: true,
			assets: draftMetadata.assets,
			modules: null,
			labels: [],
			deployment_message: 'ai deploy'
		})
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

	it('uses flow draft metadata over existing deployed metadata', () => {
		const existing = {
			path: 'f/demo/flow',
			summary: 'existing summary',
			description: 'existing description',
			value: { modules: [] },
			schema: { required: ['existing'] },
			tag: 'old-tag',
			ws_error_handler_muted: true,
			priority: 7,
			dedicated_worker: true,
			timeout: 60,
			visible_to_runner_only: true,
			on_behalf_of_email: 'existing@example.com',
			labels: ['old']
		} as unknown as Flow
		const draftValue = {
			value: { modules: [{ id: 'step', value: { type: 'identity' } }] },
			schema: { properties: { draft: { type: 'string' } } },
			groups: null
		}
		const draftMetadata: FlowDeployMetadata = {
			description: 'draft description',
			tag: 'draft-tag',
			ws_error_handler_muted: false,
			priority: 0,
			dedicated_worker: false,
			timeout: 0,
			visible_to_runner_only: false,
			on_behalf_of_email: undefined,
			labels: []
		}

		const requestBody = buildFlowDeployRequestBody(
			'f/demo/flow',
			'draft summary',
			draftValue as any,
			existing,
			'ai deploy',
			draftMetadata
		)

		expect(requestBody).toMatchObject({
			path: 'f/demo/flow',
			summary: 'draft summary',
			description: 'draft description',
			schema: draftValue.schema,
			tag: 'draft-tag',
			ws_error_handler_muted: false,
			priority: 0,
			dedicated_worker: false,
			timeout: 0,
			visible_to_runner_only: false,
			labels: [],
			deployment_message: 'ai deploy'
		})
		expect(requestBody.on_behalf_of_email).toBeUndefined()
		expect(requestBody.preserve_on_behalf_of).toBeUndefined()
		expect(requestBody.value.groups).toBeUndefined()
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
