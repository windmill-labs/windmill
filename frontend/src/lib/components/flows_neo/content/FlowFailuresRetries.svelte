<script lang="ts">
	import Editor from '$lib/components/Editor.svelte'

	import { flowStateStore } from '$lib/components/flows/flowState'
	import { flowStore } from '$lib/components/flows/flowStore'

	import { getContext } from 'svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import FlowModule from './FlowModule.svelte'
</script>

<div class="flex flex-col h-full ">
	<FlowModule
		bind:flowModule={$flowStateStore.failureModule.flowModule}
		bind:schema={$flowStateStore.failureModule.schema}
		indexes=""
	/>
</div>
