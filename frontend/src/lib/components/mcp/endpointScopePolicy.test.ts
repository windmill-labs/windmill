import { describe, expect, it } from 'vitest'
import {
	endpointPathPolicy,
	isEndpointExposed,
	parseMcpScopeState,
	pruneEndpointSelection
} from './endpointScopePolicy'
import { mcpEndpointTools } from '$lib/mcpEndpointTools'

const state = (...scopes: string[]) => parseMcpScopeState(scopes)

describe('isEndpointExposed', () => {
	it('gates run-by-path tools on the script/flow scope, not the endpoint scope', () => {
		// Selecting the endpoint without any script path exposes nothing...
		const endpointsOnly = state('mcp:endpoints:runScriptByPath,listScripts')
		expect(isEndpointExposed(endpointsOnly, 'runScriptByPath')).toBe(false)
		expect(isEndpointExposed(endpointsOnly, 'listScripts')).toBe(true)

		// ...while a script path exposes it without any endpoint scope.
		expect(isEndpointExposed(state('mcp:scripts:f/team/*'), 'runScriptByPath')).toBe(true)
		expect(isEndpointExposed(state('mcp:scripts:f/team/*'), 'runFlowByPath')).toBe(false)

		// Favorites reach their runnables through per-item tools only.
		expect(isEndpointExposed(state('mcp:favorites'), 'runScriptByPath')).toBe(false)
		expect(isEndpointExposed(state('mcp:all'), 'runScriptByPath')).toBe(true)
	})

	// A script/flow tool the URL addresses by path but the policy does not name falls
	// through to "no policy", which is "not path-confined at all" — a scoped token then
	// reaches every path of that kind. Catch the omission here rather than in a review.
	it('gives every path-addressed script/flow tool a policy', () => {
		const unpoliced = mcpEndpointTools
			.filter((e) => /\/(scripts|flows)\//.test(e.path) && e.path.includes('{path}'))
			.map((e) => e.name)
			.filter((name) => endpointPathPolicy(name) === undefined)
		expect(unpoliced).toEqual([])
	})

	it('withholds unconfinable tools from path-confined tokens', () => {
		const confined = state('mcp:scripts:f/team/*', 'mcp:endpoints:*')
		expect(isEndpointExposed(confined, 'runScriptPreviewAndWaitResult')).toBe(false)
		expect(isEndpointExposed(confined, 'getScriptByPath')).toBe(true)

		// A `*` script pattern is not a confinement.
		expect(
			isEndpointExposed(state('mcp:scripts:*', 'mcp:endpoints:*'), 'runScriptPreviewAndWaitResult')
		).toBe(true)
	})
})

describe('pruneEndpointSelection', () => {
	it('keeps a `*` endpoint scope, which would otherwise revoke every endpoint tool', () => {
		expect(pruneEndpointSelection(['*'], false)).toEqual(['*'])
		expect(pruneEndpointSelection(['*'], true)).toEqual(['*'])
	})

	it('drops only names the endpoint scope cannot honor', () => {
		expect(pruneEndpointSelection(['runScriptByPath', 'listScripts'], false)).toEqual([
			'listScripts'
		])
		// Non-GET endpoints go only when the token is read-only.
		expect(pruneEndpointSelection(['createScript', 'listScripts'], false)).toEqual([
			'createScript',
			'listScripts'
		])
		expect(pruneEndpointSelection(['createScript', 'listScripts'], true)).toEqual(['listScripts'])
	})
})
