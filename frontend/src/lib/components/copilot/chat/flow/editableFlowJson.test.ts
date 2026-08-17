import { describe, expect, it, vi } from 'vitest'
import type { FlowModule, FlowValue } from '$lib/gen'
import {
	applyEditableFlowJsonToFlow,
	buildEditableFlowJson,
	validateEditableFlowJson
} from './editableFlowJson'
import { createInlineScriptSession } from './inlineScriptsUtils'

vi.mock('../shared', () => ({
	SPECIAL_MODULE_IDS: {
		PREPROCESSOR: 'preprocessor',
		FAILURE: 'failure'
	}
}))

function makeRawScriptModule(id: string, content: string): FlowModule {
	return {
		id,
		summary: id,
		value: {
			type: 'rawscript',
			language: 'bun',
			content,
			input_transforms: {}
		}
	} as FlowModule
}

function makeFlowValue(extra: Record<string, unknown> = {}): FlowValue {
	return {
		modules: [makeRawScriptModule('step_a', 'code a')],
		...extra
	} as FlowValue
}

describe('flow settings in the compact editable view', () => {
	it('round-trips top-level flow settings through build → validate → apply', () => {
		const value = makeFlowValue({ chat_input_enabled: true, same_worker: true })
		const session = createInlineScriptSession()
		const editable = buildEditableFlowJson({ value }, session)

		expect(editable.chat_input_enabled).toBe(true)
		expect(editable.same_worker).toBe(true)

		const revalidated = validateEditableFlowJson(JSON.parse(JSON.stringify(editable)))
		const result = applyEditableFlowJsonToFlow(value, revalidated, session)

		expect(result.chat_input_enabled).toBe(true)
		expect(result.same_worker).toBe(true)
		expect(result.modules[0]?.value).toMatchObject({ type: 'rawscript', content: 'code a' })
	})

	it('applies a patched-in setting and deletes a removed one', () => {
		const value = makeFlowValue({ cache_ttl: 60 })
		const session = createInlineScriptSession()
		const editable = buildEditableFlowJson({ value }, session)

		const patched = JSON.parse(JSON.stringify(editable))
		patched.chat_input_enabled = true
		delete patched.cache_ttl

		const result = applyEditableFlowJsonToFlow(value, validateEditableFlowJson(patched), session)

		expect(result.chat_input_enabled).toBe(true)
		expect('cache_ttl' in result).toBe(false)
	})

	it('preserves original FlowValue fields outside the compact view', () => {
		const value = makeFlowValue({ some_future_field: 'kept' })
		const session = createInlineScriptSession()
		const editable = buildEditableFlowJson({ value }, session)

		expect('some_future_field' in editable).toBe(false)

		const result = applyEditableFlowJsonToFlow(
			value,
			validateEditableFlowJson(JSON.parse(JSON.stringify(editable))),
			session
		)

		expect((result as Record<string, unknown>).some_future_field).toBe('kept')
	})

	it('rejects unknown top-level keys instead of silently dropping them', () => {
		expect(() =>
			validateEditableFlowJson({
				modules: [makeRawScriptModule('step_a', 'code a')],
				chat_enabled: true
			})
		).toThrow(/Unknown top-level flow key\(s\): chat_enabled/)
	})

	it('rejects settings with the wrong type', () => {
		expect(() =>
			validateEditableFlowJson({
				modules: [makeRawScriptModule('step_a', 'code a')],
				chat_input_enabled: 'yes'
			})
		).toThrow(/chat_input_enabled/)
	})
})
