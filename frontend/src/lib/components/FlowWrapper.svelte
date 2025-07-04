<script lang="ts">
	import type { FlowBuilderProps } from './flow_builder'
	import FlowBuilder from './FlowBuilder.svelte'

	let {
		flowStore: oldFlowStore,
		...props
	}: FlowBuilderProps = $props()

	let flowStore = $state(oldFlowStore)
</script>

<FlowBuilder {flowStore} {...props} />