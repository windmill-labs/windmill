import { describe, expect, it } from 'vitest'
import {
	carriedReasoning,
	reasoningDisplay,
	REASONING_PROVIDER_DEFAULT,
	type ChatModelSettingsReasoning
} from './chatModelSettings'
import {
	getReasoningCapability,
	REASONING_OFF,
	resolveEffectiveReasoning
} from './reasoningRegistry'

/**
 * The trigger's suffix and the slider's stops are read side by side, so they have to agree
 * about one value — the provider-native off token must not read as `none` on one and `off`
 * on the other, and an effort the run does not send must not be named at all.
 */
function display(
	reasoning: Partial<ChatModelSettingsReasoning> & { provider: any; model: string }
) {
	const full = {
		value: undefined,
		offToken: undefined,
		sendsDefaultWhenUnset: false,
		onSelect: () => {},
		...reasoning
	} as ChatModelSettingsReasoning
	const capability = getReasoningCapability(full.provider, full.model)
	// Composed exactly as the component composes it, so the test exercises the real pair.
	const effective = resolveEffectiveReasoning({
		provider: full.provider,
		model: full.model,
		reasoning: full.value
	})
	return reasoningDisplay(full, capability, effective)
}

describe('reasoningDisplay', () => {
	it('says nothing for a model that cannot reason', () => {
		const shown = display({ provider: 'openai', model: 'gpt-4o' })
		expect(shown.label).toBeUndefined()
		expect(shown.stops).toEqual([])
	})

	// The session chat's own sentinel: what it stores is already the word the reader sees.
	it('reads the session chat off sentinel as off', () => {
		const shown = display({
			provider: 'openai',
			model: 'gpt-5.1',
			offToken: REASONING_OFF,
			value: REASONING_OFF,
			sendsDefaultWhenUnset: true
		})
		expect(shown.label).toBe(REASONING_OFF)
		expect(shown.currentStop).toBe(REASONING_OFF)
	})

	// An agent writes the provider's own token, which can read as anything.
	it('reads a provider-native off token as off too', () => {
		const shown = display({
			provider: 'openai',
			model: 'gpt-5.1',
			offToken: 'none',
			value: 'none'
		})
		expect(shown.label).toBe(REASONING_OFF)
		expect(shown.currentStop).toBe('none')
		expect(shown.stops[0]).toBe('none')
	})

	it('names the level a chat that fills one in will send', () => {
		const shown = display({
			provider: 'openai',
			model: 'gpt-5.1',
			offToken: REASONING_OFF,
			value: undefined,
			sendsDefaultWhenUnset: true
		})
		expect(shown.label).toBe('high')
	})

	// An agent step omits the field, so naming a level would claim something untrue.
	it('names no level where an unset effort is simply not sent', () => {
		const shown = display({
			provider: 'anthropic',
			model: 'claude-sonnet-5',
			offToken: 'none',
			value: undefined
		})
		expect(shown.label).toBe(REASONING_PROVIDER_DEFAULT)
		expect(shown.currentStop).toBe('')
	})

	// Claude 4.x only thinks when asked, so an absent effort is already off — and the flow
	// chat must be able to get back to it after a level has been picked.
	it('offers omission as the off stop where that is how the model disables', () => {
		const unset = display({ provider: 'anthropic', model: 'claude-opus-4-6', offToken: '' })
		expect(unset.label).toBe(REASONING_OFF)
		expect(unset.stops[0]).toBe('')
		const picked = display({
			provider: 'anthropic',
			model: 'claude-opus-4-6',
			offToken: '',
			value: 'high'
		})
		expect(picked.currentStop).toBe('high')
		expect(picked.stops).toContain('')
	})

	// gpt-5 reasons at medium with no effort sent, so an empty off token buys no off stop.
	it('offers no off where the model cannot stop thinking', () => {
		const shown = display({ provider: 'openai', model: 'gpt-5', offToken: '' })
		expect(shown.stops).not.toContain('')
		expect(shown.label).toBe(REASONING_PROVIDER_DEFAULT)
	})
})

describe('carriedReasoning', () => {
	const cap = (model: string) => getReasoningCapability('openai', model)

	// The bug this exists for: picking a model that cannot think left the old level in the
	// flow input, and the run sent it anyway.
	it('drops a level the new model does not have', () => {
		expect(carriedReasoning('high', '', cap('gpt-4o'))).toBeUndefined()
		expect(carriedReasoning('xhigh', '', cap('gpt-5.1'))).toBeUndefined()
	})

	it('keeps a level the new model does have', () => {
		expect(carriedReasoning('high', '', cap('gpt-5.1'))).toBe('high')
	})

	it('carries off only onto a model that can truly stop thinking', () => {
		expect(carriedReasoning(REASONING_OFF, REASONING_OFF, cap('gpt-5.1'))).toBe(REASONING_OFF)
		expect(carriedReasoning(REASONING_OFF, REASONING_OFF, cap('gpt-5'))).toBeUndefined()
	})

	// A provider the registry has no rules for draws no thinking control, so a carried level
	// would be invisible and unclearable — and still sent, since an explicitly set effort
	// goes out whatever the model.
	it('drops the effort where it has no rules for the provider', () => {
		expect(
			carriedReasoning('high', REASONING_OFF, getReasoningCapability('customai', 'deepseek-r1'))
		).toBeUndefined()
	})

	it('has nothing to carry when no effort is set', () => {
		expect(carriedReasoning(undefined, '', cap('gpt-5.1'))).toBeUndefined()
		expect(carriedReasoning('', '', cap('gpt-5.1'))).toBeUndefined()
	})
})
