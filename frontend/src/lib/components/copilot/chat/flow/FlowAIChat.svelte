<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, untrack } from 'svelte'
	import type { ExtendedOpenFlow, FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import type { FlowModule, InputTransform, OpenFlow } from '$lib/gen'
	import type { FlowAIChatHelpers } from './core'
	import { restoreInlineScriptReferences } from './inlineScriptsUtils'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import { SPECIAL_MODULE_IDS } from '../shared'
	import type { FlowCopilotContext } from '../../flow'
	import type { ScriptLintResult } from '../shared'

	let {
		flowModuleSchemaMap,
		onTestFlow
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
		onTestFlow?: (conversationId?: string) => Promise<string | undefined>
	} = $props()

	const { flowStore, flowStateStore, selectionManager, currentEditor, previewArgs } =
		getContext<FlowEditorContext>('FlowEditorContext')
	const selectedId = $derived(selectionManager.getSelectedId())

	const { exprsToSet } = getContext<FlowCopilotContext | undefined>('FlowCopilotContext') ?? {}

	// Get diffManager from the graph
	const diffManager = $derived(flowModuleSchemaMap?.getDiffManager())

	function getModule(id: string, flow: OpenFlow = flowStore.val) {
		if (id === SPECIAL_MODULE_IDS.PREPROCESSOR) {
			return flow.value.preprocessor_module
		} else if (id === SPECIAL_MODULE_IDS.FAILURE) {
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
				selectedId: selectedId === 'settings-metadata' ? '' : selectedId
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
		setSnapshot: (snapshot: ExtendedOpenFlow) => {
			diffManager?.setBeforeFlow(snapshot)
		},
		revertToSnapshot: (snapshot?: ExtendedOpenFlow) => {
			if (!diffManager) return

			// Pass snapshot to diffManager - use message's snapshot or fall back to beforeFlow
			diffManager.revertToSnapshot(flowStore, snapshot)

			// Update current editor if needed
			const targetSnapshot = snapshot ?? diffManager.beforeFlow
			if ($currentEditor && targetSnapshot) {
				const module = getModule($currentEditor.stepId, targetSnapshot)
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
		},

		// ai chat tools
		setCode: async (id: string, code: string) => {
			const module = getModule(id)
			if (!module) {
				throw new Error('Module not found')
			}
			if (module.value.type === 'rawscript') {
				// 1. Take snapshot only if none exists (preserves baseline for cumulative changes)
				if (!diffManager?.beforeFlow) {
					const snapshot = $state.snapshot(flowStore).val
					diffManager?.setBeforeFlow(snapshot)
					diffManager?.setEditMode(true)
				}

				// 2. Apply the code change
				module.value.content = code
				const { input_transforms, schema } = await loadSchemaFromModule(module)
				module.value.input_transforms = input_transforms
				refreshStateStore(flowStore)

				// Update exprsToSet if this module is currently selected
				if (id === selectedId && exprsToSet) {
					exprsToSet.set(input_transforms)
				}

				if (flowStateStore.val[id]) {
					flowStateStore.val[id].schema = schema
				} else {
					flowStateStore.val[id] = {
						schema
					}
				}

				// 3. Manually add to moduleActions, preserving existing action types
				// Note: currentFlow is auto-synced by FlowGraphV2's effect after refreshStateStore
				const currentAction = diffManager?.moduleActions[id]
				if (!currentAction) {
					diffManager?.setModuleActions({
						...diffManager?.moduleActions,
						[id]: { action: 'modified', pending: true }
					})
				}
				// If already tracked (e.g., 'added' from setFlowJson), keep that status
			} else {
				throw new Error('Module is not a rawscript or script')
			}
			if ($currentEditor && $currentEditor.type === 'script' && $currentEditor.stepId === id) {
				$currentEditor.editor.setCode(code)
			}
		},
		getFlowInputsSchema: async () => {
			return flowStore.val.schema ?? {}
		},

		updateExprsToSet: (id: string, inputTransforms: Record<string, InputTransform>) => {
			if (id === selectedId && exprsToSet) {
				exprsToSet.set(inputTransforms)
			}
		},

		// accept/reject operations (via flowDiffManager)
		acceptAllModuleActions: () => {
			diffManager?.acceptAll(flowStore)
		},
		rejectAllModuleActions: () => {
			diffManager?.rejectAll(flowStore)
		},
		hasPendingChanges: () => {
			return diffManager?.hasPendingChanges ?? false
		},

		selectStep: (id) => {
			selectionManager.selectId(id)
		},

		testFlow: async (args, conversationId) => {
			// Set preview args if provided
			if (args) {
				previewArgs.val = args
			}
			// Call the UI test function which opens preview panel
			return await onTestFlow?.(conversationId)
		},

		getLintErrors: async (moduleId: string): Promise<ScriptLintResult> => {
			// Focus the module first
			selectionManager.selectId(moduleId)

			// Poll until editor exists
			const maxWait = 3000
			const pollInterval = 100
			let elapsed = 0

			while (elapsed < maxWait) {
				if ($currentEditor?.type === 'script') {
					// Wait 500ms for LSP to analyze the code
					await new Promise((resolve) => setTimeout(resolve, 500))
					return $currentEditor.editor.getLintErrors()
				}

				await new Promise((resolve) => setTimeout(resolve, pollInterval))
				elapsed += pollInterval
			}

			return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
		},

		setFlowJson: async (
			modules: FlowModule[] | undefined,
			schema: Record<string, any> | undefined
		) => {
			try {
				if (modules || schema) {
					// Take snapshot of current flowStore and set as beforeFlow
					if (!diffManager?.hasPendingChanges) {
						const snapshot = $state.snapshot(flowStore).val
						diffManager?.setBeforeFlow(snapshot)
						diffManager?.setEditMode(true)
					}
				}

				if (modules) {
					// Restore inline script references back to full content
					const restoredModules = restoreInlineScriptReferences(modules)
					// Directly modify flowStore (immediate effect)
					flowStore.val.value.modules = restoredModules
				}

				// Update schema if provided
				if (schema !== undefined) {
					flowStore.val.schema = schema
				}

				// Refresh the state store to update UI
				refreshStateStore(flowStore)
			} catch (error) {
				console.error('setFlowJson error:', error)
				throw new Error(
					`Failed to parse or apply JSON: ${error instanceof Error ? error.message : String(error)}`
				)
			}
		}
	}

	$effect(() => {
		if (
			$currentEditor?.type === 'script' &&
			selectedId &&
			diffManager?.moduleActions[selectedId]?.pending &&
			$currentEditor.editor.getAiChatEditorHandler()
		) {
			const moduleLastSnapshot = getModule(selectedId, diffManager.beforeFlow)
			const content =
				moduleLastSnapshot?.value.type === 'rawscript' ? moduleLastSnapshot.value.content : ''
			if (content.length > 0) {
				untrack(() =>
					$currentEditor.editor.reviewAppliedCode(content, {
						onFinishedReview: () => {
							diffManager?.acceptModule(selectedId, flowStore)
							$currentEditor.hideDiffMode()
						}
					})
				)
			}
		}
	})

	$effect(() => {
		const cleanup = aiChatManager.setFlowHelpers(flowHelpers)
		return () => {
			cleanup()
		}
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForSelectedIdChanges(
			selectedId,
			flowStore.val,
			flowStateStore.val,
			$currentEditor
		)
		return () => {
			cleanup()
		}
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForCurrentEditorChanges($currentEditor)
		return () => {
			cleanup()
		}
	})
</script>
