import { describe, expect, it } from 'vitest'
import {
	handleBenchmarkApiFetch,
	hasBenchmarkApiHandler,
	registerBenchmarkWorkspaceRunnables,
	unregisterBenchmarkWorkspace
} from './mockBackend'

// A global eval run registers its workspace under a mkdtemp path, so the workspace id the
// frontend interpolates into the models URL carries slashes. A handler that assumed a single
// path segment silently fell through to the network, and every run took the offline fallback.
const WORKSPACE = '/tmp/wmill-frontend-global-benchmark-abc123'
const RESOURCE = 'f/evals/global/anthropic_main'
const URL_FOR = (workspace: string) =>
	`http://benchmark.local/api/w/${workspace}/ai/proxy/models`

describe('benchmark /ai/proxy/models', () => {
	it('serves the seeded listing for a workspace id that is a path', async () => {
		registerBenchmarkWorkspaceRunnables(WORKSPACE, {
			aiProviders: [
				{ path: RESOURCE, kind: 'anthropic', models: ['claude-sonnet-5', 'claude-opus-5'] }
			]
		})
		try {
			expect(hasBenchmarkApiHandler(URL_FOR(WORKSPACE))).toBe(true)
			const response = handleBenchmarkApiFetch(URL_FOR(WORKSPACE), {
				headers: { 'X-Resource-Path': RESOURCE, 'X-Provider': 'anthropic' }
			})
			await expect(response.json()).resolves.toEqual({
				data: [{ id: 'claude-sonnet-5' }, { id: 'claude-opus-5' }]
			})
		} finally {
			unregisterBenchmarkWorkspace(WORKSPACE)
		}
	})
})
