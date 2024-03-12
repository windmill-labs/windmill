<script lang="ts">
	import { userStore } from '$lib/stores'
	import { SettingsIcon } from 'lucide-svelte'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DefaultScriptsInner from './DefaultScriptsInner.svelte'
	import Portal from 'svelte-portal'

	let drawer: Drawer
</script>

{#if $userStore?.is_admin || $userStore?.is_super_admin}
	<Portal>
		<Drawer bind:this={drawer} placement="left">
			<DrawerContent title="Edit Default Scripts" on:close={drawer.closeDrawer}>
				<DefaultScriptsInner />
			</DrawerContent>
		</Drawer>
	</Portal>
	<Button
		on:click={drawer?.openDrawer}
		startIcon={{ icon: SettingsIcon }}
		color="light"
		size="xs2"
		variant="contained">defaults</Button
	>
{/if}
