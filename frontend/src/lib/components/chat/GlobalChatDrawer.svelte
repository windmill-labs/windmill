<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import GlobalChat from './GlobalChat.svelte'

	export let open = false

	function closeDrawer() {
		open = false
	}
</script>

<Drawer bind:open size="500px" placement="right">
	<DrawerContent title="Global Chat" on:close={closeDrawer}>
		<GlobalChat />
	</DrawerContent>
</Drawer>
