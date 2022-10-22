<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowBranchesWrapper from './FlowBranchesWrapper.svelte'
	import FlowLoop from './FlowLoop.svelte'
	import FlowModuleComponent from './FlowModuleComponent.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	// These pointers are used to easily access previewArgs of parent module, and previous module
	// Pointer to parent module, only defined within Branches or Loops.
	export let parentModule: FlowModule | undefined = undefined
	// Pointer to previous module, for easy access to testing results
	export let previousModuleId: string | undefined = undefined
</script>

{#if flowModule.id === $selectedId}
	{#if flowModule.value.type === 'forloopflow'}
		<FlowLoop bind:mod={flowModule} {parentModule} {previousModuleId} />
	{:else if flowModule.value.type === 'branchone'}
		<FlowBranchesWrapper bind:flowModule {parentModule} {previousModuleId} />
	{:else}
		<FlowModuleComponent
			bind:flowModule
			{parentModule}
			{previousModuleId}
			on:delete={() => {
				// TODO: Restore this feature
			}}
		/>
	{/if}
{:else if flowModule.value.type === 'forloopflow'}
	{#each flowModule.value.modules as submodule, index}
		<svelte:self
			bind:flowModule={submodule}
			parentModule={flowModule}
			previousModuleId={flowModule.value.modules[index - 1]?.id}
			isParentLoop={true}
		/>
	{/each}
{:else if flowModule.value.type === 'branchone'}
	{#each flowModule.value.default as submodule, index}
		<svelte:self
			bind:flowModule={submodule}
			parentModule={flowModule}
			previousModuleId={flowModule.value.default[index - 1]?.id}
			isParentLoop={true}
		/>
	{/each}
	{#each flowModule.value.branches as branch, branchIndex}
		{#each branch.modules as submodule, index}
			<svelte:self
				bind:flowModule={submodule}
				parentModule={flowModule}
				previousModuleId={flowModule.value.branches[branchIndex].modules[index - 1]?.id}
				isParentLoop={true}
			/>
		{/each}
	{/each}
{:else if flowModule.value.type === 'branchall'}
	{#each flowModule.value.branches as branch, branchIndex}
		{#each branch.modules as submodule, index}
			<svelte:self
				bind:flowModule={submodule}
				parentModule={flowModule}
				previousModuleId={flowModule.value.branches[branchIndex].modules[index - 1]?.id}
				isParentLoop={true}
			/>
		{/each}
	{/each}
{/if}
