import { describe, it, expect } from 'vitest'
import { refContainmentViolation, varContainmentViolation } from './projectInstall'
import type { Ref } from './projectBundle'

describe('refContainmentViolation', () => {
	const folder = 'proj'
	const violation = (r: Ref) => refContainmentViolation([r], folder)

	it('allows references relocated into the target folder', () => {
		expect(violation({ kind: 'resource', path: 'f/proj/db' })).toBeUndefined()
		expect(violation({ kind: 'script', path: 'f/proj/helper' })).toBeUndefined()
		expect(violation({ kind: 'flow', path: 'f/proj/sub' })).toBeUndefined()
	})

	it('allows hub script/flow references but never hub resources', () => {
		expect(violation({ kind: 'script', path: 'hub/1/x/y' })).toBeUndefined()
		expect(violation({ kind: 'flow', path: 'hub/1/a/b' })).toBeUndefined()
		// Resources are not hub-hosted, so a hub/ resource path is still an escape.
		expect(violation({ kind: 'resource', path: 'hub/1/x/y' })).toBeDefined()
	})

	it('rejects references bound to another namespace', () => {
		// The crux: an in-folder runnable pointing its resource at an existing asset.
		expect(violation({ kind: 'resource', path: 'u/admin/db' })).toContain('escapes')
		expect(violation({ kind: 'script', path: 'f/other/helper' })).toContain('escapes')
		expect(violation({ kind: 'flow', path: 'u/admin/sub' })).toContain('escapes')
	})

	it('does not treat a prefix-only folder match as internal', () => {
		expect(violation({ kind: 'script', path: 'f/proj2/helper' })).toContain('escapes')
	})

	it('reports the first offending reference and passes a fully-contained set', () => {
		expect(
			refContainmentViolation(
				[
					{ kind: 'resource', path: 'f/proj/db' },
					{ kind: 'script', path: 'hub/1/x/y' }
				],
				folder
			)
		).toBeUndefined()
		expect(
			refContainmentViolation(
				[
					{ kind: 'resource', path: 'f/proj/db' },
					{ kind: 'resource', path: 'u/admin/secret' }
				],
				folder
			)
		).toContain('u/admin/secret')
	})
})

describe('varContainmentViolation', () => {
	const folder = 'proj'

	it('allows in-folder variable references', () => {
		expect(varContainmentViolation({ token: '$var:f/proj/token' }, folder)).toBeUndefined()
		expect(varContainmentViolation({ x: 'no refs here' }, folder)).toBeUndefined()
	})

	it('rejects a `$var:` or `$jsonvar:` bound to another namespace', () => {
		// The crux: a variable arg the ref extractors miss, resolved under the perms.
		expect(varContainmentViolation({ queue_url: '$var:u/admin/token' }, folder)).toContain(
			'u/admin/token'
		)
		expect(varContainmentViolation({ cfg: '$jsonvar:f/other/secret' }, folder)).toContain('escapes')
	})

	it('ignores a `$var:` literal embedded in inline code', () => {
		const flowValue = {
			flow_env: { API: '$var:f/proj/api_key' },
			modules: [{ value: { type: 'rawscript', content: 'return "$var:u/admin/should_not_flag"' } }]
		}
		expect(varContainmentViolation(flowValue, folder)).toBeUndefined()
	})
})
