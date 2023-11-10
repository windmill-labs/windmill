<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import GroupList from './GroupList.svelte'
	import type { GridItem } from '../../types'

	export let item: GridItem

	let codeDrawer: Drawer

	export async function openDrawer() {
		codeDrawer.openDrawer()
	}
</script>

<Drawer bind:this={codeDrawer}>
	<DrawerContent title="Component Groups" on:close={codeDrawer.closeDrawer}>
		<GroupList {item} />
	</DrawerContent>
</Drawer>
