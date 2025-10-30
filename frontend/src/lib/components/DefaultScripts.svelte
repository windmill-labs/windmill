<script lang="ts">
	import { userStore } from '$lib/stores'
	import { SettingsIcon } from 'lucide-svelte'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DefaultScriptsInner from './DefaultScriptsInner.svelte'

	let drawer: Drawer | undefined = $state()

	interface Props {
		placement?: 'left' | 'right'
		size?: 'xs3' | 'xs2'
		noText?: boolean
	}

	let { placement = 'left', size = 'xs2', noText = false }: Props = $props()
</script>

{#if $userStore?.is_admin || $userStore?.is_super_admin}
	<Drawer bind:this={drawer} {placement}>
		<DrawerContent title="Edit Default Scripts" on:close={drawer.closeDrawer}>
			<DefaultScriptsInner />
		</DrawerContent>
	</Drawer>
	<Button on:click={drawer?.openDrawer} startIcon={{ icon: SettingsIcon }} variant="subtle" {size}>
		{noText ? '' : 'defaults'}
	</Button>
{/if}
