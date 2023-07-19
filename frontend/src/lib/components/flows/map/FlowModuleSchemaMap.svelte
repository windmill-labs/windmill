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
	import type { FlowModule } from '$lib/gen'
	import { emptyFlowModuleState } from '../utils'
	import FlowSettingsItem from './FlowSettingsItem.svelte'
	import FlowConstantsItem from './FlowConstantsItem.svelte'

	import { dfs } from '../flowStore'
	import { FlowGraph } from '$lib/components/graph'
	import FlowErrorHandlerItem from './FlowErrorHandlerItem.svelte'
	import { push } from '$lib/history'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Portal from 'svelte-portal'
	import { getDependentComponents } from '../flowExplorer'

	export let modules: FlowModule[] | undefined
	export let sidebarSize: number | undefined = undefined

	const { selectedId, moving, history, flowStateStore, flowStore } =
		getContext<FlowEditorContext>('FlowEditorContext')

	async function insertNewModuleAtIndex(
		modules: FlowModule[],
		index: number,
		kind: 'script' | 'forloop' | 'branchone' | 'branchall' | 'flow' | 'trigger' | 'approval' | 'end'
	): Promise<FlowModule[]> {
		push(history, $flowStore)
		var module = emptyModule($flowStateStore, kind == 'flow')
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
			leaves.forEach((leafId: string) => deleteFlowStateById(leafId, flowStateStore))
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
		push(history, $flowStore)

		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			module.value.branches.splice(module.value.branches.length, 0, {
				summary: '',
				expr: 'false',
				modules: []
			})
		}
	}

	function removeBranch(module: FlowModule, index: number) {
		push(history, $flowStore)

		if (module.value.type === 'branchone' || module.value.type === 'branchall') {
			const offset = module.value.type === 'branchone' ? 1 : 0

			if (module.value.branches[index - offset]?.modules) {
				const leaves = dfs(module.value.branches[index - offset].modules, (mod) => mod.id)
				leaves.forEach((leafId: string) => deleteFlowStateById(leafId, flowStateStore))
			}

			module.value.branches.splice(index - offset, 1)
		}
	}

	let deleteCallback: (() => void) | undefined = undefined
	let dependents: Record<string, string[]> = {}
</script>

<Portal>
	<ConfirmationModal
		title="Confirm deleting step with dependents"
		confirmationText="Delete step"
		open={Boolean(deleteCallback)}
		on:confirmed={() => {
			if (deleteCallback) {
				deleteCallback()
				deleteCallback = undefined
			}
		}}
		on:canceled={() => {
			deleteCallback = undefined
		}}
	>
		<div class="text-gray-800 pb-2"
			>Found the following steps that will require changes after this step is deleted:</div
		>
		{#each Object.entries(dependents) as [k, v]}
			<div class="pb-3">
				<h3 class="text-gray-700 font-semibold">{k}</h3>
				<ul class="text-sm">
					{#each v as dep}
						<li>{dep}</li>
					{/each}
				</ul>
			</div>
		{/each}
	</ConfirmationModal>
</Portal>
<div class="flex flex-col h-full relative -pt-1">
	<div
		class="z-10 sticky inline-flex flex-col gap-2 top-0 bg-surface-secondary flex-initial p-2 items-center border-b border-gray-300"
	>
		<FlowSettingsItem />
		<FlowConstantsItem />
	</div>

	<div class="flex-auto grow" bind:clientHeight={minHeight}>
		<FlowGraph
			insertable
			scroll
			{minHeight}
			moving={$moving?.module.id}
			rebuildOnChange={$flowStore}
			maxHeight={minHeight}
			modules={$flowStore.value?.modules}
			{selectedId}
			on:delete={({ detail }) => {
				let e = detail.detail
				dependents = getDependentComponents(e.id, $flowStore)
				const cb = () => {
					push(history, $flowStore)
					selectNextId(e.id)
					removeAtId($flowStore.value.modules, e.id)
					$flowStore = $flowStore
				}

				if (Object.keys(dependents).length > 0) {
					deleteCallback = cb
				} else {
					cb()
				}
			}}
			on:insert={async ({ detail }) => {
				if (detail.modules) {
					if ($moving) {
						push(history, $flowStore)
						let indexToRemove = $moving.modules.findIndex((m) => $moving?.module?.id == m.id)
						$moving.modules.splice(indexToRemove, 1)
						detail.modules.splice(detail.index, 0, $moving.module)
						$selectedId = $moving.module.id
						$moving = undefined
					} else {
						await insertNewModuleAtIndex(detail.modules, detail.index ?? 0, detail.detail)
						$selectedId = detail.modules[detail.index ?? 0].id
					}
					$flowStore = $flowStore
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
			on:move={async ({ detail }) => {
				if (!$moving || $moving.module.id !== detail.module.id) {
					if (detail.module && detail.modules) {
						console.log('MOVE+')
						$moving = { module: detail.module, modules: detail.modules }
					}
				} else {
					$moving = undefined
				}
			}}
		/>
	</div>
	<div
		class="z-10 absolute w-full inline-flex flex-col gap-2 bottom-0 left-0 flex-initial p-2 items-center border-b"
	>
		<FlowErrorHandlerItem />
	</div>
</div>
