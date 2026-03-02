import { expect } from 'vitest'
import type {
	FlowValue,
	FlowModule,
	RawScript,
	Identity,
	ForloopFlow,
	WhileloopFlow,
	BranchOne,
	BranchAll
} from '$lib/gen'
import type { ExtendedOpenFlow } from './types'
import type { StateStore } from '$lib/utils'

// ============================================================================
// Module Creation Helpers
// ============================================================================

/**
 * Creates a minimal RawScript module for testing
 */
export function createRawScriptModule(id: string, content: string): FlowModule {
	return {
		id,
		value: {
			type: 'rawscript',
			content,
			language: 'bun',
			input_transforms: {}
		} as RawScript
	}
}

/**
 * Creates a minimal Identity module (useful for type-change tests)
 */
export function createIdentityModule(id: string): FlowModule {
	return {
		id,
		value: {
			type: 'identity'
		} as Identity
	}
}

/**
 * Creates a ForloopFlow module with nested modules
 */
export function createForloopModule(id: string, nestedModules: FlowModule[]): FlowModule {
	return {
		id,
		value: {
			type: 'forloopflow',
			modules: nestedModules,
			iterator: { type: 'javascript', expr: '[1,2,3]' },
			skip_failures: false
		} as ForloopFlow
	}
}

/**
 * Creates a WhileloopFlow module with nested modules
 */
export function createWhileloopModule(id: string, nestedModules: FlowModule[]): FlowModule {
	return {
		id,
		value: {
			type: 'whileloopflow',
			modules: nestedModules,
			skip_failures: false
		} as WhileloopFlow
	}
}

/**
 * Creates a BranchOne module with default and conditional branches
 */
export function createBranchOneModule(
	id: string,
	defaultModules: FlowModule[],
	branches: { expr: string; modules: FlowModule[] }[]
): FlowModule {
	return {
		id,
		value: {
			type: 'branchone',
			default: defaultModules,
			branches: branches.map((b) => ({ expr: b.expr, modules: b.modules }))
		} as BranchOne
	}
}

/**
 * Creates a BranchAll module with parallel branches
 */
export function createBranchAllModule(id: string, branches: { modules: FlowModule[] }[]): FlowModule {
	return {
		id,
		value: {
			type: 'branchall',
			branches: branches.map((b) => ({ modules: b.modules }))
		} as BranchAll
	}
}

// ============================================================================
// Flow Creation Helpers
// ============================================================================

/**
 * Creates a FlowValue with the given modules
 */
export function createFlow(modules: FlowModule[]): FlowValue {
	return { modules }
}

/**
 * Creates a FlowValue with optional special modules (failure_module, preprocessor_module)
 */
export function createFlowWithSpecialModules(options: {
	modules?: FlowModule[]
	failure_module?: FlowModule
	preprocessor_module?: FlowModule
}): FlowValue {
	return {
		modules: options.modules ?? [],
		...(options.failure_module && { failure_module: options.failure_module }),
		...(options.preprocessor_module && { preprocessor_module: options.preprocessor_module })
	}
}

/**
 * Wraps a FlowValue in an ExtendedOpenFlow for manager tests
 */
export function createExtendedOpenFlow(
	flowValue: FlowValue,
	schema?: Record<string, unknown>
): ExtendedOpenFlow {
	return { value: flowValue, summary: '', schema }
}

/**
 * Creates a minimal StateStore for manager tests
 */
export function createFlowStore(flow: ExtendedOpenFlow): StateStore<ExtendedOpenFlow> {
	return { val: flow }
}

// ============================================================================
// Utility Helpers
// ============================================================================

/**
 * Deep clones an object via JSON serialization
 */
export function deepClone<T>(obj: T): T {
	return JSON.parse(JSON.stringify(obj))
}

/**
 * Asserts that modules appear in the exact order specified.
 * Also implicitly verifies the count of modules.
 */
export function expectModuleOrder(modules: FlowModule[], expectedIds: string[]): void {
	expect(modules.map((m) => m.id)).toEqual(expectedIds)
}
