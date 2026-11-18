import { describe, expect, it } from 'vitest'
import { conformBuilderAppPolicy } from './builderAppPolicy'

const builder = { username: 'op', email: 'op@x.dev' }

describe('conformBuilderAppPolicy', () => {
	it('conforms a policy loaded from an app someone else deployed', () => {
		const policy: Record<string, any> = {
			sandbox: false,
			execution_mode: 'viewer',
			on_behalf_of: 'u/admin',
			on_behalf_of_email: 'admin@x.dev'
		}
		conformBuilderAppPolicy(policy, builder)
		expect(policy).toEqual({ sandbox: true, execution_mode: 'publisher' })
	})

	it("keeps the builder's own identity", () => {
		const policy: Record<string, any> = {
			sandbox: true,
			on_behalf_of: 'u/op',
			on_behalf_of_email: 'op@x.dev'
		}
		conformBuilderAppPolicy(policy, builder)
		expect(policy).toEqual({ sandbox: true, on_behalf_of: 'u/op', on_behalf_of_email: 'op@x.dev' })
	})
})
