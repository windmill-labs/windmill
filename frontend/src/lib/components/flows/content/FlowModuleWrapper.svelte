<script lang="ts">
	import { Script, type FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowBranchesWrapper from './FlowBranchesWrapper.svelte'
	import FlowLoop from './FlowLoop.svelte'
	import FlowModuleComponent from './FlowModuleComponent.svelte'
	import FlowBranchAllWrapper from './FlowBranchAllWrapper.svelte'
	import FlowBranchOneWrapper from './FlowBranchOneWrapper.svelte'
	import {
		createInlineScriptModule,
		createLoop,
		createBranches,
		pickScript,
		createBranchAll
	} from '$lib/components/flows/flowStateUtils'
	import FlowInputs from './FlowInputs.svelte'
	import { flowStateStore, type FlowModuleState } from '../flowState'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	// These pointers are used to easily access previewArgs of parent module, and previous module
	// Pointer to parent module, only defined within Branches or Loops.
	export let parentModule: FlowModule | undefined = undefined
	// Pointer to previous module, for easy access to testing results
	export let previousModuleId: string | undefined = undefined

	function updateStores(module: FlowModule, flowModuleState: FlowModuleState) {
		if (JSON.stringify(flowModule) != JSON.stringify(module)) {
			flowModule = module
			$flowStateStore[module.id] = flowModuleState
		}
	}
</script>

{#if flowModule.id === $selectedId}
	{#if flowModule.value.type === 'forloopflow'}
		<FlowLoop bind:mod={flowModule} {parentModule} {previousModuleId} />
	{:else if flowModule.value.type === 'branchone'}
		<FlowBranchesWrapper bind:flowModule {parentModule} {previousModuleId} />
	{:else if flowModule.value.type === 'branchall'}
		<FlowBranchesWrapper bind:flowModule {parentModule} {previousModuleId} />
	{:else if flowModule.value.type === 'identity'}
		<FlowInputs
			shouldDisableTriggerScripts={parentModule !== undefined || previousModuleId !== undefined}
			on:loop={async () => {
				const [module, state] = await createLoop(flowModule.id)
				updateStores(module, state)
			}}
			on:branchone={async () => {
				const [module, state] = await createBranches(flowModule.id)
				updateStores(module, state)
			}}
			on:branchall={async () => {
				const [module, state] = await createBranchAll(flowModule.id)
				updateStores(module, state)
			}}
			on:pick={async ({ detail }) => {
				const { path, summary, kind } = detail
				const [module, state] = await pickScript(path, summary, flowModule.id)
				updateStores(module, state)
			}}
			on:new={async ({ detail }) => {
				const { language, kind, subkind } = detail

				const [module, state] = await createInlineScriptModule(
					language,
					kind,
					subkind,
					flowModule.id
				)

				if (kind == Script.kind.APPROVAL) {
					module.suspend = { required_events: 1, timeout: 1800 }
				}

				updateStores(module, state)
			}}
			failureModule={/*TODO : FIX*/ false}
		/>
	{:else}
		<FlowModuleComponent bind:flowModule {parentModule} {previousModuleId} />
	{/if}
{:else if flowModule.value.type === 'forloopflow'}
	{#each flowModule.value.modules as submodule, index}
		<svelte:self
			bind:flowModule={submodule}
			bind:parentModule={flowModule}
			previousModuleId={flowModule.value.modules[index - 1]?.id}
		/>
	{/each}
{:else if flowModule.value.type === 'branchone'}
	{#if $selectedId === `${flowModule?.id}-branch-default`}
		<div class="p-4 text-sm">Default branch</div>
	{:else}
		{#each flowModule.value.default as submodule, index}
			<svelte:self
				bind:flowModule={submodule}
				bind:parentModule={flowModule}
				previousModuleId={flowModule.value.default[index - 1]?.id}
			/>
		{/each}
	{/if}
	{#each flowModule.value.branches as branch, branchIndex}
		{#if $selectedId === `${flowModule?.id}-branch-${branchIndex}`}
			<FlowBranchOneWrapper bind:branch parentModule={flowModule} {previousModuleId} />
		{:else}
			{#each branch.modules as submodule, index}
				<svelte:self
					bind:flowModule={submodule}
					bind:parentModule={flowModule}
					previousModuleId={flowModule.value.branches[branchIndex].modules[index - 1]?.id}
				/>
			{/each}
		{/if}
	{/each}
{:else if flowModule.value.type === 'branchall'}
	{#each flowModule.value.branches as branch, branchIndex}
		{#if $selectedId === `${flowModule?.id}-branch-${branchIndex}`}
			<FlowBranchAllWrapper bind:branch />
		{:else}
			{#each branch.modules as submodule, index}
				<svelte:self
					bind:flowModule={submodule}
					bind:parentModule={flowModule}
					previousModuleId={flowModule.value.branches[branchIndex].modules[index - 1]?.id}
				/>
			{/each}
		{/if}
	{/each}
{/if}
