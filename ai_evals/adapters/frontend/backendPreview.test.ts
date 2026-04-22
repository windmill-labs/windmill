import { afterEach, describe, expect, it } from 'bun:test'
import type { BackendValidationSettings } from '../../core/backendValidation'
import { BackendPreviewClient } from './backendPreview'

const ORIGINAL_FETCH = globalThis.fetch

afterEach(() => {
	globalThis.fetch = ORIGINAL_FETCH
})

describe('BackendPreviewClient', () => {
	it('updates an existing seeded script on path conflict and waits for deployment', async () => {
		const requests: Array<{ url: string; init?: RequestInit }> = []
		globalThis.fetch = mockFetch(
			requests,
			textResponse(200, 'token'),
			textResponse(200, ''),
			textResponse(400, 'Path conflict for f/evals/add_two_numbers with non-archived hash 123'),
			jsonResponse(200, { hash: '123' }),
			textResponse(200, '456'),
			jsonResponse(200, { lock: 'script.lock', lock_error_logs: null })
		)

		const client = new BackendPreviewClient(
			buildSettings({ baseUrl: 'http://backend.test/script-upsert' })
		)

		await client.createScript({
			workspaceId: 'test',
			path: 'f/evals/add_two_numbers',
			summary: 'Add two numbers',
			content: 'export async function main(a: number, b: number) { return a + b }',
			language: 'bun'
		})

		expect(requests.map((entry) => entry.url)).toEqual([
			'http://backend.test/script-upsert/api/auth/login',
			'http://backend.test/script-upsert/api/w/test/folders/create',
			'http://backend.test/script-upsert/api/w/test/scripts/create',
			'http://backend.test/script-upsert/api/w/test/scripts/get/p/f/evals/add_two_numbers',
			'http://backend.test/script-upsert/api/w/test/scripts/create',
			'http://backend.test/script-upsert/api/w/test/scripts/deployment_status/h/456'
		])

		const updateRequest = requests[4]
		expect(updateRequest.init?.method).toBe('POST')
		expect(JSON.parse(String(updateRequest.init?.body))).toMatchObject({
			path: 'f/evals/add_two_numbers',
			parent_hash: '123',
			language: 'bun'
		})
	})

	it('updates an existing seeded flow on create conflict', async () => {
		const requests: Array<{ url: string; init?: RequestInit }> = []
		globalThis.fetch = mockFetch(
			requests,
			textResponse(200, 'token'),
			textResponse(200, ''),
			textResponse(400, 'Flow f/evals/add_numbers_flow already exists'),
			textResponse(200, '')
		)

		const client = new BackendPreviewClient(
			buildSettings({ baseUrl: 'http://backend.test/flow-upsert' })
		)

		await client.createFlow({
			workspaceId: 'test',
			path: 'f/evals/add_numbers_flow',
			summary: 'Add numbers',
			value: { modules: [] }
		})

		expect(requests.map((entry) => entry.url)).toEqual([
			'http://backend.test/flow-upsert/api/auth/login',
			'http://backend.test/flow-upsert/api/w/test/folders/create',
			'http://backend.test/flow-upsert/api/w/test/flows/create',
			'http://backend.test/flow-upsert/api/w/test/flows/update/f/evals/add_numbers_flow'
		])

		const updateRequest = requests[3]
		expect(updateRequest.init?.method).toBe('POST')
		expect(JSON.parse(String(updateRequest.init?.body))).toMatchObject({
			path: 'f/evals/add_numbers_flow',
			value: { modules: [] }
		})
	})

	it('serializes shared-workspace validations inside the overridden workspace', async () => {
		globalThis.fetch = async (input) => {
			const url = String(input)
			if (url.endsWith('/api/auth/login')) {
				return textResponse(200, 'token')
			}
			if (url.endsWith('/api/workspaces/exists')) {
				return textResponse(200, 'true')
			}
			if (url.endsWith('/api/w/shared-preview/flows/list_paths')) {
				return jsonResponse(200, [])
			}
			if (url.endsWith('/api/w/shared-preview/scripts/list_paths')) {
				return jsonResponse(200, [])
			}
			throw new Error(`Unexpected fetch: ${url}`)
		}

		const client = new BackendPreviewClient(
			buildSettings({
				baseUrl: 'http://backend.test/shared-lock',
				workspaceOverride: 'shared-preview'
			})
		)

		const order: string[] = []
		let releaseFirst: (() => void) | undefined
		let notifyFirstStart: (() => void) | undefined
		const firstStarted = new Promise<void>((resolve) => {
			notifyFirstStart = resolve
		})

		const first = client.withWorkspace('flow-test1', 1, async () => {
			order.push('first:start')
			notifyFirstStart?.()
			await new Promise<void>((resolve) => {
				releaseFirst = resolve
			})
			order.push('first:end')
		})

		const second = client.withWorkspace('flow-test2', 1, async () => {
			order.push('second:start')
			order.push('second:end')
		})

		await firstStarted
		expect(order).toEqual(['first:start'])

		releaseFirst?.()
		await Promise.all([first, second])

		expect(order).toEqual(['first:start', 'first:end', 'second:start', 'second:end'])
	})

	it('clears managed shared-workspace assets before preview runs', async () => {
		const requests: Array<{ url: string; init?: RequestInit }> = []
		globalThis.fetch = mockFetch(
			requests,
			textResponse(200, 'token'),
			textResponse(200, 'true'),
			jsonResponse(200, ['f/evals/old_subflow', 'u/admin/keep_flow']),
			textResponse(200, ''),
			jsonResponse(200, ['f/evals/old_script', 'f/shared/keep_script']),
			textResponse(200, '')
		)

		const client = new BackendPreviewClient(
			buildSettings({
				baseUrl: 'http://backend.test/shared-cleanup',
				workspaceOverride: 'shared-preview'
			})
		)

		await client.withWorkspace('flow-test1', 1, async () => undefined)

		expect(requests.map((entry) => entry.url)).toEqual([
			'http://backend.test/shared-cleanup/api/auth/login',
			'http://backend.test/shared-cleanup/api/workspaces/exists',
			'http://backend.test/shared-cleanup/api/w/shared-preview/flows/list_paths',
			'http://backend.test/shared-cleanup/api/w/shared-preview/flows/delete/f/evals/old_subflow',
			'http://backend.test/shared-cleanup/api/w/shared-preview/scripts/list_paths',
			'http://backend.test/shared-cleanup/api/w/shared-preview/scripts/delete/p/f/evals/old_script'
		])
	})

	it('retries login after a cached login failure', async () => {
		const requests: Array<{ url: string; init?: RequestInit }> = []
		globalThis.fetch = mockFetch(
			requests,
			textResponse(503, 'backend starting'),
			textResponse(200, 'token'),
			textResponse(200, 'true'),
			jsonResponse(200, []),
			jsonResponse(200, [])
		)

		const client = new BackendPreviewClient(
			buildSettings({
				baseUrl: 'http://backend.test/login-retry',
				workspaceOverride: 'shared-preview'
			})
		)

		await expect(client.withWorkspace('flow-test1', 1, async () => undefined)).rejects.toThrow(
			'login for backend validation failed'
		)
		await expect(client.withWorkspace('flow-test1', 1, async () => 'ok')).resolves.toBe('ok')

		expect(
			requests.filter((entry) => entry.url === 'http://backend.test/login-retry/api/auth/login')
		).toHaveLength(2)
	})
})

function buildSettings(
	overrides: Partial<BackendValidationSettings> = {}
): BackendValidationSettings {
	return {
		mode: 'preview',
		baseUrl: 'http://backend.test/default',
		email: 'admin@windmill.dev',
		password: 'changeme',
		keepWorkspaces: true,
		workspacePrefix: 'ai-evals',
		pollIntervalMs: 1,
		maxWaitMs: 50,
		...overrides
	}
}

function mockFetch(
	requests: Array<{ url: string; init?: RequestInit }>,
	...responses: Response[]
): typeof fetch {
	const queue = [...responses]
	return async (input, init) => {
		const url = String(input)
		requests.push({ url, init })
		const next = queue.shift()
		if (!next) {
			throw new Error(`Unexpected fetch: ${url}`)
		}
		return next
	}
}

function jsonResponse(status: number, body: unknown): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'Content-Type': 'application/json' }
	})
}

function textResponse(status: number, body: string): Response {
	return new Response(body, { status })
}
