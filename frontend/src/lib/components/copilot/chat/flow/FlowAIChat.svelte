<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, untrack } from 'svelte'
	import type { ExtendedOpenFlow, FlowEditorContext } from '$lib/components/flows/types'
	import type { InputTransform } from '$lib/gen'
	import type { FlowAIChatHelpers } from './core'
	import { createInlineScriptSession } from './inlineScriptsUtils'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import { getSubModules } from '$lib/components/flows/flowExplorer'
	import type { FlowCopilotContext } from '../../flow'
	import type { ScriptLintResult } from '../shared'
	import {
		applyFlowJsonUpdate,
		getFlowModuleById,
		updateRawScriptModuleContent
	} from './helperUtils'

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
	const inlineScriptSession = createInlineScriptSession()

	// Get diffManager from the graph
	const diffManager = $derived(flowModuleSchemaMap?.getDiffManager())
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
				const module = getFlowModuleById(flowStore.val, id)

				if (!module) {
					throw new Error('Module not found')
				}

				return getSubModules(module).flat()
			}
			return flowStore.val.value.modules
		},
		inlineScriptSession,
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
				const module = getFlowModuleById(targetSnapshot, $currentEditor.stepId)
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
			// 1. Take snapshot only if none exists (preserves baseline for cumulative changes)
			if (!diffManager?.beforeFlow) {
				const snapshot = $state.snapshot(flowStore).val
				diffManager?.setBeforeFlow(snapshot)
				diffManager?.setEditMode(true)
			}

			// 2. Apply the code change
			const module = updateRawScriptModuleContent(flowStore.val, id, code)
			if (!module) {
				throw new Error('Module not found or is not a rawscript')
			}

			inlineScriptSession.set(id, code)
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
			const module = getFlowModuleById(flowStore.val, moduleId)
			if (!module || module.value.type !== 'rawscript') {
				return { errorCount: 0, warningCount: 0, errors: [], warnings: [] }
			}

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

		setFlowJson: async ({ modules, schema, preprocessorModule, failureModule }) => {
			try {
				if (
					modules !== undefined ||
					schema !== undefined ||
					preprocessorModule !== undefined ||
					failureModule !== undefined
				) {
					// Take snapshot of current flowStore and set as beforeFlow
					if (!diffManager?.hasPendingChanges) {
						const snapshot = $state.snapshot(flowStore).val
						diffManager?.setBeforeFlow(snapshot)
						diffManager?.setEditMode(true)
					}
				}

				const result = applyFlowJsonUpdate(flowStore.val, inlineScriptSession, {
					modules,
					schema,
					preprocessorModule,
					failureModule
				})

				// Refresh the state store to update UI
				refreshStateStore(flowStore)
				return result
			} catch (error) {
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
			const moduleLastSnapshot = getFlowModuleById(diffManager.beforeFlow, selectedId)
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
