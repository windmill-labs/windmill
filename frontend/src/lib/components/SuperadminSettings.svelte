<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import SuperadminSettingsInner from './SuperadminSettingsInner.svelte'

	let drawer: Drawer | undefined = $state()

	export function openDrawer() {
		drawer?.openDrawer()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
	}
</script>

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent overflow_y={true} title="Instance settings" on:close={closeDrawer}>
		<SuperadminSettingsInner {closeDrawer} />
	</DrawerContent>
</Drawer>
