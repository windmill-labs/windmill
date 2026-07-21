import { describe, expect, it } from 'vitest'
import { flowModulesSchema } from './openFlowZod.gen'

// Guards against the generated copilot flow Zod schema (openFlowZod.gen.ts)
// drifting from the RawScript language enum in openflow.openapi.yaml. A missing
// language here silently rejects AI-generated flow edits that use it in the
// copilot flow-editing path (validateFlowModules -> flowModulesSchema).
function rawScriptModuleWithLanguage(language: string) {
	return {
		id: 'a',
		value: {
			type: 'rawscript',
			language,
			content: 'export async function main() {}',
			input_transforms: {}
		}
	}
}

describe('copilot flow module validation - rawscript language', () => {
	it('accepts bunnative (and the bun baseline)', () => {
		expect(flowModulesSchema.safeParse([rawScriptModuleWithLanguage('bun')]).success).toBe(true)
		expect(flowModulesSchema.safeParse([rawScriptModuleWithLanguage('bunnative')]).success).toBe(
			true
		)
	})

	it('still rejects an unknown language (enum is actually enforced)', () => {
		expect(
			flowModulesSchema.safeParse([rawScriptModuleWithLanguage('not_a_real_lang')]).success
		).toBe(false)
	})
})
