<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import GroupList from './GroupList.svelte'
	import type { AppComponent } from '../component'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '../../types'

	export let component: AppComponent

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	$: subgrids = Object.entries($app.subgrids ?? {}).filter(([id]) => id.startsWith(component.id))

	let codeDrawer: Drawer

	export async function openDrawer() {
		codeDrawer.openDrawer()
	}
</script>

<Drawer bind:this={codeDrawer}>
	<DrawerContent title="Group manager" on:close={codeDrawer.closeDrawer}>
		<GroupList
			selectedGroup={{
				component,
				subgrids
			}}
		/>
	</DrawerContent>
</Drawer>
