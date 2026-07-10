import { describe, it, expect } from 'vitest'
import type { FlowModule } from '$lib/gen'
import {
	buildNestedRestartPath,
	findStepPath,
	parseExpandedSubflowId,
	type ForloopGraphState,
	type FlowStatusModuleLite
} from './restartFromStepPath'

// --- minimal FlowModule fixtures (findStepPath only reads type/modules/branches/parallel) ---
const script = (id: string): FlowModule =>
	({ id, value: { type: 'rawscript', language: 'bun', content: '', input_transforms: {} } }) as any
const subflow = (id: string, path = 'f/x'): FlowModule =>
	({ id, value: { type: 'flow', path } }) as any
const forloop = (id: string, modules: FlowModule[], parallel = false): FlowModule =>
	({
		id,
		value: { type: 'forloopflow', modules, iterator: { type: 'static', value: [] }, parallel }
	}) as any
const whileloop = (id: string, modules: FlowModule[]): FlowModule =>
	({ id, value: { type: 'whileloopflow', modules, parallel: false } }) as any
const branchone = (id: string, def: FlowModule[], branches: FlowModule[][]): FlowModule =>
	({
		id,
		value: { type: 'branchone', default: def, branches: branches.map((m) => ({ modules: m })) }
	}) as any
const branchall = (id: string, branches: FlowModule[][], parallel = false): FlowModule =>
	({
		id,
		value: { type: 'branchall', branches: branches.map((m) => ({ modules: m })), parallel }
	}) as any

function build(opts: {
	selectedJobStep: string
	rawFlowModules: FlowModule[]
	flowStatusModules?: FlowStatusModuleLite[]
	graphModuleStates?: Record<string, ForloopGraphState>
	expandedSubflows?: Record<string, { modules: FlowModule[] }>
}) {
	return buildNestedRestartPath({
		flowStatusModules: [],
		graphModuleStates: {},
		expandedSubflows: {},
		...opts
	})
}

describe('parseExpandedSubflowId', () => {
	it('parses a multi-level subflow id', () => {
		expect(parseExpandedSubflowId('subflow:a:b:leaf')).toEqual({
			subflowSteps: ['a', 'b'],
			leaf: 'leaf'
		})
	})
	it('returns undefined for a bare id or a single segment', () => {
		expect(parseExpandedSubflowId('leaf')).toBeUndefined()
		expect(parseExpandedSubflowId('subflow:leaf')).toBeUndefined()
	})
})

describe('buildNestedRestartPath', () => {
	it('pure nested subflows: top is the outer subflow, no iterations', () => {
		const r = build({
			selectedJobStep: 'subflow:stop:smid:a',
			rawFlowModules: [subflow('stop', 'f/mid')],
			expandedSubflows: {
				stop: { modules: [subflow('smid', 'f/leaf')] },
				'subflow:stop:smid': { modules: [script('a')] }
			}
		})
		expect(r).toEqual({
			topStepId: 'stop',
			topBranchOrIterationN: undefined,
			path: [{ step_id: 'smid' }, { step_id: 'a' }],
			iterationCounts: {}
		})
	})

	it('subflow nested inside a ForLoop: recovers the ForLoop as the top step (the reported bug)', () => {
		const r = build({
			selectedJobStep: 'subflow:s:a',
			rawFlowModules: [forloop('L', [subflow('s', 'f/leaf')])],
			expandedSubflows: { s: { modules: [script('a')] } },
			graphModuleStates: { L: { selectedForloopIndex: 1, flow_jobs: ['j0', 'j1'] } }
		})
		expect(r).toEqual({
			topStepId: 'L',
			topBranchOrIterationN: 1,
			path: [{ step_id: 's' }, { step_id: 'a' }],
			iterationCounts: { top: 2 }
		})
	})

	it('ForLoop sitting between two subflow boundaries is recovered as an inner path step', () => {
		const r = build({
			selectedJobStep: 'subflow:sa:sb:a',
			rawFlowModules: [subflow('sa', 'f/mid')],
			expandedSubflows: {
				sa: { modules: [forloop('L', [subflow('sb', 'f/leaf')])] },
				'subflow:sa:sb': { modules: [script('a')] }
			},
			graphModuleStates: { 'subflow:sa:L': { selectedForloopIndex: 0, flow_jobs: ['j0', 'j1'] } }
		})
		expect(r).toEqual({
			topStepId: 'sa',
			topBranchOrIterationN: undefined,
			path: [{ step_id: 'L', branch_or_iteration_n: 0 }, { step_id: 'sb' }, { step_id: 'a' }],
			iterationCounts: { 'inner-0': 2 }
		})
	})

	it('leaf directly inside a top-level ForLoop (no subflow)', () => {
		const r = build({
			selectedJobStep: 'x',
			rawFlowModules: [forloop('L', [script('x')])],
			graphModuleStates: { L: { selectedForloopIndex: 2, flow_jobs: ['a', 'b', 'c'] } }
		})
		expect(r).toEqual({
			topStepId: 'L',
			topBranchOrIterationN: 2,
			path: [{ step_id: 'x' }],
			iterationCounts: { top: 3 }
		})
	})

	it('the leaf itself is a ForLoop: exposes its own iteration too', () => {
		const r = build({
			selectedJobStep: 'M',
			rawFlowModules: [forloop('L', [forloop('M', [script('y')])])],
			graphModuleStates: {
				L: { selectedForloopIndex: 0, flow_jobs: ['a'] },
				M: { selectedForloopIndex: 1, flow_jobs: ['p', 'q'] }
			}
		})
		expect(r).toEqual({
			topStepId: 'L',
			topBranchOrIterationN: 0,
			path: [{ step_id: 'M', branch_or_iteration_n: 1 }],
			iterationCounts: { top: 1, 'inner-0': 2 }
		})
	})

	it('BranchOne top step: no iteration sent (backend locks the branch from the run)', () => {
		const r = build({
			selectedJobStep: 'x',
			rawFlowModules: [branchone('BO', [script('d')], [[script('x')]])],
			flowStatusModules: [{ id: 'BO', branch_chosen: { type: 'branch', branch: 0 } }]
		})
		expect(r).toEqual({
			topStepId: 'BO',
			topBranchOrIterationN: undefined,
			path: [{ step_id: 'x' }],
			iterationCounts: {}
		})
	})

	it('BranchOne default branch taken and leaf is in the default branch', () => {
		const r = build({
			selectedJobStep: 'd',
			rawFlowModules: [branchone('BO', [script('d')], [[script('x')]])],
			flowStatusModules: [{ id: 'BO', branch_chosen: { type: 'default' } }]
		})
		expect(r?.topStepId).toBe('BO')
		expect(r?.path).toEqual([{ step_id: 'd' }])
	})

	it('rejects a leaf inside a BranchOne branch the original run did not take', () => {
		const r = build({
			selectedJobStep: 'x',
			rawFlowModules: [branchone('BO', [script('d')], [[script('x')]])],
			// Original run took the default, not branch 0 (which contains x).
			flowStatusModules: [{ id: 'BO', branch_chosen: { type: 'default' } }]
		})
		expect(r).toBeNull()
	})

	it('rejects unsupported containers: parallel ForLoop, BranchAll, WhileLoop', () => {
		expect(
			build({ selectedJobStep: 'x', rawFlowModules: [forloop('L', [script('x')], true)] })
		).toBeNull()
		expect(
			build({ selectedJobStep: 'x', rawFlowModules: [branchall('B', [[script('x')]])] })
		).toBeNull()
		expect(
			build({ selectedJobStep: 'x', rawFlowModules: [whileloop('W', [script('x')])] })
		).toBeNull()
	})

	it('returns null for a top-level step (handled by the top-level path)', () => {
		expect(build({ selectedJobStep: 'top', rawFlowModules: [script('top')] })).toBeNull()
	})

	it('falls back to a flat path when a subflow’s modules are not loaded yet', () => {
		const r = build({
			selectedJobStep: 'subflow:stop:smid:a',
			rawFlowModules: [subflow('stop', 'f/mid')],
			expandedSubflows: {} // stop not expanded → modules unavailable at level 1
		})
		expect(r).toEqual({
			topStepId: 'stop',
			topBranchOrIterationN: undefined,
			path: [{ step_id: 'smid' }, { step_id: 'a' }],
			iterationCounts: {}
		})
	})
})

describe('findStepPath', () => {
	it('locates a leaf inside a ForLoop and reports the ancestor chain', () => {
		const p = findStepPath([forloop('L', [script('x')])], 'x')
		expect(p?.target.id).toBe('x')
		expect(p?.ancestors).toEqual([{ stepId: 'L', type: 'forloopflow', parallel: false }])
	})
	it('does not cross subflow boundaries', () => {
		expect(findStepPath([subflow('s')], 'a')).toBeUndefined()
	})
})
