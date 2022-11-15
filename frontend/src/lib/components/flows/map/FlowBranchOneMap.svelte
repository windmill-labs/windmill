<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { getContext } from 'svelte'
	import type { FlowModule } from '$lib/gen'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	import { faCodeBranch, faTimesCircle, faTrashAlt } from '@fortawesome/free-solid-svg-icons'
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
			if (module.value.type === 'branchone') {
				const newModule = emptyModule()
				module.value.branches.splice(module.value.branches.length, 0, {
					summary: '',
					expr: '',
					modules: [newModule]
				})
				module = module

				$flowStateStore[newModule.id] = emptyFlowModuleState()
			}
		})
	}

	function removeBranch(index: number) {
		if (module.value.type === 'branchone') {
			module.value.branches[index].modules.forEach((mod) => {
				deleteFlowStateById(mod.id)
			})

			module.value.branches.splice(index, 1)
			module = module
		}
	}
</script>

{#if module.value.type === 'branchone'}
	<div class="flex text-xs">
		<div
			class="w-full space-y-2 flex flex-col border p-2 bg-gray-500 border-gray-600 bg-opacity-10 rounded-sm my-2 relative"
		>
			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				on:click={() => {
					selectedBranch = 0
					select(`${module.id}-branch-default`)
				}}
				class={classNames(
					'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center',
					$selectedId === `${module.id}-branch-default`
						? 'outline outline-offset-1 outline-2  outline-slate-900'
						: ''
				)}
			>
				<Icon data={faCodeBranch} class="mr-2" />
				<span
					class="truncate text-xs flex flex-row justify-between w-full flex-wrap gap-2 items-center"
				>
					Default branch
				</span>
			</div>
			<div>
				<FlowModuleSchemaMap bind:modules={module.value.default} color="indigo" />
			</div>

			{#each module.value.branches ?? [] as branch, branchIndex (branchIndex)}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					on:click={() => {
						selectedBranch = branchIndex + 1
						select(`${module.id}-branch-${branchIndex}`)
					}}
					class={classNames(
						'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
						$selectedId === `${module.id}-branch-${branchIndex}`
							? 'outline outline-offset-1 outline-2  outline-slate-900'
							: ''
					)}
				>
					<Icon data={faCodeBranch} class="mr-2" />
					<span
						class="text-xs flex flex-row justify-between w-full flex-wrap gap-2 items-center truncate"
					>
						{branch.summary || `Branch ${branchIndex}`}
						<button
							class="absolute -top-1 right-1 rounded-full m-auto  h-4 w-4 "
							on:click={() => removeBranch(branchIndex)}
							><Icon
								data={faTrashAlt}
								class="text-gray-600 hover:text-red-600"
								scale={0.6}
							/></button
						>
					</span>
				</div>

				<div>
					<FlowModuleSchemaMap bind:modules={branch.modules} color="indigo" />
				</div>
			{/each}
			<div class="overflow-clip">
				<Button
					btnClasses=""
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
