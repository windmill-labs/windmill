<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { cleanInputs } from '../utils'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

	interface Props {
		drawer: Drawer | undefined
	}

	let { drawer = $bindable() }: Props = $props()

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let flow = $derived(aiChatManager.flowAiChatHelpers?.getPreviewFlow() ?? flowStore.val)
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="OpenFlow" on:close={() => drawer?.toggleDrawer()}>
		{#if flow}
			<FlowViewer flow={cleanInputs(flow)} tab="raw" />
		{/if}
	</DrawerContent>
</Drawer>
