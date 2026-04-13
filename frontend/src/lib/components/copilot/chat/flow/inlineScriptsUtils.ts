import type { FlowModule } from '$lib/gen'

export interface InlineScriptSession {
	clear(): void
	set(moduleId: string, content: string): void
	get(moduleId: string): string | undefined
	has(moduleId: string): boolean
	getAll(): Record<string, string>
	extractAndReplaceInlineScripts(modules: FlowModule[]): FlowModule[]
	restoreInlineScriptReferences(modules: FlowModule[]): FlowModule[]
	findUnresolvedInlineScriptRefs(modules: FlowModule[]): string[]
}

class DefaultInlineScriptSession implements InlineScriptSession {
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

	extractAndReplaceInlineScripts(modules: FlowModule[]): FlowModule[] {
		return extractAndReplaceInlineScripts(modules, this)
	}

	restoreInlineScriptReferences(modules: FlowModule[]): FlowModule[] {
		return restoreInlineScriptReferences(modules, this)
	}

	findUnresolvedInlineScriptRefs(modules: FlowModule[]): string[] {
		return findUnresolvedInlineScriptRefs(modules)
	}
}

export function createInlineScriptSession(): InlineScriptSession {
	return new DefaultInlineScriptSession()
}

function extractAndReplaceInlineScripts(
	modules: FlowModule[],
	session: Pick<InlineScriptSession, 'set'>
): FlowModule[] {
	if (!modules || !Array.isArray(modules)) {
		return []
	}

	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			session.set(module.id, newModule.value.content)
			newModule.value = {
				...newModule.value,
				content: `inline_script.${module.id}`
			}
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: extractAndReplaceInlineScripts(newModule.value.modules, session)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules, session) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: extractAndReplaceInlineScripts(newModule.value.default, session)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? extractAndReplaceInlineScripts(branch.modules, session) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
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
							session.set(tool.id, tool.value.content as string)
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

function restoreInlineScriptReferences(
	modules: FlowModule[],
	session: Pick<InlineScriptSession, 'get'>
): FlowModule[] {
	return modules.map((module) => {
		const newModule = { ...module }

		if (newModule.value.type === 'rawscript' && newModule.value.content) {
			const match = newModule.value.content.match(/^inline_script\.(.+)$/)
			if (match) {
				const storedContent = session.get(match[1])
				if (storedContent !== undefined) {
					newModule.value = {
						...newModule.value,
						content: storedContent
					}
				}
			}
		} else if (newModule.value.type === 'forloopflow' || newModule.value.type === 'whileloopflow') {
			if (newModule.value.modules) {
				newModule.value = {
					...newModule.value,
					modules: restoreInlineScriptReferences(newModule.value.modules, session)
				}
			}
		} else if (newModule.value.type === 'branchone') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules, session) : []
					}))
				}
			}
			if (newModule.value.default) {
				newModule.value = {
					...newModule.value,
					default: restoreInlineScriptReferences(newModule.value.default, session)
				}
			}
		} else if (newModule.value.type === 'branchall') {
			if (newModule.value.branches) {
				newModule.value = {
					...newModule.value,
					branches: newModule.value.branches.map((branch) => ({
						...branch,
						modules: branch.modules ? restoreInlineScriptReferences(branch.modules, session) : []
					}))
				}
			}
		} else if (newModule.value.type === 'aiagent') {
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
							const match = (tool.value.content as string).match(/^inline_script\.(.+)$/)
							if (match) {
								const storedContent = session.get(match[1])
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

function findUnresolvedInlineScriptRefs(modules: FlowModule[]): string[] {
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
