<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, type Snippet } from 'svelte'
	import type { FlowCopilotContext } from '../../flow'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import AIChat from '../AIChat.svelte'
	import type { FlowModule } from '$lib/gen'
	import { getIndexInNestedModules, getNestedModules } from './utils'
	import type { FlowModuleState } from '$lib/components/flows/flowState'
	import { getStringError } from '../utils'

	let {
		flowModuleSchemaMap,
		headerLeft
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
		headerLeft: Snippet
	} = $props()

	const { flowStore, flowStateStore, selectedId, currentEditor, flowInputsStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const { exprsToSet } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') ?? {}

	function getScriptOptions() {
		const module = dfs($selectedId, $flowStore, false)[0]

		if (
			module &&
			module.value.type === 'rawscript' &&
			$currentEditor &&
			$currentEditor.type === 'script' &&
			$currentEditor.stepId === module.id
		) {
			const moduleState: FlowModuleState | undefined = $flowStateStore[module.id]

			return {
				args: moduleState?.previewArgs ?? {},
				error:
					moduleState && !moduleState.previewSuccess
						? getStringError(moduleState.previewResult)
						: undefined,
				code: module.value.content,
				lang: module.value.language,
				path: module.id,
				diffMode: $currentEditor.diffMode,
				lastDeployedCode: $currentEditor.lastDeployedCode,
				lastSavedCode: undefined,
				showDiffMode: () => {
					if (
						$currentEditor &&
						$currentEditor.type === 'script' &&
						$currentEditor.stepId === module.id
					) {
						$currentEditor.showDiffMode()
					}
				},
				applyCode: (code: string) => {
					if (
						$currentEditor &&
						$currentEditor.type === 'script' &&
						$currentEditor.stepId === module.id
					) {
						$currentEditor.editor.reviewAndApplyCode(code)
					}
				}
			}
		}

		return undefined
	}

	let scriptOptions = $derived.by(getScriptOptions)
</script>

<AIChat
	{headerLeft}
	{scriptOptions}
	flowHelpers={{
		getFlow: () => $flowStore,
		setCode: (code) => {
			if (
				$currentEditor &&
				$currentEditor.type === 'script' &&
				$currentEditor.stepId === $selectedId
			) {
				$currentEditor.editor.setCode(code)
			} else {
				console.error('No script editor found')
			}
		},
		insertStep: async (location, step) => {
			console.log('insertStep', location, step)
			const { index, modules } =
				location.type === 'start'
					? {
							index: -1,
							modules: $flowStore.value.modules
						}
					: location.type === 'start_inside_forloop'
						? {
								index: -1,
								modules: getNestedModules($flowStore, location.inside)
							}
						: location.type === 'start_inside_branch'
							? {
									index: -1,
									modules: getNestedModules($flowStore, location.inside, location.branchIndex)
								}
							: getIndexInNestedModules($flowStore, location.afterId)

			const indexToInsertAt = index + 1

			let newModules: FlowModule[] | undefined
			switch (step.type) {
				case 'rawscript': {
					newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
						modules,
						indexToInsertAt,
						'script',
						undefined,
						undefined,
						{
							language: step.language,
							kind: 'script',
							subkind: 'flow'
						}
					)
					break
				}
				case 'script': {
					newModules = await flowModuleSchemaMap?.insertNewModuleAtIndex(
						modules,
						indexToInsertAt,
						'script',
						{
							path: step.path,
							summary: '',
							hash: undefined
						}
					)
					break
				}
				case 'forloop':
				case 'branchall':
				case 'branchone': {
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

			const newModule = newModules?.[indexToInsertAt]

			if (!newModule) {
				throw new Error('Failed to insert module')
			}

			if (['branchone', 'branchall'].includes(step.type)) {
				await flowModuleSchemaMap?.addBranch(newModule)
			}

			$flowStateStore = $flowStateStore
			$flowStore = $flowStore
			return newModule.id
		},
		removeStep: async (id) => {
			console.log('removeStep', id)
			const { modules } = getIndexInNestedModules($flowStore, id)
			flowModuleSchemaMap?.selectNextId(id)
			flowModuleSchemaMap?.removeAtId(modules, id)

			if ($flowInputsStore) {
				delete $flowInputsStore[id]
			}

			$flowStore = $flowStore

			flowModuleSchemaMap?.updateFlowInputsStore()
		},
		getStepInputs: async (id) => {
			const module = dfs(id, $flowStore, false)[0]
			if (!module) {
				throw new Error('Module not found')
			}

			const inputs =
				module.value.type === 'script' || module.value.type === 'rawscript'
					? module.value.input_transforms
					: {}
			console.log('getStepInputs inputs', id, inputs)

			return inputs
		},
		setStepInputs: async (id, inputs) => {
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
				const module = dfs(id, $flowStore, false)[0]
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
				$flowStore = $flowStore
			}
		},
		getFlowInputsSchema: async () => {
			return $flowStore.schema ?? {}
		},
		setFlowInputsSchema: async (newInputs) => {
			$flowStore.schema = newInputs
		},
		selectStep: (id) => {
			$selectedId = id
		},
		getStepCode: (id) => {
			const module = dfs(id, $flowStore, false)[0]
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
				const module = dfs(id, $flowStore, false)[0]

				if (!module) {
					throw new Error('Module not found')
				}

				return getSubModules(module).flat()
			}
			return $flowStore.value.modules
		},
		setBranchPredicate: async (id, branchIndex, expression) => {
			const module = dfs(id, $flowStore, false)[0]
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
			$flowStore = $flowStore
		},
		addBranch: async (id) => {
			const module = dfs(id, $flowStore, false)[0]
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type !== 'branchall' && module.value.type !== 'branchone') {
				throw new Error('Module is not a branchall or branchone')
			}
			flowModuleSchemaMap?.addBranch(module)
			$flowStore = $flowStore
		},
		removeBranch: async (id, branchIndex) => {
			const module = dfs(id, $flowStore, false)[0]
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
			$flowStore = $flowStore
		},
		setForLoopIteratorExpression: async (id, expression) => {
			if ($currentEditor && $currentEditor.type === 'iterator' && $currentEditor.stepId === id) {
				$currentEditor.editor.setCode(expression)
			} else {
				const module = dfs(id, $flowStore, false)[0]
				if (!module) {
					throw new Error('Module not found')
				}
				if (module.value.type !== 'forloopflow') {
					throw new Error('Module is not a forloopflow')
				}
				module.value.iterator = { type: 'javascript', expr: expression }
				$flowStore = $flowStore
			}
		}
	}}
/>
