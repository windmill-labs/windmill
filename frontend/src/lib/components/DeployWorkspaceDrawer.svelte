<script lang="ts">
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DeployWorkspace from './DeployWorkspace.svelte'
	import { type AdditionalInformation, type Kind } from '$lib/utils_deployable'

	let initialPath: string | undefined = undefined
	let kind: Kind | undefined = undefined
	let drawer: Drawer | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let deployWorkspace: DeployWorkspace | undefined = undefined
	let additionalInformation: AdditionalInformation | undefined = undefined

	export async function openDrawer(
		initialPath_l: string,
		kind_l: Kind,
		additionalInformation_l?: AdditionalInformation
	) {
		additionalInformation = additionalInformation_l
		initialPath = initialPath_l
		kind = kind_l
		drawer?.openDrawer()
	}
</script>

<Drawer bind:this={drawer} size="900px">
	<DrawerContent title="Deploy {initialPath}" on:close={drawer.closeDrawer}>
		{#if (kind != 'trigger' && kind != undefined && initialPath != undefined) || (kind === 'trigger' && initialPath != undefined && additionalInformation?.triggers != undefined)}
			<DeployWorkspace
				hideButton
				{initialPath}
				{kind}
				{additionalInformation}
				bind:workspaceToDeployTo
				bind:this={deployWorkspace}
			/>
		{/if}

		{#snippet actions()}
			<Button
				disabled={workspaceToDeployTo == undefined}
				on:click={() => deployWorkspace?.deployAll()}>Deploy All</Button
			>
		{/snippet}
	</DrawerContent>
</Drawer>
