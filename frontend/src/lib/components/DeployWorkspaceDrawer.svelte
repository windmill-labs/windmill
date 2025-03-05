<script lang="ts">
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DeployWorkspace from './DeployWorkspace.svelte'
	import { type AdditionalInformations, type Kind } from '$lib/utils_deployable'

	let initialPath: string | undefined = undefined
	let kind: Kind | undefined = undefined
	let drawer: Drawer | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let deployWorkspace: DeployWorkspace | undefined = undefined
	let additionalInformations: AdditionalInformations | undefined = undefined

	export async function openDrawer(
		initialPath_l: string,
		kind_l: Kind,
		additionalInformations_l?: AdditionalInformations
	) {
		additionalInformations = additionalInformations_l
		initialPath = initialPath_l
		kind = kind_l
		drawer?.openDrawer()
	}
</script>

<Drawer bind:this={drawer} size="900px">
	<DrawerContent title="Deploy {initialPath}" on:close={drawer.closeDrawer}>
		{#if (kind != 'triggers' && kind != undefined && initialPath != undefined) || (kind === 'triggers' && initialPath != undefined && additionalInformations?.triggers != undefined)}
			<DeployWorkspace
				hideButton
				{initialPath}
				{kind}
				{additionalInformations}
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
