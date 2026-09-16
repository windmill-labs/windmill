import { describe, expect, it } from 'vitest'
import {
	carriedReasoning,
	fixedReasoningReason,
	reasoningControlState,
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

	// The run sends an explicitly set effort whatever the model, so a token typed against a
	// provider we have no rules for has to reach the trigger — silence would hide it.
	it('names a set effort even where it can offer no ladder', () => {
		const shown = display({ provider: 'customai', model: 'deepseek-r1', value: 'high' })
		expect(shown.label).toBe('high')
		expect(shown.stops).toEqual([])
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

const asReasoning = (over: Partial<ChatModelSettingsReasoning>): ChatModelSettingsReasoning =>
	({
		provider: 'openai',
		model: 'gpt-5.1',
		value: undefined,
		offToken: REASONING_OFF,
		sendsDefaultWhenUnset: false,
		writable: true,
		typedWhenUnknown: true,
		onSelect: () => {},
		...over
	}) as ChatModelSettingsReasoning

/** The control is always drawn; this is the only thing that decides what it draws. */
describe('reasoningControlState', () => {
	const cap = (model: string) => getReasoningCapability('openai', model)

	it('shows the ladder for a model with levels', () => {
		expect(reasoningControlState(asReasoning({}), cap('gpt-5.1'))).toBe('ladder')
	})

	it('says a model cannot think when the registry knows it cannot', () => {
		expect(reasoningControlState(asReasoning({ model: 'gpt-4o' }), cap('gpt-4o'))).toBe(
			'unsupported'
		)
	})

	// Not the same as "cannot think": we have no rules for the provider, so the flow's own
	// token is typed rather than picked.
	it('asks for a typed token where it has no rules for the provider', () => {
		expect(
			reasoningControlState(
				asReasoning({ provider: 'customai', model: 'deepseek-r1' }),
				getReasoningCapability('customai', 'deepseek-r1')
			)
		).toBe('unknown')
	})

	// The session chat has no typed effort: a provider with no rules reads as unable to think.
	it('offers no typed token to a chat that does not take one', () => {
		expect(
			reasoningControlState(
				asReasoning({ provider: 'customai', model: 'deepseek-r1', typedWhenUnknown: false }),
				getReasoningCapability('customai', 'deepseek-r1')
			)
		).toBe('unsupported')
	})

	// A provider with a full ladder must not be described as unreadable just because no
	// model has been picked yet — which is the state right after choosing a resource.
	it('waits for a model rather than blaming the provider', () => {
		expect(
			reasoningControlState(asReasoning({ model: undefined }), { supported: false, known: false })
		).toBe('awaiting-model')
	})

	it('shows what the flow fixed when this chat cannot write it', () => {
		expect(reasoningControlState(asReasoning({ writable: false }), cap('gpt-5.1'))).toBe('fixed')
	})
})

describe('fixedReasoningReason', () => {
	it('names the level the run will use', () => {
		expect(
			fixedReasoningReason(asReasoning({ value: 'high' }), { supported: true, known: true })
		).toBe('high · set in the flow')
	})

	// The step naming no effort at all is the common shape; saying it was "set in the flow"
	// would describe a line the flow does not contain.
	it('does not claim a level the step never set', () => {
		expect(
			fixedReasoningReason(asReasoning({ value: undefined }), { supported: true, known: true })
		).toBe('Not set in the flow, so the provider decides')
	})

	it('surfaces a level fixed on a model that cannot use it', () => {
		expect(
			fixedReasoningReason(asReasoning({ value: 'high', model: 'gpt-4o' }), {
				supported: false,
				known: true
			})
		).toContain('cannot think')
	})
})
