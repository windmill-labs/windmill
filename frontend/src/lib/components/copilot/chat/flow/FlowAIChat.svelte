<script lang="ts">
	import FlowModuleSchemaMap from '$lib/components/flows/map/FlowModuleSchemaMap.svelte'
	import { getContext, untrack } from 'svelte'
	import type { ExtendedOpenFlow, FlowEditorContext } from '$lib/components/flows/types'
	import { dfs } from '$lib/components/flows/previousResults'
	import { dfs as dfsApply } from '$lib/components/flows/dfs'
	import type { OpenFlow } from '$lib/gen'
	import { getIndexInNestedModules } from './utils'
	import type { AIModuleAction, FlowAIChatHelpers } from './core'
	import { loadSchemaFromModule } from '$lib/components/flows/flowInfers'
	import { aiChatManager } from '../AIChatManager.svelte'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import YAML from 'yaml'
	import { getSubModules } from '$lib/components/flows/flowExplorer'

	let {
		flowModuleSchemaMap
	}: {
		flowModuleSchemaMap: FlowModuleSchemaMap | undefined
	} = $props()

	const { flowStore, flowStateStore, selectedId, currentEditor } =
		getContext<FlowEditorContext>('FlowEditorContext')

	let affectedModules: Record<
		string,
		{
			action: AIModuleAction
		}
	> = $state({})
	let lastSnapshot: ExtendedOpenFlow | undefined = $state(undefined)

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
			return Object.keys(affectedModules).length > 0
		},
		acceptAllModuleActions() {
			for (const id of Object.keys(affectedModules)) {
				this.acceptModuleAction(id)
			}
		},
		rejectAllModuleActions() {
			// Do it in reverse to revert nested modules first then parents
			const ids = Object.keys(affectedModules)
			for (let i = ids.length - 1; i >= 0; i--) {
				this.revertModuleAction(ids[i])
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

						// Apply the old code to the editor and hide diff editor if the reverted module is a rawscript
						if (
							newModule.value.type === 'rawscript' &&
							$currentEditor?.type === 'script' &&
							$currentEditor.stepId === id
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
					delete affectedModules[id]
				}
			}
		},
		acceptModuleAction: (id: string) => {
			if (affectedModules[id]?.action === 'removed') {
				deleteStep(id)
			}

			if (
				affectedModules[id]?.action === 'modified' &&
				$currentEditor &&
				$currentEditor.type === 'script' &&
				$currentEditor.stepId === id
			) {
				const aiChatEditorHandler = $currentEditor.editor.getAiChatEditorHandler()
				if (aiChatEditorHandler) {
					aiChatEditorHandler.keepAll({ disableReviewCallback: true })
				}
			}
			delete affectedModules[id]
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
			setModuleStatus(id, 'modified')
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

				// Store IDs of modules that existed before the change
				const previousModuleIds = new Set(dfsApply(flowStore.val.value.modules, (m) => m.id))

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

				// Mark all modules as modified
				const newModuleIds = dfsApply(flowStore.val.value.modules, (m) => m.id)
				for (const id of newModuleIds) {
					// If module is new, mark as added; otherwise mark as modified
					const action = previousModuleIds.has(id) ? 'modified' : 'added'
					setModuleStatus(id, action)
				}

				// Mark modules that were removed
				for (const id of previousModuleIds) {
					if (!newModuleIds.includes(id)) {
						setModuleStatus(id, 'removed')
					}
				}

				// Mark special modules if changed
				if (parsed.preprocessor_module !== undefined) {
					setModuleStatus('preprocessor', 'modified')
				}
				if (parsed.failure_module !== undefined) {
					setModuleStatus('failure', 'modified')
				}
				if (parsed.schema !== undefined) {
					setModuleStatus('Input', 'modified')
				}

				// Refresh the state store to update UI
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
			affectedModules[$selectedId] &&
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

	let diffDrawer: DiffDrawer | undefined = $state(undefined)
</script>

<DiffDrawer bind:this={diffDrawer} />
