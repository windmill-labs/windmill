<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowLoop from './FlowLoop.svelte'
	import FlowModuleComponent from './FlowModuleComponent.svelte'

	const { selectedId, select } = getContext<FlowEditorContext>('FlowEditorContext')

	export let flowModule: FlowModule

	// These pointers are used to easily access previewArgs of parent module, and previous module
	// Pointer to parent module, only defined within Branches or Loops.
	export let parentModuleId: string | undefined = undefined
	// Pointer to previous module, for easy access to testing results
	export let previousModuleId: string | undefined = undefined
</script>

{#if flowModule.id === $selectedId}
	{#if flowModule.value.type === 'forloopflow'}
		<FlowLoop bind:mod={flowModule} {parentModuleId} {previousModuleId} />
	{:else}
		<FlowModuleComponent
			bind:flowModule
			on:delete={() => {
				// TODO: Restore this feature
			}}
		/>
	{/if}
{:else if flowModule.value.type === 'forloopflow'}
	{#each flowModule.value.modules as submodule, index}
		<svelte:self
			bind:flowModule={submodule}
			parentModuleId={flowModule.id}
			previousModuleId={flowModule.value.modules[index - 1]?.id}
		/>
	{/each}
{/if}
