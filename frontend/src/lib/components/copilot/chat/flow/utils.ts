import { dfs } from '$lib/components/flows/previousResults'
import type { FlowModule, OpenFlow } from '$lib/gen'

// Helper to find module by ID in a flow
export function getModuleById(flow: OpenFlow, moduleId: string): FlowModule | undefined {
	const allModules = dfs(moduleId, flow, false)
	return allModules[0]
}

export function getIndexInNestedModules(
	flow: OpenFlow,
	id: string
): { index: number; modules: FlowModule[] } | null {
	const accessingModules = dfs(id, flow, true).reverse()

	if (accessingModules.length === 0) {
		// Module not found in flow
		return null
	}

	let parent = flow.value.modules
	let lastIndex = -1
	for (const [ai, am] of accessingModules.entries()) {
		const index = parent.findIndex((m) => m.id === am.id)

		if (index === -1) {
			// Module no longer exists in expected location (may have been deleted with parent)
			return null
		}

		lastIndex = index

		if (ai === accessingModules.length - 1) {
			break
		}

		if (
			parent[index].value.type === 'forloopflow' ||
			parent[index].value.type === 'whileloopflow'
		) {
			parent = parent[index].value.modules
		} else if (
			parent[index].value.type === 'branchall' ||
			parent[index].value.type === 'branchone'
		) {
			const branchIdx = parent[index].value.branches.findIndex((b) =>
				b.modules.some((m) => m.id === accessingModules[ai + 1].id)
			)
			if (branchIdx === -1) {
				// Module no longer exists in branch (may have been deleted)
				return null
			}
			parent = parent[index].value.branches[branchIdx].modules
		} else {
			// Unexpected module type in path
			return null
		}
	}

	if (lastIndex === -1) {
		return null
	}

	return {
		index: lastIndex,
		modules: parent
	}
}
export function getNestedModules(flow: OpenFlow, id: string, branchIndex?: number) {
	const result = getIndexInNestedModules(flow, id)
	if (!result) {
		throw new Error(`Module not found: ${id}`)
	}
	const { index, modules } = result

	// we know index is correct because we've already checked it in getIndexInNestedModules
	const module = modules[index]

	if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
		return module.value.modules
	} else if (
		branchIndex !== undefined &&
		(module.value.type === 'branchall' || module.value.type === 'branchone')
	) {
		if (module.value.type === 'branchone' && branchIndex === -1) {
			return module.value.default
		}

		const branch = module.value.branches[branchIndex]

		if (!branch) {
			throw new Error(
				`Branch not found: ${id} in ${module.value.branches.map((b) => b.modules.map((m) => m.id).join(', ')).join(';')}`
			)
		}

		return branch.modules
	} else if (module.value.type === 'aiagent') {
		return module.value.tools
	} else {
		throw new Error('Module is not a loop or branch')
	}
}

/**
 * Recursively resolves all $ref references in a JSON Schema by inlining them.
 * This ensures the schema is fully self-contained for AI providers that don't
 * support external references or have strict schema validation (e.g., Google/Gemini).
 *
 * @param schema - The schema object to resolve
 * @param rootSchema - The root schema document containing all definitions
 * @param visited - Set of visited $ref paths to prevent infinite recursion
 * @returns Fully resolved schema with all $ref references inlined
 */
export function resolveSchemaRefs(schema: any, rootSchema: any, visited = new Set<string>()): any {
	if (!schema || typeof schema !== 'object') return schema

	// Handle $ref
	if (schema.$ref) {
		const refPath = schema.$ref.replace('#/', '').split('/')

		// Prevent infinite recursion with circular refs
		if (visited.has(schema.$ref)) {
			return { type: 'object' } // Fallback for circular refs
		}
		visited.add(schema.$ref)

		let resolved = rootSchema
		for (const part of refPath) {
			resolved = resolved[part]
		}

		// Recursively resolve the referenced schema
		return resolveSchemaRefs(resolved, rootSchema, new Set(visited))
	}

	// Handle arrays
	if (Array.isArray(schema)) {
		return schema.map((item) => resolveSchemaRefs(item, rootSchema, visited))
	}

	// Handle objects - recursively process all properties
	const result: any = {}
	for (const key in schema) {
		result[key] = resolveSchemaRefs(schema[key], rootSchema, visited)
	}
	return result
}

/**
 * Recursively collects all module IDs from a module and its nested structures
 */
export function collectAllModuleIds(module: FlowModule): string[] {
	const ids: string[] = [module.id]

	if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
		if (module.value.modules) {
			for (const nested of module.value.modules) {
				ids.push(...collectAllModuleIds(nested))
			}
		}
	} else if (module.value.type === 'branchone') {
		if (module.value.branches) {
			for (const branch of module.value.branches) {
				if (branch.modules) {
					for (const nested of branch.modules) {
						ids.push(...collectAllModuleIds(nested))
					}
				}
			}
		}
		if (module.value.default) {
			for (const nested of module.value.default) {
				ids.push(...collectAllModuleIds(nested))
			}
		}
	} else if (module.value.type === 'branchall') {
		if (module.value.branches) {
			for (const branch of module.value.branches) {
				if (branch.modules) {
					for (const nested of branch.modules) {
						ids.push(...collectAllModuleIds(nested))
					}
				}
			}
		}
	} else if (module.value.type === 'aiagent') {
		if (module.value.tools) {
			for (const tool of module.value.tools) {
				ids.push(tool.id)
			}
		}
	}

	return ids
}

/**
 * Recursively finds a module by ID in the flow structure
 */
export function findModuleInFlow(modules: FlowModule[], id: string): FlowModule | undefined {
	for (const module of modules) {
		if (module.id === id) {
			return module
		}

		// Search in nested structures
		if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			if (module.value.modules) {
				const found = findModuleInFlow(module.value.modules, id)
				if (found) return found
			}
		} else if (module.value.type === 'branchone') {
			if (module.value.branches) {
				for (const branch of module.value.branches) {
					if (branch.modules) {
						const found = findModuleInFlow(branch.modules, id)
						if (found) return found
					}
				}
			}
			if (module.value.default) {
				const found = findModuleInFlow(module.value.default, id)
				if (found) return found
			}
		} else if (module.value.type === 'branchall') {
			if (module.value.branches) {
				for (const branch of module.value.branches) {
					if (branch.modules) {
						const found = findModuleInFlow(branch.modules, id)
						if (found) return found
					}
				}
			}
		} else if (module.value.type === 'aiagent') {
			// Search in AI agent tools
			if (module.value.tools) {
				for (const tool of module.value.tools) {
					if (tool.id === id) {
						// Return a pseudo-FlowModule for compatibility
						return { id: tool.id, value: tool.value, summary: tool.summary } as FlowModule
					}
				}
			}
		}
	}
	return undefined
}

/**
 * Recursively removes a module by ID from the flow structure
 * Returns the updated modules array
 */
export function removeModuleFromFlow(modules: FlowModule[], id: string): FlowModule[] {
	const result: FlowModule[] = []

	for (const module of modules) {
		if (module.id === id) {
			// Skip this module (remove it)
			continue
		}

		const newModule = { ...module }

		// Recursively remove from nested structures
		if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: removeModuleFromFlow(newModule.value.modules, id)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? removeModuleFromFlow(branch.modules, id) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: removeModuleFromFlow(newModule.value.default, id)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? removeModuleFromFlow(branch.modules, id) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
			// Remove tool from AI agent's tools array
			if (newModule.value.tools) {
				newModule.value = {
					...newModule.value,
					tools: newModule.value.tools.filter((tool) => tool.id !== id)
				}
			}
		}

		result.push(newModule)
	}

	return result
}

/**
 * Recursively removes a branch by index from a branchall/branchone container
 * Returns the updated modules array
 */
export function removeBranchFromFlow(
	modules: FlowModule[],
	containerId: string,
	branchIndex: number
): FlowModule[] {
	return modules.map((module) => {
		if (module.id === containerId) {
			if (module.value.type === 'branchall') {
				const branches = module.value.branches || []
				if (branchIndex < 0 || branchIndex >= branches.length) {
					throw new Error(`Branch index ${branchIndex} out of bounds (0-${branches.length - 1})`)
				}
				return {
					...module,
					value: {
						...module.value,
						branches: branches.filter((_, i) => i !== branchIndex)
					}
				} as FlowModule
			}
			if (module.value.type === 'branchone') {
				const branches = module.value.branches || []
				if (branchIndex < 0 || branchIndex >= branches.length) {
					throw new Error(`Branch index ${branchIndex} out of bounds (0-${branches.length - 1})`)
				}
				return {
					...module,
					value: {
						...module.value,
						branches: branches.filter((_, i) => i !== branchIndex)
					}
				} as FlowModule
			}
			throw new Error(`Module '${containerId}' is not a branchall/branchone container`)
		}

		// Recursively search nested structures
		const newModule = { ...module }
		if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: removeBranchFromFlow(newModule.value.modules, containerId, branchIndex)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules
							? removeBranchFromFlow(branch.modules, containerId, branchIndex)
							: []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: removeBranchFromFlow(newModule.value.default, containerId, branchIndex)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules
							? removeBranchFromFlow(branch.modules, containerId, branchIndex)
							: []
					}))
				}
			}
		}

		return newModule
	})
}

/**
 * Parses a branch path string into navigation components
 * Examples: 'branches.0' -> {type: 'branches', index: 0}
 *           'default' -> {type: 'default'}
 *           'modules' -> {type: 'modules'}
 */
export function parseBranchPath(path: string): { type: string; index?: number } {
	if (path === 'default') {
		return { type: 'default' }
	}
	if (path === 'modules') {
		return { type: 'modules' }
	}
	if (path === 'tools') {
		return { type: 'tools' }
	}

	const match = path.match(/^(branches)\.(\d+)$/)
	if (match) {
		return { type: match[1], index: parseInt(match[2], 10) }
	}

	throw new Error(`Invalid branch path: ${path}`)
}

/**
 * Gets the target array for module insertion based on insideId and branchPath
 */
function getTargetArray(
	modules: FlowModule[],
	insideId: string,
	branchPath: string
): FlowModule[] | undefined {
	const container = findModuleInFlow(modules, insideId)
	if (!container) {
		return undefined
	}

	const parsed = parseBranchPath(branchPath)

	if (container.value.type === 'forloopflow' || container.value.type === 'whileloopflow') {
		if (parsed.type === 'modules') {
			return container.value.modules || []
		}
		throw new Error(`Invalid branchPath '${branchPath}' for loop module. Use 'modules'`)
	} else if (container.value.type === 'branchone') {
		if (parsed.type === 'branches' && parsed.index !== undefined) {
			return container.value.branches?.[parsed.index]?.modules
		} else if (parsed.type === 'default') {
			return container.value.default
		}
		throw new Error(
			`Invalid branchPath '${branchPath}' for branchone module. Use 'branches.N' or 'default'`
		)
	} else if (container.value.type === 'branchall') {
		if (parsed.type === 'branches' && parsed.index !== undefined) {
			return container.value.branches?.[parsed.index]?.modules
		}
		throw new Error(`Invalid branchPath '${branchPath}' for branchall module. Use 'branches.N'`)
	} else if (container.value.type === 'aiagent') {
		if (parsed.type === 'tools') {
			// Return tools array (AgentTool[]), caller handles the different structure
			return (container.value.tools as any) || []
		}
		throw new Error(`Invalid branchPath '${branchPath}' for aiagent module. Use 'tools'`)
	}

	throw new Error(`Module '${insideId}' is not a container type`)
}

/**
 * Updates a nested array within a container module
 */
function updateNestedArray(
	module: FlowModule,
	branchPath: string,
	updatedArray: FlowModule[]
): FlowModule {
	const parsed = parseBranchPath(branchPath)
	const newModule = { ...module }

	if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
		if (parsed.type === 'modules') {
			newModule.value = {
				...newModule.value,
				modules: updatedArray
			}
		}
	} else if (newModule.value.type === 'branchone') {
		if (parsed.type === 'branches' && parsed.index !== undefined && newModule.value.branches) {
			const newBranches = [...newModule.value.branches]
			newBranches[parsed.index] = {
				...newBranches[parsed.index],
				modules: updatedArray
			}
			newModule.value = {
				...newModule.value,
				branches: newBranches
			}
		} else if (parsed.type === 'default') {
			newModule.value = {
				...newModule.value,
				default: updatedArray
			}
		}
	} else if (newModule.value.type === 'branchall') {
		if (parsed.type === 'branches' && parsed.index !== undefined && newModule.value.branches) {
			const newBranches = [...newModule.value.branches]
			newBranches[parsed.index] = {
				...newBranches[parsed.index],
				modules: updatedArray
			}
			newModule.value = {
				...newModule.value,
				branches: newBranches
			}
		}
	} else if (newModule.value.type === 'aiagent') {
		if (parsed.type === 'tools') {
			// Note: updatedArray is actually AgentTool[] when dealing with AI agents
			newModule.value = {
				...newModule.value,
				tools: updatedArray as any
			}
		}
	}

	return newModule
}

/**
 * Recursively adds a module to the flow structure
 */
export function addModuleToFlow(
	modules: FlowModule[],
	afterId: string | null,
	insideId: string | null,
	branchPath: string | null,
	newModule: FlowModule
): FlowModule[] {
	// Case 1a: Adding a NEW branch to branchall/branchone (insideId set, branchPath null)
	if (insideId && branchPath === null) {
		return modules.map((module) => {
			if (module.id === insideId) {
				// Adding a new branch to branchall
				if (module.value.type === 'branchall') {
					const newBranch = {
						summary: (newModule as any).summary || '',
						skip_failure: (newModule as any).skip_failure ?? false,
						modules: (newModule as any).modules || []
					}
					return {
						...module,
						value: {
							...module.value,
							branches: [...(module.value.branches || []), newBranch]
						}
					} as FlowModule
				}
				// Adding a new branch to branchone
				if (module.value.type === 'branchone') {
					const newBranch = {
						summary: (newModule as any).summary || '',
						expr: (newModule as any).expr || 'false',
						modules: (newModule as any).modules || []
					}
					return {
						...module,
						value: {
							...module.value,
							branches: [...(module.value.branches || []), newBranch]
						}
					} as FlowModule
				}
				throw new Error(
					`Cannot add branch to module '${insideId}': branchPath=null is only valid for branchall/branchone containers`
				)
			}

			// Recursively search nested structures for the target container
			const newModuleCopy = { ...module }
			if (
				newModuleCopy.value.type === 'forloopflow' ||
				newModuleCopy.value.type === 'whileloopflow'
			) {
				if (newModuleCopy.value.modules) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						modules: addModuleToFlow(
							newModuleCopy.value.modules,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchone') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
				if (newModuleCopy.value.default) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						default: addModuleToFlow(
							newModuleCopy.value.default,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchall') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
			}
			return newModuleCopy
		})
	}

	// Case 1b: Adding inside a container (insideId + branchPath both set)
	if (insideId && branchPath) {
		return modules.map((module) => {
			if (module.id === insideId) {
				// Special handling for AI agent tools
				if (module.value.type === 'aiagent' && branchPath === 'tools') {
					// For AI agents, newModule structure is { id, summary, value: { tool_type, ...FlowModuleValue } }
					// The value should already include tool_type from the caller
					const newTool = {
						id: newModule.id,
						summary: newModule.summary,
						value: newModule.value as any
					}
					return {
						...module,
						value: {
							...module.value,
							tools: [...(module.value.tools || []), newTool]
						}
					} as FlowModule
				}

				const targetArray = getTargetArray(modules, insideId, branchPath)
				if (!targetArray) {
					throw new Error(
						`Cannot find target array for insideId '${insideId}' with branchPath '${branchPath}'`
					)
				}
				const updatedArray =
					afterId !== null
						? addModuleToFlow(targetArray, afterId, null, null, newModule)
						: [newModule, ...targetArray] // afterId null = insert at beginning
				return updateNestedArray(module, branchPath, updatedArray)
			}

			// Recursively search nested structures
			const newModuleCopy = { ...module }
			if (
				newModuleCopy.value.type === 'forloopflow' ||
				newModuleCopy.value.type === 'whileloopflow'
			) {
				if (newModuleCopy.value.modules) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						modules: addModuleToFlow(
							newModuleCopy.value.modules,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchone') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
				if (newModuleCopy.value.default) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						default: addModuleToFlow(
							newModuleCopy.value.default,
							afterId,
							insideId,
							branchPath,
							newModule
						)
					}
				}
			} else if (newModuleCopy.value.type === 'branchall') {
				if (newModuleCopy.value.branches) {
					newModuleCopy.value = {
						...newModuleCopy.value,
						branches: newModuleCopy.value.branches.map((branch) => ({
							...branch,
							modules: branch.modules
								? addModuleToFlow(branch.modules, afterId, insideId, branchPath, newModule)
								: []
						}))
					}
				}
			}

			return newModuleCopy
		})
	}

	// Case 2: Adding at current level after a specific module
	if (afterId !== null) {
		const result: FlowModule[] = []
		for (const module of modules) {
			result.push(module)
			if (module.id === afterId) {
				result.push(newModule)
			}
		}
		return result
	}

	// Case 3: afterId is null - insert at the beginning
	return [newModule, ...modules]
}

/**
 * Recursively replaces a module by ID
 */
export function replaceModuleInFlow(
	modules: FlowModule[],
	id: string,
	newModule: FlowModule
): FlowModule[] {
	return modules.map((module) => {
		if (module.id === id) {
			return { ...newModule, id } // Ensure ID remains the same
		}

		const newModuleCopy = { ...module }

		// Recursively replace in nested structures
		if (
			newModuleCopy.value.type === 'forloopflow' ||
			newModuleCopy.value.type === 'whileloopflow'
		) {
			if (newModuleCopy.value.modules) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					modules: replaceModuleInFlow(newModuleCopy.value.modules, id, newModule)
				}
			}
		} else if (newModuleCopy.value.type === 'branchone') {
			if (newModuleCopy.value.branches) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					branches: newModuleCopy.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? replaceModuleInFlow(branch.modules, id, newModule) : []
					}))
				}
			}
			if (newModuleCopy.value.default) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					default: replaceModuleInFlow(newModuleCopy.value.default, id, newModule)
				}
			}
		} else if (newModuleCopy.value.type === 'branchall') {
			if (newModuleCopy.value.branches) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					branches: newModuleCopy.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? replaceModuleInFlow(branch.modules, id, newModule) : []
					}))
				}
			}
		} else if (newModuleCopy.value.type === 'aiagent') {
			// Replace tool in AI agent's tools array
			if (newModuleCopy.value.tools) {
				newModuleCopy.value = {
					...newModuleCopy.value,
					tools: newModuleCopy.value.tools.map((tool) =>
						tool.id === id
							? { id, summary: newModule.summary, value: newModule.value as any }
							: tool
					)
				}
			}
		}

		return newModuleCopy
	})
}
