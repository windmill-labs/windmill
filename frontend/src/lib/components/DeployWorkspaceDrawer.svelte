<script lang="ts">
	import { Button, Drawer } from './common'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import DeployWorkspace from './DeployWorkspace.svelte'
	import ParentWorkspaceProtectionAlert from './ParentWorkspaceProtectionAlert.svelte'
	import { type AdditionalInformation, type Kind } from '$lib/utils_deployable'

	let initialPath: string | undefined = $state(undefined)
	let kind: Kind | undefined = $state(undefined)
	let drawer: Drawer | undefined = $state(undefined)
	let workspaceToDeployTo: string | undefined = $state(undefined)
	let deployWorkspace: DeployWorkspace | undefined = $state(undefined)
	let additionalInformation: AdditionalInformation | undefined = $state(undefined)
	let canDeployToWorkspace = $state(true)

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
		{#if workspaceToDeployTo}
			<ParentWorkspaceProtectionAlert
				parentWorkspaceId={workspaceToDeployTo}
				onUpdateCanDeploy={(canDeploy) => {
					canDeployToWorkspace = canDeploy
				}}
			/>
		{/if}
		{#if (kind != 'trigger' && kind != undefined && initialPath != undefined) || (kind === 'trigger' && initialPath != undefined && additionalInformation?.triggers != undefined)}
			<DeployWorkspace
				hideButton
				{initialPath}
				{kind}
				{additionalInformation}
				bind:workspaceToDeployTo
				bind:this={deployWorkspace}
				bind:canDeployToWorkspace
			/>
		{/if}

		{#snippet actions()}
			<Button
				disabled={workspaceToDeployTo == undefined || !canDeployToWorkspace}
				on:click={() => deployWorkspace?.deployAll()}>Deploy All</Button
			>
		{/snippet}
	</DrawerContent>
</Drawer>
