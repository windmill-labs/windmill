<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import type { FlowModule } from '$lib/gen'
	import { classNames, emptyModule, emptySchema } from '$lib/utils'
	import { faCodeBranch, faPlus } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { FlowModuleState } from '../flowState'
	import { emptyFlowModuleState, NEVER_TESTED_THIS_FAR } from '../flowStateUtils'
	import type { FlowEditorContext } from '../types'
	import FlowModuleSchemaMap from './FlowModuleSchemaMap.svelte'
	export let index: number

	export let moduleStates: FlowModuleState[] | undefined
	export let module: FlowModule

	const { select, selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	function addBranch() {
		if (moduleStates) {
			moduleStates = [
				...moduleStates,
				{
					schema: emptySchema(),
					childFlowModules: [emptyFlowModuleState()],
					previewResult: NEVER_TESTED_THIS_FAR
				}
			]
			moduleStates = moduleStates
		}
		if (module.value.type === 'branches') {
			module.value.branches = [
				...module.value.branches,
				{
					summary: '',
					expr: '',
					modules: [emptyModule()]
				}
			]
			module = module
		}
	}

	$: console.log({ moduleStates, module })
</script>

{#if moduleStates && module.value.type === 'branches'}
	<Button
		btnClasses="my-2"
		size="xs"
		color="dark"
		startIcon={{ icon: faPlus }}
		on:click={addBranch}
	>
		Add branch
	</Button>
	{#each moduleStates as moduleState, moduleStateIndex}
		{#if moduleState.childFlowModules && moduleStateIndex === 0}
			<span>Start default branch</span>

			<FlowModuleSchemaMap
				prefix={String(index)}
				suffix={String(moduleStateIndex)}
				bind:moduleStates={moduleState.childFlowModules}
				bind:modules={module.value.default.modules}
			/>
		{:else if moduleState.childFlowModules && moduleStateIndex > 0}
			<div
				on:click={() => select('settings')}
				class={classNames(
					'border w-full rounded-md p-2 bg-white text-sm cursor-pointer flex items-center mb-4',
					$selectedId.includes('settings')
						? 'outline outline-offset-1 outline-2  outline-slate-900'
						: ''
				)}
			>
				<Icon data={faCodeBranch} class="mr-2" />
				<span class="font-bold flex flex-row justify-between w-full flex-wrap gap-2 items-center">
					Branche {moduleStateIndex}
				</span>
			</div>
			<FlowModuleSchemaMap
				prefix={String(index)}
				suffix={String(moduleStateIndex)}
				bind:moduleStates={moduleState.childFlowModules}
				bind:modules={module.value.branches[moduleStateIndex].modules}
			/>
		{/if}
	{/each}
{/if}
