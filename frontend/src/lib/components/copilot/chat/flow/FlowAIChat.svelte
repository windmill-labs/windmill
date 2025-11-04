<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, untrack } from 'svelte'
	import type { ExtendedOpenFlow, FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import type { OpenFlow } from '$lib/gen'
	import { getIndexInNestedModules } from './utils'
	import type { FlowAIChatHelpers } from './core'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import YAML from 'yaml'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import { flowDiffManager } from '$lib/components/flows/flowDiffManager.svelte'

	let {
		flowModuleSchemaMap
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
	} = $props()

	const { flowStore, flowStateStore, selectedId, currentEditor } =
		getContext<FlowEditorContext>('FlowEditorContext')

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
		hasDiff: () => {
			return flowDiffManager.hasPendingChanges
		},
		acceptAllModuleActions() {
			flowDiffManager.acceptAll({
				flowStore,
				selectNextId: (id) => flowModuleSchemaMap?.selectNextId(id),
				onDelete: deleteStep,
				onScriptAccept: (moduleId) => {
					if (
						$currentEditor &&
						$currentEditor.type === 'script' &&
						$currentEditor.stepId === moduleId
					) {
						const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
						if (aiChatEditorHandler) {
							aiChatEditorHandler.keepAll({ disableReviewCallback: true })
						}
					}
				}
			})
		},
		rejectAllModuleActions() {
			flowDiffManager.rejectAll({
				flowStore,
				onScriptRevert: (moduleId, originalContent) => {
					if (
						$currentEditor &&
						$currentEditor.type === 'script' &&
						$currentEditor.stepId === moduleId
					) {
						const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
						if (aiChatEditorHandler) {
							aiChatEditorHandler.revertAll({ disableReviewCallback: true })
						}
					}
				},
				onHideDiffMode: () => {
					if ($currentEditor && $currentEditor.type === 'script') {
						$currentEditor.hideDiffMode()
					}
				}
			})
		},
		setLastSnapshot: (snapshot) => {
			flowDiffManager.setSnapshot(snapshot)
		},
		revertToSnapshot: (snapshot?: ExtendedOpenFlow) => {
			if (snapshot) {
				flowDiffManager.revertToSnapshot(flowStore)

				// Update current editor if needed
				if ($currentEditor) {
					const module = getModule($currentEditor.stepId, snapshot)
					if (module) {
						if ($currentEditor.type === 'script' && module.value.type === 'rawscript') {
							$currentEditor.editor.setCode(module.value.content)
						} else if ($currentEditor.type === 'iterator' && module.value.type === 'forloopflow') {
							$currentEditor.editor.setCode(
								module.value.iterator.type === 'javascript' ? module.value.iterator.expr : ''
							)
						}
					}
				}
			} else {
				flowDiffManager.revertToSnapshot(flowStore)
			}
		},
		revertModuleAction: (id: string) => {
			flowDiffManager.rejectModule(id, {
				flowStore,
				onScriptRevert: (moduleId, originalContent) => {
					if (
						$currentEditor &&
						$currentEditor.type === 'script' &&
						$currentEditor.stepId === moduleId
					) {
						const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
						if (aiChatEditorHandler) {
							aiChatEditorHandler.revertAll({ disableReviewCallback: true })
						}
					}
				},
				onHideDiffMode: () => {
					if ($currentEditor && $currentEditor.type === 'script') {
						$currentEditor.hideDiffMode()
					}
				}
			})
		},
		acceptModuleAction: (id: string) => {
			flowDiffManager.acceptModule(id, {
				flowStore,
				selectNextId: (id) => flowModuleSchemaMap?.selectNextId(id),
				onDelete: deleteStep,
				onScriptAccept: (moduleId) => {
					if (
						$currentEditor &&
						$currentEditor.type === 'script' &&
						$currentEditor.stepId === moduleId
					) {
						const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
						if (aiChatEditorHandler) {
							aiChatEditorHandler.keepAll({ disableReviewCallback: true })
						}
					}
				}
			})
		},
		// ai chat tools
		setCode: async (id: string, code: string) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type === 'rawscript') {
				module.value.content = code
				const { input_transforms, schema } = await loadSchemaFromModule(module)
				module.value.input_transforms = input_transforms
				refreshStateStore(flowStore)

				if (flowStateStore.val[id]) {
					flowStateStore.val[id].schema = schema
				} else {
					flowStateStore.val[id] = {
						schema
					}
				}
			} else {
				throw new Error('Module is not a rawscript or script')
			}
			if ($currentEditor && $currentEditor.type === 'script' && $currentEditor.stepId === id) {
				$currentEditor.editor.setCode(code)
			}
			// Note: No need to manually track status - timeline will compute diff automatically
		},
		getFlowInputsSchema: async () => {
			return flowStore.val.schema ?? {}
		},
		setFlowYaml: async (yaml: string) => {
			try {
				// Parse YAML to JavaScript object
				const parsed = YAML.parse(yaml)

				// Validate that it has the expected structure
				if (!parsed.modules || !Array.isArray(parsed.modules)) {
					throw new Error('YAML must contain a "modules" array')
				}

				// Update the before flow
				const snapshot = $state.snapshot(flowStore).val
				flowModuleSchemaMap?.setBeforeFlow(snapshot)

				// Update the flow structure
				flowStore.val.value.modules = parsed.modules

				if (parsed.preprocessor_module !== undefined) {
					flowStore.val.value.preprocessor_module = parsed.preprocessor_module || undefined
				}

				if (parsed.failure_module !== undefined) {
					flowStore.val.value.failure_module = parsed.failure_module || undefined
				}

				// Update schema if provided
				if (parsed.schema !== undefined) {
					flowStore.val.schema = parsed.schema
				}

				// Refresh the state store to update UI
				// The timeline derived state will automatically compute the diff
				refreshStateStore(flowStore)
			} catch (error) {
				throw new Error(
					`Failed to parse or apply YAML: ${error instanceof Error ? error.message : String(error)}`
				)
			}
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

	$effect(() => {
		const cleanup = aiChatManager.setFlowHelpers(flowHelpers)
		return cleanup
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForSelectedIdChanges(
			$selectedId,
			flowStore.val,
			flowStateStore.val,
			$currentEditor
		)
		return cleanup
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForCurrentEditorChanges($currentEditor)
		return cleanup
	})

	// Automatically show revert review when selecting a rawscript module with pending changes
	$effect(() => {
		const moduleActions = flowModuleSchemaMap?.getModuleActions() ?? {}
		const beforeFlow = flowDiffManager.beforeFlow
		if (
			$currentEditor?.type === 'script' &&
			$selectedId &&
			moduleActions?.[$selectedId]?.pending &&
			$currentEditor.editor.getAiChatEditorHandler() &&
			beforeFlow
		) {
			const moduleLastSnapshot = getModule($selectedId, beforeFlow)
			const content =
				moduleLastSnapshot?.value.type === 'rawscript' ? moduleLastSnapshot.value.content : ''
			if (content.length > 0) {
				untrack(() =>
					$currentEditor.editor.reviewAppliedCode(content, {
						onFinishedReview: () => {
							const id = $selectedId
							flowHelpers.acceptModuleAction(id)
							$currentEditor.hideDiffMode()
						}
					})
				)
			}
		}
	})
</script>
