<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import { getContext } from 'svelte'
	import { Button } from '../../common'
	import type { FlowEditorContext } from '../types'
	import { cleanInputs } from '../utils'
	import { FileJson } from 'lucide-svelte'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	let jsonViewerDrawer: Drawer
</script>

<Button
	btnClasses="mr-2"
	size="xs"
	variant="border"
	color="light"
	on:click={() => jsonViewerDrawer.toggleDrawer()}
	startIcon={{ icon: FileJson }}
>
	JSON
</Button>

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="OpenFlow JSON" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		{#if $flowStore}
			<FlowViewer flow={cleanInputs($flowStore)} tab="json" />
		{/if}
	</DrawerContent>
</Drawer>
