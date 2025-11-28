<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, untrack } from 'svelte'
	import type { ExtendedOpenFlow, FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import type { OpenFlow } from '$lib/gen'
	import type { FlowAIChatHelpers } from './core'
	import { restoreInlineScriptReferences } from './core'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import { getSubModules } from '$lib/components/flows/flowExplorer'

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

	// Get diffManager from the graph
	const diffManager = $derived(flowModuleSchemaMap?.getDiffManager())

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
				selectedId: selectedId
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

		// Snapshot management - AI sets this when making changes
		setLastSnapshot: (snapshot) => {
			diffManager?.setSnapshot(snapshot)
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
					diffManager?.setSnapshot(snapshot)
					diffManager?.setEditMode(true)
				}

				// 2. Apply the code change
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

				// 3. Update afterFlow (needed for diff viewer)
				diffManager?.setAfterFlow({
					modules: flowStore.val.value.modules,
					preprocessor_module: flowStore.val.value.preprocessor_module,
					failure_module: flowStore.val.value.failure_module
				})

				// 4. Manually add to moduleActions, preserving existing action types
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

		setFlowJson: async (json: string) => {
			try {
				// Parse JSON to JavaScript object
				console.log('HERE JSON', json)
				const parsed = JSON.parse(json)

				// Validate that it has the expected structure
				if (!parsed.modules || !Array.isArray(parsed.modules)) {
					throw new Error('JSON must contain a "modules" array')
				}

				// Restore inline script references back to full content
				const restoredModules = restoreInlineScriptReferences(parsed.modules)

				// Also restore preprocessor and failure modules if they have references
				let restoredPreprocessor = parsed.preprocessor_module
				if (
					restoredPreprocessor?.value?.type === 'rawscript' &&
					restoredPreprocessor.value.content
				) {
					const match = restoredPreprocessor.value.content.match(/^inline_script\.(.+)$/)
					if (match) {
						// Wrap in array to reuse the restoration function
						const restored = restoreInlineScriptReferences([restoredPreprocessor])
						restoredPreprocessor = restored[0]
					}
				}

				let restoredFailure = parsed.failure_module
				if (restoredFailure?.value?.type === 'rawscript' && restoredFailure.value.content) {
					const match = restoredFailure.value.content.match(/^inline_script\.(.+)$/)
					if (match) {
						const restored = restoreInlineScriptReferences([restoredFailure])
						restoredFailure = restored[0]
					}
				}

				// Take snapshot of current flowStore BEFORE making changes
				if (!diffManager?.hasPendingChanges) {
					const snapshot = $state.snapshot(flowStore).val
					diffManager?.setSnapshot(snapshot)
				}

				// Directly modify flowStore (immediate effect)
				flowStore.val.value.modules = restoredModules

				if (parsed.preprocessor_module !== undefined) {
					flowStore.val.value.preprocessor_module = restoredPreprocessor || undefined
				}

				if (parsed.failure_module !== undefined) {
					flowStore.val.value.failure_module = restoredFailure || undefined
				}

				// Update schema if provided
				if (parsed.schema !== undefined) {
					flowStore.val.schema = parsed.schema
					diffManager?.setAfterInputSchema(parsed.schema)
				}

				diffManager?.setEditMode(true)
				diffManager?.setAfterFlow({
					modules: restoredModules,
					preprocessor_module: restoredPreprocessor || undefined,
					failure_module: restoredFailure || undefined
				})
				console.log('HERE: [setFlowJson] afterFlow', diffManager?.afterFlow)

				// Refresh the state store to update UI
				// The $effect in FlowGraphV2 will detect changes and update afterFlow for diff computation
				refreshStateStore(flowStore)
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
		return cleanup
	})

	$effect(() => {
		const cleanup = aiChatManager.listenForSelectedIdChanges(
			selectedId,
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
</script>
