import { describe, expect, it } from 'vitest'
import { flowModulesSchema } from './openFlowZod.gen'

// Guards against the generated copilot flow Zod schema (openFlowZod.gen.ts)
// drifting from the asset `kind` enum in openflow.openapi.yaml. A missing kind
// here rejects the whole flow module in the copilot editing path
// (validateFlowModules -> flowModulesSchema), so an unrelated AI edit to a flow
// containing a step that reads a warehouse table would fail.
function rawScriptModuleWithAssetKind(kind: string) {
	return {
		id: 'a',
		value: {
			type: 'rawscript',
			language: 'bun',
			content: 'export async function main() {}',
			input_transforms: {},
			assets: [{ path: 'analytics.stg_orders', kind, access_type: 'r' }]
		}
	}
}

describe('copilot flow module validation - asset kind', () => {
	it('accepts table (and the s3object baseline)', () => {
		expect(flowModulesSchema.safeParse([rawScriptModuleWithAssetKind('s3object')]).success).toBe(
			true
		)
		expect(flowModulesSchema.safeParse([rawScriptModuleWithAssetKind('dbt')]).success).toBe(true)
	})

	it('still rejects a kind the API does not define', () => {
		expect(flowModulesSchema.safeParse([rawScriptModuleWithAssetKind('warehouse')]).success).toBe(
			false
		)
	})
})
