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
	import { emptyModule } from '../flowStateUtils'
	import { emptyFlowModuleState } from '../utils'
	import { flowStateStore } from '../flowState'

	export let module: FlowModule

	let selectedBranch = 0

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	function addBranch() {
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
	}

	function removeBranch(index: number) {
		if (module.value.type === 'branchone') {
			flowStateStore.update((fss) => {
				if (module.value.type === 'branchone') {
					module.value.branches[index].modules.forEach((mod) => {
						delete fss[mod.id]
					})
				}

				// Should also delete  custom branch state

				return fss
			})

			module.value.branches.splice(index, 1)
			module = module
		}
	}
</script>

{#if module.value.type === 'branchone'}
	<div class="flex text-xs">
		<div
			class="w-full space-y-2 flex flex-col border p-2 bg-gray-500 bg-opacity-10 rounded-sm my-2"
		>
			<Button
				size="xs"
				color="dark"
				startIcon={{ icon: faCodeBranch }}
				on:click={() => addBranch()}
			>
				Add branch
			</Button>

			<!-- svelte-ignore a11y-click-events-have-key-events -->
			<div
				on:click={() => {
					selectedBranch = 0
				}}
				class={classNames(
					'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
					false ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
				)}
			>
				<Icon data={faCodeBranch} class="mr-2" />
				<span class="text-xs flex flex-row justify-between w-full flex-wrap gap-2 items-center">
					Default branch
				</span>
			</div>
			{#if selectedBranch === 0}
				<div transition:slide>
					<FlowModuleSchemaMap bind:modules={module.value.default} color="indigo" />
				</div>
			{/if}

			{#each module.value.branches ?? [] as branch, branchIndex}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<div
					on:click={() => {
						selectedBranch = branchIndex + 1
					}}
					class={classNames(
						'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
						false ? 'outline outline-offset-1 outline-2  outline-slate-900' : ''
					)}
				>
					<Icon data={faCodeBranch} class="mr-2" />
					<span class="text-xs flex flex-row justify-between w-full flex-wrap gap-2 items-center">
						Branch {branchIndex}
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

				{#if selectedBranch === branchIndex + 1}
					<div transition:slide>
						<FlowModuleSchemaMap bind:modules={branch.modules} color="indigo" />
					</div>
				{/if}
			{/each}
		</div>
	</div>
{/if}
