<script lang="ts">
	import { emptyModule, emptySchema } from '$lib/utils'

	import { getContext } from 'svelte'

	import { flowStateStore } from '../flowState'
	import { NEVER_TESTED_THIS_FAR } from '../flowStateUtils'
	import { flowStore } from '../flowStore'
	import type { FlowEditorContext } from '../types'

	import FlowModule from './FlowModule.svelte'

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

{#if $flowStore.value.failure_module}
	<FlowModule
		args={previewArgs}
		bind:flowModule={$flowStore.value.failure_module}
		bind:flowModuleState={$flowStateStore.failureModule}
		on:delete={() => {
			$flowStore.value.failure_module = undefined
		}}
	/>
{:else}
	<div class="p-4">
		<button
			on:click={() => {
				$flowStateStore.failureModule = {
					schema: emptySchema(),
					previewResult: NEVER_TESTED_THIS_FAR,
					childFlowModules: []
				}
				$flowStore.value.failure_module = emptyModule()
			}}
			class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-2 focus:outline-none "
		>
			Create failure module
		</button>
	</div>
{/if}
