<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext } from 'svelte'
	import type { FlowCopilotContext } from '../../flow'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import type { FlowModule } from '$lib/gen'
	import { getIndexInNestedModules, getNestedModules } from './utils'
	import type { FlowAIChatHelpers } from './core'
	import {
		insertNewFailureModule,
		insertNewPreprocessorModule
	} from '$lib/components/flows/flowStateUtils.svelte'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'

	let {
		flowModuleSchemaMap
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
	} = $props()

	const { flowStore, flowStateStore, selectedId, currentEditor, flowInputsStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const { exprsToSet } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') ?? {}

	function getModule(id: string) {
		if (id === 'preprocessor') {
			return flowStore.val.value.preprocessor_module
		} else if (id === 'failure') {
			return flowStore.val.value.failure_module
		} else {
			return dfs(id, flowStore.val, false)[0]
		}
	}

	const flowHelpers: FlowAIChatHelpers = {
		getFlowAndSelectedId: () => ({ flow: flowStore.val, selectedId: $selectedId }),
		setCode: async (id, code) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type === 'rawscript') {
				module.value.content = code
				const { input_transforms, schema } = await loadSchemaFromModule(module)
				module.value.input_transforms = input_transforms
				refreshStateStore(flowStore)

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
							modules: flowStore.val.value.modules
						}
					: location.type === 'start_inside_forloop'
						? {
								index: -1,
								modules: getNestedModules(flowStore.val, location.inside)
							}
						: location.type === 'start_inside_branch'
							? {
									index: -1,
									modules: getNestedModules(flowStore.val, location.inside, location.branchIndex)
								}
							: location.type === 'after'
								? getIndexInNestedModules(flowStore.val, location.afterId)
								: {
										index: -1,
										modules: flowStore.val.value.modules
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
				refreshStateStore(flowStore)

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
				refreshStateStore(flowStore)

				return newModule.id
			}
		},
		removeStep: async (id) => {
			flowModuleSchemaMap?.selectNextId(id)
			if (id === 'preprocessor') {
				flowStore.val.value.preprocessor_module = undefined
			} else if (id === 'failure') {
				flowStore.val.value.failure_module = undefined
			} else {
				const { modules } = getIndexInNestedModules(flowStore.val, id)
				flowModuleSchemaMap?.removeAtId(modules, id)
			}

			if ($flowInputsStore) {
				delete $flowInputsStore[id]
			}

			refreshStateStore(flowStore)

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
				refreshStateStore(flowStore)
			}
		},
		getFlowInputsSchema: async () => {
			return flowStore.val.schema ?? {}
		},
		setFlowInputsSchema: async (newInputs) => {
			flowStore.val.schema = newInputs
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
			return flowStore.val.value.modules
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
			refreshStateStore(flowStore)
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
			refreshStateStore(flowStore)
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
			refreshStateStore(flowStore)
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
				refreshStateStore(flowStore)
			}
		}
	}

	$effect(() => {
		const cleanup = aiChatManager.setFlowHelpers(flowHelpers)
		return cleanup
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForSelectedIdChanges(
			$selectedId,
			flowStore.val,
			$flowStateStore,
			$currentEditor
		)
		return cleanup
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForCurrentEditorChanges($currentEditor)
		return cleanup
	})
</script>
