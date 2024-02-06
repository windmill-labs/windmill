<script lang="ts">
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DeployWorkspace from './DeployWorkspace.svelte'

	type Kind = 'script' | 'resource' | 'schedule' | 'variable' | 'flow' | 'app' | 'raw_app'

	let initialPath: string | undefined = undefined
	let kind: Kind | undefined = undefined

	let drawer: Drawer | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let deployWorkspace: DeployWorkspace | undefined = undefined
	export async function openDrawer(initialPath_l: string, kind_l: Kind) {
		initialPath = initialPath_l
		kind = kind_l
		drawer?.openDrawer()
	}
</script>

<Drawer bind:this={drawer} size="900px">
	<DrawerContent title="Deploy {initialPath}" on:close={drawer.closeDrawer}>
		{#if kind != undefined && initialPath != undefined}
			<DeployWorkspace
				hideButton
				{initialPath}
				{kind}
				bind:workspaceToDeployTo
				bind:this={deployWorkspace}
			/>
		{/if}

		<svelte:fragment slot="actions">
			<Button
				disabled={workspaceToDeployTo == undefined}
				on:click={() => deployWorkspace?.deployAll()}>Deploy All</Button
			>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
