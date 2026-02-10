<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import SuperadminSettingsInner from './SuperadminSettingsInner.svelte'

	interface Props {
		disableChatOffset?: boolean
	}

	let { disableChatOffset = false }: Props = $props()

	let drawer: Drawer | undefined = $state()

	export function openDrawer() {
		drawer?.openDrawer()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
	}
</script>

<Drawer bind:this={drawer} size="1100px" {disableChatOffset}>
	<DrawerContent noPadding title="Instance settings" on:close={closeDrawer}>
		<SuperadminSettingsInner {closeDrawer} />
	</DrawerContent>
</Drawer>
