<script lang="ts">
	import { getContext } from 'svelte'

	import type { FlowEditorContext } from '../types'
	import FlowModuleWrapper from './FlowModuleWrapper.svelte'
	import FlowSettings from './FlowSettings.svelte'
	import FlowInput from './FlowInput.svelte'
	import FlowFailureModule from './FlowFailureModule.svelte'
	import FlowLoopWrapper from './FlowLoopWrapper.svelte'
	import FlowBranchesWrapper from './FlowBranchesWrapper.svelte'
	import FlowDefaultBranchWrapper from './FlowDefaultBranchWrapper.svelte'
	import { flowStateStore } from '../flowState'

	export let initialPath: string

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	/**
	 * {#if $selectedId.includes('loop')}
			<FlowLoopWrapper />
		{:else if $selectedId.includes('branches')}
			<FlowBranchesWrapper />
		{:else if $selectedId.includes('branch')}
			<FlowDefaultBranchWrapper />
		{:else if $selectedId === 'inputs'}
			<FlowInput />
		{:else if $selectedId === 'failure'}
			<FlowFailureModule />
		{:else}
			<FlowModuleWrapper />
		{/if}
	*/

	// Previous result du premier module -> avoid 2 fois previous_result
</script>

{#key $selectedId}
	{#if $selectedId === 'settings'}
		<FlowSettings {initialPath} />
	{:else if $selectedId === 'inputs'}
		<FlowInput />
	{:else if $selectedId === 'settings-schedule'}
		<FlowSettings {initialPath} defaultTab="schedule" />
	{:else if $selectedId === 'failure'}
		<FlowFailureModule />
	{:else}
		<FlowModuleWrapper />
	{/if}
{/key}
