<script lang="ts">
	import { getContext } from 'svelte'
	import { flowModuleMap } from '../flowModuleMap'
	import { flowStateStore } from '../flowState'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'
	import FlowModule from './FlowModule.svelte'

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')

	$: mod = $flowModuleMap[$selectedId]
</script>

{#if mod?.flowModule}
	<FlowModule
		bind:flowModule={mod.flowModule}
		on:delete={() => {
			/*
			TODO:
			$flowStateStore.modules.splice(parentIndex, 1)
			$flowStateStore = $flowStateStore
			$flowStore.value.modules.splice(parentIndex, 1)
			$flowStore = $flowStore
			*/
		}}
	/>
{/if}
