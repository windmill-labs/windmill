import { describe, expect, it } from 'vitest'
import { isStepOfFlow, type AgentEditorTarget } from './agentEditorStore.svelte'

/** An agent opened from a step of the flow at `flowPath`. */
function fromFlowStep(flowPath: string): AgentEditorTarget {
	return { path: 'u/admin/triage', host: { flowPath, moduleId: 'a' } }
}

/** A nested agent opened from inside an agent editor, which hosts its flow under the agent's own
 *  path — so `flowPath` here is a resource path that a flow may also carry. */
function fromAgentEditor(agentPath: string): AgentEditorTarget {
	return {
		path: 'u/admin/nested',
		host: { flowPath: agentPath, moduleId: 'a', fromAgentEditor: true }
	}
}

describe('isStepOfFlow', () => {
	it('claims a step of its own flow', () => {
		expect(isStepOfFlow(fromFlowStep('u/admin/triage_flow'), 'u/admin/triage_flow')).toBe(true)
	})

	it('leaves another flow alone', () => {
		expect(isStepOfFlow(fromFlowStep('u/admin/other'), 'u/admin/triage_flow')).toBe(false)
	})

	// The one a bare path comparison gets wrong: a flow and an agent resource may share a path, and
	// the flow mount would then claim a nested agent that belongs to the open editor — two editors
	// over one draft, and closing the flow tab takes the visible one down with it.
	it('leaves an agent editor’s nested target alone, same path or not', () => {
		expect(isStepOfFlow(fromAgentEditor('u/admin/triage_flow'), 'u/admin/triage_flow')).toBe(false)
		expect(isStepOfFlow(fromAgentEditor('u/admin/agent'), 'u/admin/triage_flow')).toBe(false)
	})

	it('leaves a target with no host to the resources page', () => {
		expect(isStepOfFlow({ path: 'u/admin/triage' }, 'u/admin/triage_flow')).toBe(false)
	})
})
