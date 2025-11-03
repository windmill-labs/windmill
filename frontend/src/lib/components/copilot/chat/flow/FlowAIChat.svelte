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
	import { buildFlowTimeline } from '$lib/components/flows/flowDiff'

	let {
		flowModuleSchemaMap
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
	} = $props()

	const { flowStore, flowStateStore, selectedId, currentEditor } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let lastSnapshot: ExtendedOpenFlow | undefined = $state(undefined)

	// Compute diff timeline using buildFlowTimeline
	let timeline = $derived.by(() => {
		if (!lastSnapshot) return undefined

		return buildFlowTimeline(lastSnapshot.value, flowStore.val.value, {
			markRemovedAsShadowed: false
		})
	})

	// Derive module actions from timeline
	let moduleActions = $derived(timeline?.afterActions)

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
			return timeline !== undefined && Object.keys(moduleActions ?? {}).length > 0
		},
		acceptAllModuleActions() {
			for (const id of Object.keys(moduleActions ?? {})) {
				this.acceptModuleAction(id)
			}
		},
		rejectAllModuleActions() {
			// Do it in reverse to revert nested modules first then parents
			const ids = Object.keys(moduleActions ?? {})
			for (let i = ids.length - 1; i >= 0; i--) {
				this.revertModuleAction(ids[i])
			}
			lastSnapshot = undefined
		},
		setLastSnapshot: (snapshot) => {
			lastSnapshot = snapshot
		},
		revertToSnapshot: (snapshot?: ExtendedOpenFlow) => {
			lastSnapshot = undefined
			if (snapshot) {
				flowStore.val = snapshot
				refreshStateStore(flowStore)

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
			}
		},
		revertModuleAction: (id: string) => {
			{
				// Handle __ prefixed IDs for type-changed modules
				const actualId = id.startsWith('__') ? id.substring(2) : id
				const action = moduleActions?.[id]

				if (action && lastSnapshot) {
					if (id === 'Input') {
						flowStore.val.schema = lastSnapshot.schema
					} else if (action === 'added') {
						deleteStep(actualId)
					} else if (action === 'removed') {
						// For removed modules, restore from lastSnapshot
						const oldModule = getModule(actualId, lastSnapshot)
						if (oldModule) {
							// Re-insert the module at its original position
							// This is complex, so for now we'll revert the whole flow and re-apply other changes
							// TODO: Implement proper re-insertion logic
							console.warn('Reverting removed module - full flow revert needed')
						}
					} else if (action === 'modified') {
						const oldModule = getModule(actualId, lastSnapshot)
						if (!oldModule) {
							throw new Error('Module not found')
						}
						const newModule = getModule(actualId)
						if (!newModule) {
							throw new Error('Module not found')
						}

						// Apply the old code to the editor and hide diff editor if the reverted module is a rawscript
						if (
							newModule.value.type === 'rawscript' &&
							$currentEditor?.type === 'script' &&
							$currentEditor.stepId === actualId
						) {
							const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
							if (aiChatEditorHandler) {
								aiChatEditorHandler.revertAll({ disableReviewCallback: true })
								$currentEditor.hideDiffMode()
							}
						}

						Object.keys(newModule).forEach((k) => delete newModule[k])
						Object.assign(newModule, $state.snapshot(oldModule))
					}

					refreshStateStore(flowStore)
				}
			}
		},
		acceptModuleAction: (id: string) => {
			// Handle __ prefixed IDs for type-changed modules
			const actualId = id.startsWith('__') ? id.substring(2) : id
			const action = moduleActions?.[id]

			if (action === 'removed') {
				deleteStep(actualId)
			}

			if (
				action === 'modified' &&
				$currentEditor &&
				$currentEditor.type === 'script' &&
				$currentEditor.stepId === actualId
			) {
				const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
				if (aiChatEditorHandler) {
					aiChatEditorHandler.keepAll({ disableReviewCallback: true })
				}
			}

			// Note: We don't delete from moduleActions since it's derived from timeline
			// Accepting all changes means clearing the lastSnapshot
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

				// Create snapshot for diff/revert functionality if it doesn't exist
				if (!lastSnapshot) {
					lastSnapshot = $state.snapshot(flowStore).val
				}

				// Update the before flow
				flowModuleSchemaMap?.setBeforeFlow(lastSnapshot)

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
		if (
			$currentEditor?.type === 'script' &&
			$selectedId &&
			moduleActions?.[$selectedId] &&
			$currentEditor.editor.getAiChatEditorHandler()
		) {
			const moduleLastSnapshot = getModule($selectedId, lastSnapshot)
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
