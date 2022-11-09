<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	import { slide } from 'svelte/transition'
	import { faCodeBranch, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { Button } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { deleteFlowStateById, emptyModule, idMutex } from '../flowStateUtils'
	import { emptyFlowModuleState } from '../utils'
	import { flowStateStore } from '../flowState'

	export let module: FlowModule

	let selectedBranch = 0
	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	async function addBranch() {
		await idMutex.runExclusive(async () => {
			if (module.value.type === 'branchall') {
				const newModule = emptyModule()
				module.value.branches.splice(module.value.branches.length, 0, {
					summary: '',
					skip_failure: false,
					modules: [newModule]
				})
				module = module

				$flowStateStore[newModule.id] = emptyFlowModuleState()
			}
		})
	}

	function removeBranch(index: number) {
		if (module.value.type === 'branchall') {
			module.value.branches[index].modules.forEach((mod) => {
				deleteFlowStateById(mod.id)
			})

			module.value.branches.splice(index, 1)
			module = module
		}
	}
</script>

{#if module.value.type === 'branchall'}
	<div class="flex text-xs">
		<div
			class="w-full space-y-2 flex flex-col border p-2 bg-gray-500 bg-opacity-10 rounded-sm my-2"
		>
			{#each module.value.branches ?? [] as branch, branchIndex (branchIndex)}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					on:click={() => {
						selectedBranch = branchIndex
						select(`${module.id}-branch-${branchIndex}`)
					}}
					class={classNames(
						'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center',
						$selectedId === `${module.id}-branch-${branchIndex}`
							? 'outline outline-offset-1 outline-2  outline-slate-900'
							: ''
					)}
				>
					<Icon data={faCodeBranch} class="mr-2" />
					<span class="text-xs flex flex-row justify-between w-full flex-wrap gap-2 items-center">
						{branch.summary || `Branch ${branchIndex}`}
						<Button
							iconOnly
							size="xs"
							startIcon={{ icon: faTrashAlt }}
							color="light"
							variant="border"
							on:click={() => removeBranch(branchIndex)}
						/>
					</span>
				</div>

				<div>
					<FlowModuleSchemaMap bind:modules={branch.modules} color="indigo" />
				</div>
			{/each}
			<div>
				<Button
					size="xs"
					color="dark"
					startIcon={{ icon: faCodeBranch }}
					on:click={() => addBranch()}
				>
					Add branch
				</Button>
			</div>
		</div>
	</div>
{/if}
