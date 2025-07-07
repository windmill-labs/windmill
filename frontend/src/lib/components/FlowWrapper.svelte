<script lang="ts">
	import AiChatLayout from './copilot/chat/AiChatLayout.svelte'
	import type { FlowBuilderProps } from './flow_builder'
	import FlowBuilder from './FlowBuilder.svelte'

	let { flowStore: oldFlowStore, disableAi, ...props }: FlowBuilderProps = $props()

	let flowStore = $state(oldFlowStore)
</script>

<AiChatLayout noPadding={true} {disableAi}>
	<FlowBuilder {flowStore} {disableAi} {...props} />
</AiChatLayout>
