<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, type Snippet } from 'svelte'
	import type { FlowCopilotContext } from '../../flow'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import AIChat from '../AIChat.svelte'
	import type { FlowModule, OpenFlow, ScriptLang } from '$lib/gen'
	import { getIndexInNestedModules, getNestedModules } from './utils'
	import type { FlowModuleState } from '$lib/components/flows/flowState'
	import { getStringError } from '../utils'
	import type { FlowAIChatHelpers } from './core'
	import {
		insertNewFailureModule,
		insertNewPreprocessorModule
	} from '$lib/components/flows/flowStateUtils.svelte'
	import type { ScriptOptions } from '../ContextManager.svelte'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'

	let {
		flowModuleSchemaMap,
		headerLeft
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
		headerLeft: Snippet
	} = $props()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	const { flowStore, flowStateStore, selectedId, currentEditor, flowInputsStore } =
		flowEditorContext

	const { exprsToSet } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') ?? {}

	function getModule(id: string) {
		if (id === 'preprocessor') {
			return flowStore.value.preprocessor_module
		} else if (id === 'failure') {
			return flowStore.value.failure_module
		} else {
			return dfs(id, flowStore, false)[0]
		}
	}

	function getScriptOptions(id: string): ScriptOptions | undefined {
		const module = getModule(id)

		if (module && module.value.type === 'rawscript') {
			const moduleState: FlowModuleState | undefined = $flowStateStore[module.id]

			const editorRelated =
				$currentEditor && $currentEditor.type === 'script' && $currentEditor.stepId === module.id
					? {
							diffMode: $currentEditor.diffMode,
							lastDeployedCode: $currentEditor.lastDeployedCode,
							lastSavedCode: undefined
						}
					: {
							diffMode: false,
							lastDeployedCode: undefined,
							lastSavedCode: undefined
						}

			return {
				args: moduleState?.previewArgs ?? {},
				error:
					moduleState && !moduleState.previewSuccess
						? getStringError(moduleState.previewResult)
						: undefined,
				code: module.value.content,
				lang: module.value.language,
				path: module.id,
				...editorRelated
			}
		}

		return undefined
	}

	let scriptOptions = $derived.by(() => getScriptOptions($selectedId))

	const flowHelpers: FlowAIChatHelpers & {
		getFlowAndSelectedId: () => { flow: OpenFlow; selectedId: string }
	} = {
		getFlowAndSelectedId: () => ({ flow: flowStore, selectedId: $selectedId }),
		setCode: async (id, code) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type === 'rawscript') {
				module.value.content = code
				const { input_transforms, schema } = await loadSchemaFromModule(module)
				module.value.input_transforms = input_transforms
				flowEditorContext.flowStore = flowStore

				if ($flowStateStore[id]) {
					$flowStateStore[id].schema = schema
				} else {
					$flowStateStore[id] = {
						schema
					}
				}
			} else {
				throw new Error('Module is not a rawscript or script')
			}
			if ($currentEditor && $currentEditor.type === 'script' && $currentEditor.stepId === id) {
				$currentEditor.editor.setCode(code)
			}
		},
		insertStep: async (location, step) => {
			const { index, modules } =
				location.type === 'start'
					? {
							index: -1,
							modules: flowStore.value.modules
						}
					: location.type === 'start_inside_forloop'
						? {
								index: -1,
								modules: getNestedModules(flowStore, location.inside)
							}
						: location.type === 'start_inside_branch'
							? {
									index: -1,
									modules: getNestedModules(flowStore, location.inside, location.branchIndex)
								}
							: location.type === 'after'
								? getIndexInNestedModules(flowStore, location.afterId)
								: {
										index: -1,
										modules: flowStore.value.modules
									}

			const indexToInsertAt = index + 1

			let newModules: FlowModule[] | undefined = undefined
			switch (step.type) {
				case 'rawscript': {
					const inlineScript = {
						language: step.language,
						kind: 'script' as const,
						subkind: 'flow' as const
					}
					if (location.type === 'preprocessor') {
						await insertNewPreprocessorModule(flowStore, flowStateStore, inlineScript)
					} else if (location.type === 'failure') {
						await insertNewFailureModule(flowStore, flowStateStore, inlineScript)
					} else {
						newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
							modules,
							indexToInsertAt,
							'script',
							undefined,
							undefined,
							inlineScript
						)
					}
					break
				}
				case 'script': {
					const wsScript = {
						path: step.path,
						summary: '',
						hash: undefined
					}
					if (location.type === 'preprocessor') {
						await insertNewPreprocessorModule(flowStore, flowStateStore, undefined, wsScript)
					} else if (location.type === 'failure') {
						await insertNewFailureModule(flowStore, flowStateStore, undefined, wsScript)
					} else {
						newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
							modules,
							indexToInsertAt,
							'script',
							wsScript
						)
					}
					break
				}
				case 'forloop':
				case 'branchall':
				case 'branchone': {
					if (location.type === 'preprocessor' || location.type === 'failure') {
						throw new Error('Cannot insert a non-script module for preprocessing or error handling')
					}
					newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
						modules,
						indexToInsertAt,
						step.type
					)
					break
				}
				default: {
					throw new Error('Unknown step type')
				}
			}

			if (location.type === 'preprocessor' || location.type === 'failure') {
				$flowStateStore = $flowStateStore
				flowEditorContext.flowStore = flowStore

				return location.type
			} else {
				const newModule = newModules?.[indexToInsertAt]

				if (!newModule) {
					throw new Error('Failed to insert module')
				}

				if (['branchone', 'branchall'].includes(step.type)) {
					await flowModuleSchemaMap?.addBranch(newModule)
				}

				$flowStateStore = $flowStateStore
				flowEditorContext.flowStore = flowStore

				return newModule.id
			}
		},
		removeStep: async (id) => {
			flowModuleSchemaMap?.selectNextId(id)
			if (id === 'preprocessor') {
				flowStore.value.preprocessor_module = undefined
			} else if (id === 'failure') {
				flowStore.value.failure_module = undefined
			} else {
				const { modules } = getIndexInNestedModules(flowStore, id)
				flowModuleSchemaMap?.removeAtId(modules, id)
			}

			if ($flowInputsStore) {
				delete $flowInputsStore[id]
			}

			flowEditorContext.flowStore = flowStore

			flowModuleSchemaMap?.updateFlowInputsStore()
		},
		getStepInputs: async (id) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			const inputs =
				module.value.type === 'script' || module.value.type === 'rawscript'
					? module.value.input_transforms
					: {}

			return inputs
		},
		setStepInputs: async (id, inputs) => {
			if (id === 'preprocessor') {
				throw new Error('Cannot set inputs for preprocessor')
			}

			const regex = /\[\[(.+?)\]\]\s*\n([\s\S]*?)(?=\n\[\[|$)/g

			const parsedInputs = Array.from(inputs.matchAll(regex)).map((match) => ({
				input: match[1],
				value: match[2].trim()
			}))

			if (id === $selectedId) {
				exprsToSet?.set({})
				const argsToUpdate = {}
				for (const { input, value } of parsedInputs) {
					argsToUpdate[input] = {
						type: 'javascript',
						expr: value
					}
				}
				exprsToSet?.set(argsToUpdate)
			} else {
				const module = getModule(id)
				if (!module) {
					throw new Error('Module not found')
				}

				if (module.value.type !== 'script' && module.value.type !== 'rawscript') {
					throw new Error('Module is not a script or rawscript')
				}

				for (const { input, value } of parsedInputs) {
					module.value.input_transforms[input] = {
						type: 'javascript',
						expr: value
					}
				}
				flowEditorContext.flowStore = flowStore
			}
		},
		getFlowInputsSchema: async () => {
			return flowStore.schema ?? {}
		},
		setFlowInputsSchema: async (newInputs) => {
			flowStore.schema = newInputs
		},
		selectStep: (id) => {
			$selectedId = id
		},
		getStepCode: (id) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type === 'rawscript') {
				return module.value.content
			} else {
				throw new Error('Module is not a rawscript')
			}
		},
		getModules: (id?: string) => {
			if (id) {
				const module = getModule(id)

				if (!module) {
					throw new Error('Module not found')
				}

				return getSubModules(module).flat()
			}
			return flowStore.value.modules
		},
		setBranchPredicate: async (id, branchIndex, expression) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type !== 'branchone') {
				throw new Error('Module is not a branchall or branchone')
			}
			const branch = module.value.branches[branchIndex]
			if (!branch) {
				throw new Error('Branch not found')
			}
			branch.expr = expression
			flowEditorContext.flowStore = flowStore
		},
		addBranch: async (id) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type !== 'branchall' && module.value.type !== 'branchone') {
				throw new Error('Module is not a branchall or branchone')
			}
			flowModuleSchemaMap?.addBranch(module)
			flowEditorContext.flowStore = flowStore
		},
		removeBranch: async (id, branchIndex) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type !== 'branchall' && module.value.type !== 'branchone') {
				throw new Error('Module is not a branchall or branchone')
			}

			// for branch one, we set index + 1 because the removeBranch function assumes the index is shifted by 1 because of the default branch
			flowModuleSchemaMap?.removeBranch(
				module,
				module.value.type === 'branchone' ? branchIndex + 1 : branchIndex
			)
			flowEditorContext.flowStore = flowStore
		},
		setForLoopIteratorExpression: async (id, expression) => {
			if ($currentEditor && $currentEditor.type === 'iterator' && $currentEditor.stepId === id) {
				$currentEditor.editor.setCode(expression)
			} else {
				const module = getModule(id)
				if (!module) {
					throw new Error('Module not found')
				}
				if (module.value.type !== 'forloopflow') {
					throw new Error('Module is not a forloopflow')
				}
				module.value.iterator = { type: 'javascript', expr: expression }
				flowEditorContext.flowStore = flowStore
			}
		}
	}

	let aiChat: AIChat | undefined = undefined

	export async function generateStep(moduleId: string, lang: ScriptLang, instructions: string) {
		flowHelpers.selectStep(moduleId)
		aiChat?.sendRequest({
			instructions: instructions,
			mode: 'script',
			lang: lang,
			isPreprocessor: moduleId === 'preprocessor'
		})
	}

	export function addSelectedLinesToContext(lines: string, startLine: number, endLine: number) {
		aiChat?.addSelectedLinesToContext(lines, startLine, endLine)
	}
</script>

<AIChat
	bind:this={aiChat}
	{headerLeft}
	{scriptOptions}
	{flowHelpers}
	showDiffMode={() => {
		if ($currentEditor && $currentEditor.type === 'script') {
			$currentEditor.showDiffMode()
		}
	}}
	applyCode={(code: string) => {
		if ($currentEditor && $currentEditor.type === 'script') {
			$currentEditor.hideDiffMode()
			$currentEditor.editor.reviewAndApplyCode(code)
		}
	}}
/>
