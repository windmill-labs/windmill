import { describe, expect, it } from 'vitest'
import { accessNarrowing } from './accessNarrowing'
import {
	capabilitiesForRole,
	fullSessionAccess
} from '$lib/components/copilot/chat/global/sessionAccess'
import { NO_RUN_ACCESS, USE_MODE_ACCESS } from '$lib/components/copilot/chat/sessionCapabilities'
import { FULL_CAPABILITIES } from '$lib/roleCapabilities'

describe('accessNarrowing', () => {
	it('says nothing where the workspace holds the token to what was picked', () => {
		for (const picked of ['full', 'read_run', 'read', 'selected'] as const)
			expect(accessNarrowing(picked, fullSessionAccess())).toBeUndefined()
		expect(accessNarrowing('read_run', USE_MODE_ACCESS)).toBeUndefined()
		expect(accessNarrowing('read', NO_RUN_ACCESS)).toBeUndefined()
	})

	it('names how a workspace narrows each level', () => {
		expect(accessNarrowing('read_run', NO_RUN_ACCESS)).toContain('only reads')
		expect(accessNarrowing('selected', NO_RUN_ACCESS)).toContain('no tools')
		expect(accessNarrowing('full', USE_MODE_ACCESS)).toContain('protected')
		const noCode = capabilitiesForRole({
			isAdmin: false,
			deployRulesPass: false,
			capabilities: { ...FULL_CAPABILITIES, code: false, flows: 'compose', apps: 'compose' }
		})
		expect(accessNarrowing('full', noCode)).toBe(
			"your role can't build scripts and the deploy rules don't let you deploy, so the client can't either"
		)
	})
})
