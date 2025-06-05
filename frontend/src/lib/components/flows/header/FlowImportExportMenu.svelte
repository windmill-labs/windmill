<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { cleanInputs } from '../utils'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	export let drawer: Drawer | undefined
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="OpenFlow" on:close={() => drawer?.toggleDrawer()}>
		{#if flowStore}
			<FlowViewer flow={cleanInputs(flowStore)} tab="raw" />
		{/if}
	</DrawerContent>
</Drawer>
