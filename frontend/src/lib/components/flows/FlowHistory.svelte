<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'

	import FlowHistoryInner from './FlowHistoryInner.svelte'

	export let path: string
	let drawer: Drawer

	export function open() {
		drawer.openDrawer()
	}

	const dispatch = createEventDispatcher()
</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		on:close={() => {
			drawer?.closeDrawer()
		}}
		noPadding
		title="Deployment History"
	>
		<FlowHistoryInner
			allowFork
			on:historyRestore={() => {
				drawer.closeDrawer()
				dispatch('historyRestore')
			}}
			{path}
		/>
	</DrawerContent>
</Drawer>
