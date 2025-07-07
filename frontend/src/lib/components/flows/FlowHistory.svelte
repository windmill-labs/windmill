<script lang="ts">
	import Drawer from '../common/drawer/Drawer.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'

	import FlowHistoryInner from './FlowHistoryInner.svelte'

	interface Props {
		path: string
		onHistoryRestore?: () => void
	}

	let { path, onHistoryRestore }: Props = $props()
	let drawer: Drawer | undefined = $state()

	export function open() {
		drawer?.openDrawer()
	}
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
			onHistoryRestore={() => {
				drawer?.closeDrawer()
				onHistoryRestore?.()
			}}
			{path}
		/>
	</DrawerContent>
</Drawer>
