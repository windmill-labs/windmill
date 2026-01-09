import type { FlowModule } from '$lib/gen'

/**
 * Storage for inline scripts extracted from flow modules.
 * Maps module IDs to their rawscript content for token-efficient transmission to AI.
 */
class InlineScriptStore {
	private scripts: Map<string, string> = new Map()

	clear() {
		this.scripts.clear()
	}

	set(moduleId: string, content: string) {
		this.scripts.set(moduleId, content)
	}

	get(moduleId: string): string | undefined {
		return this.scripts.get(moduleId)
	}

	has(moduleId: string): boolean {
		return this.scripts.has(moduleId)
	}

	getAll(): Record<string, string> {
		return Object.fromEntries(this.scripts.entries())
	}
}

export const inlineScriptStore = new InlineScriptStore()

/**
 * Recursively extracts all rawscript content from flow modules and stores them.
 * Replaces the content with references like "inline_script.{module_id}".
 */
export function extractAndReplaceInlineScripts(modules: FlowModule[]): FlowModule[] {
	if (!modules || !Array.isArray(modules)) {
		return []
	}

	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			// Store the original content
			inlineScriptStore.set(module.id, newModule.value.content)

			// Replace with reference
			newModule.value = {
				...newModule.value,
				content: `inline_script.${module.id}`
			}
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			// Recursively process nested modules in loops
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: extractAndReplaceInlineScripts(newModule.value.modules)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			// Process branches and default modules
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: extractAndReplaceInlineScripts(newModule.value.default)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			// Process all branches
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
			// Process AI agent tools
			if (newModule.value.tools) {
				newModule.value = {
					...newModule.value,
					tools: newModule.value.tools.map((tool) => {
						if (
							tool.value &&
							'tool_type' in tool.value &&
							tool.value.tool_type === 'flowmodule' &&
							'type' in tool.value &&
							tool.value.type === 'rawscript' &&
							'content' in tool.value &&
							tool.value.content
						) {
							inlineScriptStore.set(tool.id, tool.value.content as string)
							return {
								...tool,
								value: {
									...tool.value,
									content: `inline_script.${tool.id}`
								}
							}
						}
						return tool
					})
				}
			}
		}

		return newModule
	})
}

/**
 * Recursively restores inline script references back to their full content.
 * If content matches pattern "inline_script.{id}", looks up and restores the original.
 * If content doesn't match (new/modified script), keeps it as-is.
 */
export function restoreInlineScriptReferences(modules: FlowModule[]): FlowModule[] {
	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			const content = newModule.value.content
			// Check if it's a reference
			const match = content.match(/^inline_script\.(.+)$/)
			if (match) {
				const moduleId = match[1]
				const storedContent = inlineScriptStore.get(moduleId)
				if (storedContent !== undefined) {
					// Restore original content
					newModule.value = {
						...newModule.value,
						content: storedContent
					}
				}
				// If not found in store, keep the reference as-is (shouldn't happen normally)
			}
			// If not a reference, it's new/modified content - keep as-is
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			// Recursively process nested modules in loops
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: restoreInlineScriptReferences(newModule.value.modules)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			// Process branches and default modules
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: restoreInlineScriptReferences(newModule.value.default)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			// Process all branches
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
			// Process AI agent tools
			if (newModule.value.tools) {
				newModule.value = {
					...newModule.value,
					tools: newModule.value.tools.map((tool) => {
						if (
							tool.value &&
							'tool_type' in tool.value &&
							tool.value.tool_type === 'flowmodule' &&
							'type' in tool.value &&
							tool.value.type === 'rawscript' &&
							'content' in tool.value &&
							tool.value.content
						) {
							const content = tool.value.content as string
							const match = content.match(/^inline_script\.(.+)$/)
							if (match) {
								const toolId = match[1]
								const storedContent = inlineScriptStore.get(toolId)
								if (storedContent !== undefined) {
									return {
										...tool,
										value: {
											...tool.value,
											content: storedContent
										}
									}
								}
							}
						}
						return tool
					})
				}
			}
		}

		return newModule
	})
}

/**
 * Recursively finds any unresolved inline script references in flow modules.
 * Returns array of module IDs that still have `inline_script.{id}` patterns.
 */
export function findUnresolvedInlineScriptRefs(modules: FlowModule[]): string[] {
	const unresolvedRefs: string[] = []

	function checkModule(module: FlowModule) {
		if (module.value.type === 'rawscript' && module.value.content) {
			const match = module.value.content.match(/^inline_script\.(.+)$/)
			if (match) {
				unresolvedRefs.push(match[1])
			}
		} else if (module.value.type === 'forloopflow' || module.value.type === 'whileloopflow') {
			if (module.value.modules) {
				module.value.modules.forEach(checkModule)
			}
		} else if (module.value.type === 'branchone') {
			if (module.value.branches) {
				module.value.branches.forEach((branch) => {
					branch.modules?.forEach(checkModule)
				})
			}
			if (module.value.default) {
				module.value.default.forEach(checkModule)
			}
		} else if (module.value.type === 'branchall') {
			if (module.value.branches) {
				module.value.branches.forEach((branch) => {
					branch.modules?.forEach(checkModule)
				})
			}
		} else if (module.value.type === 'aiagent') {
			// Check AI agent tools
			if (module.value.tools) {
				for (const tool of module.value.tools) {
					if (
						tool.value &&
						'tool_type' in tool.value &&
						tool.value.tool_type === 'flowmodule' &&
						'type' in tool.value &&
						tool.value.type === 'rawscript' &&
						'content' in tool.value &&
						tool.value.content
					) {
						const match = (tool.value.content as string).match(/^inline_script\.(.+)$/)
						if (match) {
							unresolvedRefs.push(match[1])
						}
					}
				}
			}
		}
	}

	modules.forEach(checkModule)
	return unresolvedRefs
}
