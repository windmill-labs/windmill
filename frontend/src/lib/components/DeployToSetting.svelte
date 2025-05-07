<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { enterpriseLicense, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { fade } from 'svelte/transition'
	import Tooltip from './Tooltip.svelte'
	import { Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import Toggle from './Toggle.svelte'
	import { emptyString } from '$lib/utils'

	$: deployableWorkspaces = $usersWorkspaceStore?.workspaces
		.map((w) => w.id)
		.filter((w) => w != $workspaceStore)

	type DeployUITypeMap = {
		scripts: boolean
		flows: boolean
		apps: boolean
		resources: boolean
		variables: boolean
		secrets: boolean
		triggers: boolean
	}
	type DeployUIType = 'script' | 'flow' | 'app' | 'resource' | 'variable' | 'secret' | 'trigger'

	const all_ok: DeployUITypeMap = {
		scripts: true,
		flows: true,
		apps: true,
		resources: true,
		variables: true,
		secrets: true,
		triggers: true
	}

	export let workspaceToDeployTo: string | undefined
	export let deployUiSettings: {
		include_path: string[]
		include_type: DeployUITypeMap
	} = {
		include_path: [],
		include_type: all_ok
	}
	function deployUITypeMapToArray(
		typesMap: DeployUITypeMap,
		expectedValue: boolean
	): DeployUIType[] {
		let result: DeployUIType[] = []
		if (typesMap.scripts == expectedValue) {
			result.push('script')
		}
		if (typesMap.flows == expectedValue) {
			result.push('flow')
		}
		if (typesMap.apps == expectedValue) {
			result.push('app')
		}
		if (typesMap.resources == expectedValue) {
			result.push('resource')
		}
		if (typesMap.variables == expectedValue) {
			result.push('variable')
		}
		if (typesMap.secrets == expectedValue) {
			result.push('secret')
		}
		if (typesMap.triggers == expectedValue) {
			result.push('trigger')
		}
		return result
	}

	async function editWindmillDeploymentUISettings() {
		let include_path = deployUiSettings.include_path.filter((elmt) => !emptyString(elmt))
		let include_type = deployUITypeMapToArray(deployUiSettings.include_type, true)
		await WorkspaceService.editWorkspaceDeployUiSettings({
			workspace: $workspaceStore!,
			requestBody: {
				deploy_ui_settings: {
					include_path: include_path,
					include_type: include_type
				}
			}
		})
		sendUserToast('Workspace Deployment UI settings updated')
	}
</script>

<h3 class="mt-8">Workspace to link to</h3>
<div class="flex min-w-0 mt-2">
	<select
		bind:value={workspaceToDeployTo}
		on:change={async (e) => {
			await WorkspaceService.editDeployTo({
				workspace: $workspaceStore ?? '',
				requestBody: { deploy_to: workspaceToDeployTo == '' ? undefined : workspaceToDeployTo }
			})
			if (workspaceToDeployTo == '') {
				workspaceToDeployTo = undefined
				sendUserToast('Disabled setting deployable workspace')
			} else {
				sendUserToast('Set deployable workspace to ' + workspaceToDeployTo)
			}
		}}
	>
		{#if deployableWorkspaces?.length == 0}
			<option disabled>No workspace deployable to</option>
		{/if}
		<option value="">Disable deployment</option>
		{#each deployableWorkspaces ?? [] as name}
			<option value={name}>{name}</option>
		{/each}
	</select>
</div>
<h3 class="mt-6 mb-3">Deployable items</h3>
<div class="flex flex-wrap gap-20">
	<div class="max-w-md w-full">
		{#if Array.isArray(deployUiSettings?.include_path)}
			<h4 class="flex gap-2 mb-4"
				>Filter on path<Tooltip>
					Only scripts, flows and apps with their path matching one of those filters will be allowed
					to be deployed in the deploy UI. The filters allow '*'' and '**' characters, with '*''
					matching any character allowed in paths until the next slash (/) and '**' matching
					anything including slashes.
				</Tooltip></h4
			>
			{#each deployUiSettings.include_path ?? [] as regexpPath, idx}
				<div class="flex mt-1 items-center">
					<input type="text" bind:value={regexpPath} id="arg-input-array" />
					<button
						transition:fade|local={{ duration: 100 }}
						class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
						aria-label="Clear"
						on:click={() => {
							deployUiSettings.include_path.splice(idx, 1)
							deployUiSettings.include_path = [...deployUiSettings.include_path]
						}}
					>
						<X size={14} />
					</button>
				</div>
			{/each}
		{/if}
		<div class="flex mt-2">
			<Button
				variant="border"
				color="light"
				size="xs"
				btnClasses="mt-1"
				on:click={() => {
					deployUiSettings.include_path = [...deployUiSettings.include_path, '']
				}}
				id="deploy-ui-add-path-filter"
				startIcon={{ icon: Plus }}
			>
				Add filter
			</Button>
		</div>
	</div>

	<div class="max-w-md w-full">
		<h4 class="flex gap-2 mb-4"
			>Filter on type<Tooltip>
				You can filter which types of item can be deployed to the production workspace. By default
				everything is deployable.
			</Tooltip></h4
		>
		<div class="flex flex-col gap-2 mt-1">
			<Toggle bind:checked={deployUiSettings.include_type.scripts} options={{ right: 'Scripts' }} />
			<Toggle bind:checked={deployUiSettings.include_type.flows} options={{ right: 'Flows' }} />
			<Toggle bind:checked={deployUiSettings.include_type.apps} options={{ right: 'Apps' }} />
			<Toggle
				bind:checked={deployUiSettings.include_type.resources}
				options={{ right: 'Resources' }}
			/>
			<div class="flex gap-3">
				<Toggle
					bind:checked={deployUiSettings.include_type.variables}
					on:change={(ev) => {
						if (!ev.detail) {
							deployUiSettings.include_type.secrets = false
						}
					}}
					options={{ right: 'Variables ' }}
				/>
				<span>-</span>
				<Toggle
					disabled={!deployUiSettings.include_type.variables}
					bind:checked={deployUiSettings.include_type.secrets}
					options={{ left: 'Include secrets' }}
				/>
			</div>
			<Toggle
				bind:checked={deployUiSettings.include_type.triggers}
				options={{ right: 'Trigger' }}
			/>
		</div>
	</div>
</div>
{#if $enterpriseLicense}
	<div class="flex mt-5 mb-5 gap-1">
		<Button
			color="blue"
			disabled={workspaceToDeployTo == undefined}
			on:click={() => {
				editWindmillDeploymentUISettings()
			}}>Save Deployment UI settings</Button
		>
	</div>
{/if}
