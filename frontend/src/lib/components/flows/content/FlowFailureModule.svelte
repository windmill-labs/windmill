<script lang="ts">
	import { getContext } from 'svelte'

	import { flowStateStore } from '../flowState'
	import type { FlowEditorContext } from '../types'

	import FlowModule from './FlowModule.svelte'

	const { previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<FlowModule
	args={previewArgs}
	indexes={'failure'}
	bind:flowModule={$flowStateStore.failureModule.flowModule}
	bind:schema={$flowStateStore.failureModule.schema}
	bind:childFlowModules={$flowStateStore.failureModule.childFlowModules}
/>
