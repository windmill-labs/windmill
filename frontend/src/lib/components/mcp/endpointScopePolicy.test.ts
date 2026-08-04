import { describe, expect, it } from 'vitest'
import { isEndpointExposed, parseMcpScopeState } from './endpointScopePolicy'

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
