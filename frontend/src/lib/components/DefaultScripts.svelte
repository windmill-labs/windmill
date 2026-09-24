<script lang="ts">
	import { SettingsIcon } from 'lucide-svelte'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DefaultScriptsInner from './DefaultScriptsInner.svelte'
	import { useOperatingUser } from '$lib/components/operatingWorkspace.svelte'

	const operatingUser = useOperatingUser()
	const actingUser = $derived(operatingUser.current)

	interface Props {
		placement?: 'left' | 'right'
		size?: 'xs3' | 'xs2'
		noText?: boolean
	}

	let { placement = 'left', size = 'xs2', noText = false }: Props = $props()

	let drawer: Drawer | undefined = $state()
</script>

{#if actingUser?.is_admin || actingUser?.is_super_admin}
	<Drawer bind:this={drawer} {placement}>
		<DrawerContent title="Edit Default Scripts" on:close={drawer?.closeDrawer}>
			<DefaultScriptsInner />
		</DrawerContent>
	</Drawer>
	<Button on:click={drawer?.openDrawer} startIcon={{ icon: SettingsIcon }} variant="subtle" {size}>
		{noText ? '' : 'defaults'}
	</Button>
{/if}
