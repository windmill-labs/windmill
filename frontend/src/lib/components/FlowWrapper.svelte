<script lang="ts">
	import AiChatLayout from './copilot/chat/AiChatLayout.svelte'
	import type { FlowBuilderProps } from './flow_builder'
	import FlowBuilder from './FlowBuilder.svelte'

	let {
		flowStore: oldFlowStore,
		disableAi,
		light,
		...props
	}: FlowBuilderProps & { light?: boolean } = $props()

	let flowStore = $state(oldFlowStore)

	$effect(() => {
		flowStore = oldFlowStore
	})
	
	let trialRender = $state(true)

	if (light) {
		setTimeout(() => {
			trialRender = false
		}, 1000 * 300)
	}
</script>

{#if trialRender}
	<AiChatLayout noPadding={true} {disableAi}>
		{#if light}<div class="bg-red-500 absolute z-10">Trial version</div>{/if}
		<FlowBuilder {flowStore} {disableAi} {...props} />
	</AiChatLayout>
{:else}
	<div class="flex flex-col items-center justify-center h-screen">
		<div class="text-2xl font-bold"
			>Windmill Whitelabel SDK is in trial mode and disabled itself after 5 minutes</div
		>
	</div>
{/if}
