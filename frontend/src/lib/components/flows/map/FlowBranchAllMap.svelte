<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	import { slide } from 'svelte/transition'
	import { faCodeBranch, faTimesCircle, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { Button } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import Icon from 'svelte-awesome'
	import { deleteFlowStateById, emptyModule, idMutex } from '../flowStateUtils'

	let selectedBranch = 0

	export let module: FlowModule

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	async function addBranch() {
		await idMutex.runExclusive(async () => {
			if (module.value.type === 'branchall') {
				const newModule = emptyModule()
				module.value.branches.splice(module.value.branches.length, 0, {
					summary: '',
					skip_failure: false,
					modules: []
				})
				module = module
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
	<div class="flex text-xs px-2">
		<div
			class="w-full space-y-2 pb-2 flex flex-col border bg-gray-500 border-gray-600 bg-opacity-10 rounded-sm my-2 relative"
		>
			{#each module.value.branches ?? [] as branch, branchIndex (branchIndex)}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div transition:slide|local>
					<div
						on:click={() => {
							selectedBranch = branchIndex
							select(`${module.id}-branch-${branchIndex}`)
						}}
						class={classNames(
							`border-b ${
								branchIndex > 0 ? 'border-t' : ''
							} w-full p-2 bg-white border-gray-500 text-sm cursor-pointer flex items-center relative module`,
							$selectedId === `${module.id}-branch-${branchIndex}`
								? 'outline outline-2  outline-slate-900'
								: ''
						)}
					>
						<Icon data={faCodeBranch} scale={1.1} class="mr-2" />
						<span
							class="text-xs flex flex-row justify-between w-full flex-wrap gap-2 items-center truncate"
						>
							{branch.summary || `Branch ${branchIndex}`}
							<button
								class="absolute -top-2 -right-1 rounded-full h-4 w-4 bg-white center-center {$selectedId ===
								`${module.id}-branch-${branchIndex}`
									? ''
									: '!hidden'} trash"
								on:click={() => removeBranch(branchIndex)}
								><Icon
									data={faTimesCircle}
									class="text-gray-600 hover:text-red-600"
									scale={0.9}
								/></button
							>
						</span>
					</div>

					<div>
						<FlowModuleSchemaMap parentType="branchall" bind:modules={branch.modules} />
					</div>
				</div>
			{/each}
			<div class="overflow-clip ml-2 mt-2">
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

<style>
	.module:hover .trash {
		display: flex !important;
	}
</style>
