<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowBranchesWrapper from './FlowBranchesWrapper.svelte'
	import FlowLoop from './FlowLoop.svelte'
	import FlowModuleComponent from './FlowModuleComponent.svelte'
	import FlowBranchWrapper from './FlowBranchWrapper.svelte'

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
			<FlowBranchWrapper bind:branch parentModule={flowModule} {previousModuleId} />
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
		{#if $selectedId === `${parentModule?.id}-branch-${branchIndex}`}
			TODO
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
