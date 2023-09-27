<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import GroupList from './GroupList.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext, GridItem } from '../../types'
	import { getAllSubgridsAndComponentIds } from '../appUtils'

	export let item: GridItem

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	$: subgrids = getSubgrids(item)

	function getSubgrids(item: GridItem) {
		let allSubgrids = {}
		let subgrids = getAllSubgridsAndComponentIds($app, item.data)[0]
		for (let key of subgrids) {
			allSubgrids[key] = $app.subgrids?.[key]
		}
		return allSubgrids
	}

	let codeDrawer: Drawer

	export async function openDrawer() {
		codeDrawer.openDrawer()
	}
</script>

<Drawer bind:this={codeDrawer}>
	<DrawerContent title="Group manager" on:close={codeDrawer.closeDrawer}>
		<GroupList
			selectedGroup={{
				item,
				subgrids
			}}
		/>
	</DrawerContent>
</Drawer>
