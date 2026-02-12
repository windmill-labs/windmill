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
	import { validateDeployPathFilters } from '$lib/validators/workspaceSettings'
	import Alert from './common/alert/Alert.svelte'
	import SettingsFooter from './workspaceSettings/SettingsFooter.svelte'
	import SettingCard from './instanceSettings/SettingCard.svelte'
	import Select from './select/Select.svelte'

	let deployableWorkspaces = $derived(
		$usersWorkspaceStore?.workspaces.map((w) => w.id).filter((w) => w != $workspaceStore)
	)

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

	let {
		workspaceToDeployTo = $bindable(),
		deployUiSettings = $bindable({
			include_path: [],
			include_type: all_ok
		}),
		hasUnsavedChanges = false,
		onSave,
		onDiscard,
		onWorkspaceToDeployToSave
	}: {
		workspaceToDeployTo: string | undefined
		deployUiSettings: {
			include_path: string[]
			include_type: DeployUITypeMap
		}
		hasUnsavedChanges?: boolean
		onSave?: () => void
		onDiscard: () => void
		onWorkspaceToDeployToSave?: (workspaceToDeployTo: string | undefined) => void
	} = $props()

	// Validation state
	let pathValidationErrors: Record<number, string> = $state({})
	let hasValidationErrors = $derived(Object.keys(pathValidationErrors).length > 0)

	// Validate path filters whenever they change
	$effect(() => {
		if (deployUiSettings?.include_path) {
			const validationResult = validateDeployPathFilters(deployUiSettings.include_path)
			pathValidationErrors = validationResult.errors
		}
	})
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

	async function editWorkspaceToDeployTo() {
		try {
			await WorkspaceService.editDeployTo({
				workspace: $workspaceStore ?? '',
				requestBody: { deploy_to: workspaceToDeployTo === '' ? undefined : workspaceToDeployTo }
			})

			if (workspaceToDeployTo === '' || workspaceToDeployTo === undefined) {
				sendUserToast('Disabled setting deployable workspace')
				onWorkspaceToDeployToSave?.(undefined)
			} else {
				sendUserToast('Set deployable workspace to ' + workspaceToDeployTo)
				onWorkspaceToDeployToSave?.(workspaceToDeployTo)
			}
		} catch (error) {
			sendUserToast(`Failed to save workspace deployment setting: ${error}`, true)
		}
	}

	async function editWindmillDeploymentUISettings() {
		// Validate before saving
		const validationResult = validateDeployPathFilters(deployUiSettings.include_path)
		if (!validationResult.isValid) {
			sendUserToast('Please fix validation errors before saving', true)
			return
		}

		let include_path = deployUiSettings.include_path.filter((elmt) => !emptyString(elmt))
		let include_type = deployUITypeMapToArray(deployUiSettings.include_type, true)

		try {
			// Save workspace to deploy to first
			await editWorkspaceToDeployTo()

			// Then save deployment UI settings
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
			onSave?.()
		} catch (error) {
			sendUserToast(`Failed to save deployment settings: ${error}`, true)
		}
	}
</script>

<SettingCard label="Workspace to link to" class="mt-6">
	<Select
		items={[
			{ label: 'Disable deployment', value: '' },
			...(deployableWorkspaces ?? []).map((w) => ({ label: w, value: w }))
		]}
		bind:value={workspaceToDeployTo}
		placeholder={deployableWorkspaces?.length === 0
			? 'No workspace deployable to'
			: 'Select workspace'}
	/>
</SettingCard>
<SettingCard
	label="Deployable items"
	description="You can filter which items can be deployed to the production workspace. By default everything is deployable."
	class="mt-6"
>
	<div class="flex flex-wrap gap-6 mt-2">
		<div class="max-w-md w-full">
			{#if Array.isArray(deployUiSettings?.include_path)}
				<h4 class="flex gap-2 mb-2 text-xs font-semibold text-emphasis"
					>Filter on path<Tooltip>
						Only scripts, flows and apps with their path matching one of those filters will be
						allowed to be deployed in the deploy UI. The filters allow '*'' and '**' characters,
						with '*'' matching any character allowed in paths until the next slash (/) and '**'
						matching anything including slashes.
					</Tooltip></h4
				>
				{#each deployUiSettings.include_path ?? [] as _, idx}
					<div class="flex flex-col mt-1">
						<div class="flex items-center">
							<input
								type="text"
								bind:value={deployUiSettings.include_path[idx]}
								id="arg-input-array-{idx}"
								class="flex-1 {pathValidationErrors[idx] ? 'border-red-500' : ''}"
								placeholder="e.g., f/*, u/admin/**"
							/>
							<button
								transition:fade|local={{ duration: 100 }}
								class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover ml-2"
								aria-label="Clear"
								onclick={() => {
									deployUiSettings.include_path.splice(idx, 1)
									deployUiSettings.include_path = [...deployUiSettings.include_path]
									// Clear validation error for this index
									delete pathValidationErrors[idx]
									pathValidationErrors = { ...pathValidationErrors }
								}}
							>
								<X size={14} />
							</button>
						</div>
						{#if pathValidationErrors[idx]}
							<div class="text-xs text-red-600 dark:text-red-400 mt-1"
								>{pathValidationErrors[idx]}</div
							>
						{/if}
					</div>
				{/each}
			{/if}
			<div class="flex mt-2">
				<Button
					variant="default"
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
			<h4 class="flex gap-2 mb-2 text-xs font-semibold text-emphasis"
				>Filter on type<Tooltip>
					You can filter which types of item can be deployed to the production workspace. By default
					everything is deployable.
				</Tooltip></h4
			>
			<div class="flex flex-col gap-2 mt-1">
				<Toggle
					bind:checked={deployUiSettings.include_type.scripts}
					options={{ right: 'Scripts' }}
				/>
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
</SettingCard>
{#if hasValidationErrors}
	<Alert type="error" title="Validation Errors" class="mt-4">
		Please fix the validation errors in the path filters before saving.
	</Alert>
{/if}
{#if $enterpriseLicense}
	<SettingsFooter
		{hasUnsavedChanges}
		onSave={editWindmillDeploymentUISettings}
		{onDiscard}
		saveLabel="Save deployment UI"
		disabled={workspaceToDeployTo == undefined || hasValidationErrors}
		class="mt-8"
	/>
{/if}
