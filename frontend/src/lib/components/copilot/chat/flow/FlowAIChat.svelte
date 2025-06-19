<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, untrack } from 'svelte'
	import type { FlowCopilotContext } from '../../flow'
	import type { ExtendedOpenFlow, FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import { dfs as dfsApply } from '$lib/components/flows/dfs'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import type { FlowModule, OpenFlow } from '$lib/gen'
	import { getIndexInNestedModules, getNestedModules } from './utils'
	import type { AIModuleAction, FlowAIChatHelpers } from './core'
	import {
		insertNewFailureModule,
		insertNewPreprocessorModule
	} from '$lib/components/flows/flowStateUtils.svelte'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'

	let {
		flowModuleSchemaMap
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
	} = $props()

	const { flowStore, flowStateStore, selectedId, currentEditor } =
		getContext<FlowEditorContext>('FlowEditorContext')

	const { exprsToSet } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') ?? {}

	let affectedModules: Record<
		string,
		{
			action: AIModuleAction
		}
	> = $state({})
	let lastSnapshot: ExtendedOpenFlow | undefined = $state(undefined)
	let previewFlow = $derived.by(() => {
		const flow = $state.snapshot(flowStore).val
		if (Object.values(affectedModules).some((m) => m.action === 'removed')) {
			dfsApply(flow.value.modules, (m, modules) => {
				const action = affectedModules[m.id]?.action
				if (action === 'removed') {
					modules.splice(modules.indexOf(m), 1)
				}
			})
		}
		return flow
	})

	function setModuleStatus(id: string, action: AIModuleAction) {
		const existingAction: AIModuleAction | undefined = affectedModules[id]?.action

		if (existingAction === 'added' && action === 'modified') {
			// means it was added but then edited => keep the action as added
			action = 'added'
		} else if (existingAction === 'added' && action === 'removed') {
			delete affectedModules[id]
			deleteStep(id)
			return
		} else if (existingAction === 'removed' && action === 'added') {
			action = 'modified'
		}
		affectedModules[id] = {
			action
		}
	}

	function getModule(id: string, flow: OpenFlow = flowStore.val) {
		if (id === 'preprocessor') {
			return flow.value.preprocessor_module
		} else if (id === 'failure') {
			return flow.value.failure_module
		} else {
			return dfs(id, flow, false)[0]
		}
	}

	const flowHelpers: FlowAIChatHelpers = {
		// flow context
		getFlowAndSelectedId: () => {
			const flow = $state.snapshot(flowStore).val
			return {
				flow,
				selectedId: $selectedId
			}
		},
		// flow apply/reject
		getPreviewFlow: () => {
			return previewFlow
		},
		hasDiff: () => {
			return Object.keys(affectedModules).length > 0
		},
		acceptAllModuleActions: () => {
			for (const [id, affectedModule] of Object.entries(affectedModules)) {
				if (affectedModule.action === 'removed') {
					deleteStep(id)
				}
			}
			affectedModules = {}
		},
		rejectAllModuleActions() {
			for (const id of Object.keys(affectedModules)) {
				this.revertModuleAction(id)
			}
			affectedModules = {}
		},
		setLastSnapshot: (snapshot) => {
			lastSnapshot = snapshot
		},
		revertToSnapshot: (snapshot?: ExtendedOpenFlow) => {
			affectedModules = {}
			if (snapshot) {
				flowStore.val = snapshot
				refreshStateStore(flowStore)
			}
		},
		showModuleDiff(id: string) {
			if (!lastSnapshot) {
				return
			}
			const moduleLastSnapshot = id === 'Input' ? lastSnapshot.schema : getModule(id, lastSnapshot)
			const currentModule = id === 'Input' ? flowStore.val.schema : getModule(id)

			if (moduleLastSnapshot && currentModule) {
				diffDrawer?.openDrawer()
				diffDrawer?.setDiff({
					mode: 'simple',
					title: `Diff for ${id}`,
					original: moduleLastSnapshot,
					current: currentModule,
					button: {
						text: 'Accept',
						onClick: () => {
							diffDrawer?.closeDrawer()
							this.acceptModuleAction(id)
						}
					}
				})
			}
		},
		getModuleAction: (id: string) => {
			return affectedModules[id]?.action
		},
		revertModuleAction: (id: string) => {
			{
				const action = affectedModules[id]?.action
				if (action && lastSnapshot) {
					if (id === 'Input') {
						flowStore.val.schema = lastSnapshot.schema
					} else if (action === 'added') {
						deleteStep(id)
					} else if (action === 'modified') {
						const oldModule = getModule(id, lastSnapshot)
						if (!oldModule) {
							throw new Error('Module not found')
						}
						const newModule = getModule(id)
						if (!newModule) {
							throw new Error('Module not found')
						}
						newModule.value = oldModule.value
					}

					refreshStateStore(flowStore)
					delete affectedModules[id]
				}
			}
		},
		acceptModuleAction: (id: string) => {
			if (affectedModules[id]?.action === 'removed') {
				deleteStep(id)
			}
			delete affectedModules[id]
		},
		// ai chat tools
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
			setModuleStatus(id, 'modified')
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

				setModuleStatus(location.type, 'added')

				return location.type
			} else {
				const newModule = newModules?.[indexToInsertAt]

				if (!newModule) {
					throw new Error('Failed to insert module')
				}

				if (['branchone', 'branchall'].includes(step.type)) {
					await flowModuleSchemaMap?.addBranch(newModule.id)
				}

				$flowStateStore = $flowStateStore
				refreshStateStore(flowStore)

				setModuleStatus(newModule.id, 'added')

				return newModule.id
			}
		},
		removeStep: (id) => {
			setModuleStatus(id, 'removed')
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

			setModuleStatus(id, 'modified')
		},
		getFlowInputsSchema: async () => {
			return flowStore.val.schema ?? {}
		},
		setFlowInputsSchema: async (newInputs) => {
			flowStore.val.schema = newInputs
			setModuleStatus('Input', 'modified')
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

			setModuleStatus(id, 'modified')
		},
		addBranch: async (id) => {
			flowModuleSchemaMap?.addBranch(id)
			refreshStateStore(flowStore)

			setModuleStatus(id, 'modified')
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
				module.id,
				module.value.type === 'branchone' ? branchIndex + 1 : branchIndex
			)
			refreshStateStore(flowStore)

			setModuleStatus(id, 'modified')
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

			setModuleStatus(id, 'modified')
		}
	}

	function deleteStep(id: string) {
		flowModuleSchemaMap?.selectNextId(id)
		if (id === 'preprocessor') {
			flowStore.val.value.preprocessor_module = undefined
		} else if (id === 'failure') {
			flowStore.val.value.failure_module = undefined
		} else {
			const { modules } = getIndexInNestedModules(flowStore.val, id)
			flowModuleSchemaMap?.removeAtId(modules, id)
		}

		refreshStateStore(flowStore)
	}

	const allModuleIds = $derived(dfsApply(flowStore.val.value.modules, (m) => m.id))

	$effect(() => {
		// remove any affected modules that are no longer in the flow
		const untrackedAffectedModules = untrack(() => affectedModules)
		for (const id of Object.keys(untrackedAffectedModules)) {
			if (!allModuleIds.includes(id)) {
				delete affectedModules[id]
			}
		}
	})

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

	let diffDrawer: DiffDrawer | undefined = $state(undefined)
</script>

<DiffDrawer bind:this={diffDrawer} />
