<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import {
		createBranchAll,
		createBranches,
		createLoop,
		deleteFlowStateById,
		emptyModule
	} from '$lib/components/flows/flowStateUtils'
	import { flowStateStore } from '../flowState'
	import type { FlowModule } from '$lib/gen'
	import RemoveStepConfirmationModal from '../content/RemoveStepConfirmationModal.svelte'
	import { emptyFlowModuleState } from '../utils'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowConstantsItem from './FlowConstantsItem.svelte'

	import { dfs, flowStore } from '../flowStore'
	import { FlowGraph } from '$lib/components/graph'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'

	export let modules: FlowModule[] | undefined
	export let sidebarSize: number | undefined = undefined

	let idToRemove: string | undefined = undefined

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewModuleAtIndex(
		modules: FlowModule[],
		index: number,
		kind: 'script' | 'forloop' | 'branchone' | 'branchall' | 'flow' | 'trigger' | 'approval' | 'end'
	): Promise<FlowModule[]> {
		var module = emptyModule(kind == 'flow')
		var state = emptyFlowModuleState()
		if (kind == 'forloop') {
			;[module, state] = await createLoop(module.id)
		} else if (kind == 'branchone') {
			;[module, state] = await createBranches(module.id)
		} else if (kind == 'branchall') {
			;[module, state] = await createBranchAll(module.id)
		}
		$flowStateStore[module.id] = state
		if (kind == 'trigger') {
			module.summary = 'Trigger'
		} else if (kind == 'approval') {
			module.summary = 'Approval'
		} else if (kind == 'end') {
			module.summary = 'Terminate flow'
			module.stop_after_if = { skip_if_stopped: false, expr: 'true' }
		}
		if (!modules) return [module]
		modules.splice(index, 0, module)
		return modules
	}

	function removeAtId(modules: FlowModule[], id: string): FlowModule[] {
		const index = modules.findIndex((mod) => mod.id == id)
		if (index != -1) {
			const [removed] = modules.splice(index, 1)
			const leaves = dfs([removed], (mod) => mod.id)
			leaves.forEach((leafId: string) => deleteFlowStateById(leafId))
			return modules
		}
		return modules.map((mod) => {
			if (mod.value.type == 'forloopflow') {
				mod.value.modules = removeAtId(mod.value.modules, id)
			} else if (mod.value.type == 'branchall') {
				mod.value.branches = mod.value.branches.map((branch) => {
					branch.modules = removeAtId(branch.modules, id)
					return branch
				})
			} else if (mod.value.type == 'branchone') {
				mod.value.branches = mod.value.branches.map((branch) => {
					branch.modules = removeAtId(branch.modules, id)
					return branch
				})
				mod.value.default = removeAtId(mod.value.default, id)
			}
			return mod
		})
	}

	// async function insertModule(
	// 	modules: FlowModule[],
	// 	id: string,
	// 	kind: string,
	// 	where: 'before' | 'after'
	// ): Promise<FlowModule[]> {
	// 	console.log(modules, id, kind, where)
	// 	const index = modules.findIndex((mod) => mod.id == id)
	// 	if (index != -1) {
	// 		console.log('INSERTING', modules, index)
	// 		return await insertNewModuleAtIndex(modules, index + (where == 'after' ? 1 : 0), kind as any)
	// 	}
	// 	return await Promise.all(
	// 		modules.map(async (mod) => {
	// 			if (mod.value.type == 'forloopflow') {
	// 				mod.value.modules = await insertModule(mod.value.modules, id, kind, where)
	// 			} else if (mod.value.type == 'branchall') {
	// 				mod.value.branches = await Promise.all(
	// 					mod.value.branches.map(async (branch) => {
	// 						branch.modules = await insertModule(branch.modules, id, kind, where)
	// 						return branch
	// 					})
	// 				)
	// 			} else if (mod.value.type == 'branchone') {
	// 				mod.value.branches = await Promismod.value.branches.map((branch) => {
	// 					branch.modules = await insertModule(branch.modules, id, kind, where)
	// 					return branch
	// 				})
	// 				mod.value.default = await insertModule(mod.value.default, id, kind, where)
	// 			}
	// 			return mod
	// 		})
	// 	)
	// }

	$: confirmationModalOpen = idToRemove !== undefined

	$: sidebarMode == 'graph' ? (sidebarSize = 40) : (sidebarSize = 20)

	let sidebarMode: 'list' | 'graph' = 'graph'

	let minHeight = 0

	function selectNextId(id: any) {
		if (modules) {
			let allIds = dfs(modules, (mod) => mod.id)
			if (allIds.length > 1) {
				const idx = allIds.indexOf(id)
				$selectedId = idx == 0 ? allIds[0] : allIds[idx - 1]
			} else {
				$selectedId = 'settings-metadata'
			}
		}
	}
	async function addBranch(module: FlowModule) {
		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			module.value.branches.splice(module.value.branches.length, 0, {
				summary: '',
				expr: 'false',
				modules: []
			})
		}
	}

	function removeBranch(module: FlowModule, index: number) {
		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			if (module.value.branches[index]?.modules) {
				const leaves = dfs(module.value.branches[index].modules, (mod) => mod.id)
				leaves.forEach((leafId: string) => deleteFlowStateById(leafId))
			}

			module.value.branches.splice(index - 1, 1)
			module = { ...module }
		}
	}
</script>

<div class="flex flex-col h-full relative">
	<div
		class="z-10 sticky inline-flex flex-col gap-2 top-0 bg-gray-50 flex-initial p-2 items-center border-b border-gray-300"
	>
		<FlowSettingsItem />
		<FlowConstantsItem />
	</div>

	<div class="flex-auto grow" bind:clientHeight={minHeight}>
		<FlowGraph
			insertable
			{minHeight}
			rebuildOnChange={$flowStore}
			maxHeight={minHeight}
			modules={$flowStore.value?.modules}
			{selectedId}
			on:delete={({ detail }) => {
				let e = detail.detail
				if (e.event.shiftKey || e.type == 'identity') {
					selectNextId(e.id)
					removeAtId($flowStore.value.modules, e.id)
					$flowStore = $flowStore
				} else {
					idToRemove = e.id
				}
			}}
			on:insert={async ({ detail }) => {
				if (detail.modules) {
					await insertNewModuleAtIndex(detail.modules, detail.index ?? 0, detail.detail)
					$flowStore = $flowStore
					$selectedId = detail.modules[detail.index ?? 0].id
				}
			}}
			on:newBranch={async ({ detail }) => {
				if (detail.module) {
					await addBranch(detail.module)
					$flowStore = $flowStore
				}
			}}
			on:deleteBranch={async ({ detail }) => {
				if (detail.module) {
					await removeBranch(detail.module, detail.index)
					$flowStore = $flowStore
					$selectedId = detail.module.id
				}
			}}
		/>
	</div>
	<div
		class="z-10 absolute w-full inline-flex flex-col gap-2 bottom-0 left-0  flex-initial p-2 items-center border-b"
	>
		<FlowErrorHandlerItem />
	</div>
</div>

<RemoveStepConfirmationModal
	bind:open={confirmationModalOpen}
	on:canceled={() => {
		idToRemove = undefined
	}}
	on:confirmed={() => {
		if (idToRemove !== undefined) {
			selectNextId(idToRemove)
			removeAtId($flowStore.value.modules, idToRemove)
			$flowStore = $flowStore
			idToRemove = undefined
		}
	}}
/>
