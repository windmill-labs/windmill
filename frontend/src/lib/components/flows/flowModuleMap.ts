import type { FlowModule } from '$lib/gen'
import { writable } from 'svelte/store'

export type FlowModuleMap = Record<
	string,
	| {
			// Pointer to parent module, only defined within Branches or Loops
			parentModuleId?: string
			// Pointer to previous module, for easy access to testing results
			previousModuleId?: string
			flowModule: FlowModule
	  }
	| undefined
>

/**
 * This store is an other way to view the FlowStore:
 * We can access any leaf module and relative positions (parentModuleId, previousModuleId) by its id.
 */
export const flowModuleMap = writable<FlowModuleMap>({})
