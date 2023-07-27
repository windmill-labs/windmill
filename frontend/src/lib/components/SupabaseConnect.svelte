<script lang="ts">
	// import { oauthStore, workspaceStore } from '$lib/stores'
	import { DrawerContent } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	// import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	// import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'

	let drawer: Drawer
	export async function open() {
		drawer.openDrawer?.()
		step = 'init'
	}

	let step: 'init' | 'resource' = 'init'

	// let token = 'foo'
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Connect or Create a Supabase Database" on:close={drawer.closeDrawer}>
		{#if step === 'init'}
			<h2>Connect an existing database</h2>
			<h2>Create a new database</h2>
		{/if}
	</DrawerContent>
</Drawer>
