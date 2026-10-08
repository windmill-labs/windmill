<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Section from '$lib/components/Section.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { SaveIcon, EyeIcon, EyeOffIcon } from 'lucide-svelte'
	import { untrack } from 'svelte'

	const defaultVisibility = {
		runs: true,
		schedules: true,
		resources: true,
		variables: true,
		assets: true,
		triggers: true,
		audit_logs: true,
		groups: true,
		folders: true,
		workers: true
	}
	let operatorWorkspaceSettings = $state({ ...defaultVisibility })

	// Kept out of `operatorWorkspaceSettings` so the visibility table's "Enable all" never flips a
	// write right, and so these rows stay out of that table.
	let builderFlows = $state(false)
	let builderApps = $state(false)
	// Withdrawable rather than granted: operators hold these until an admin turns them off.
	let manageSchedules = $state(true)
	let manageTriggers = $state(true)

	let originalSettings = $state({
		...untrack(() => operatorWorkspaceSettings),
		builder_flows: false,
		builder_apps: false,
		manage_schedules: true,
		manage_triggers: true
	})
	let isChanged = $state(false)
	let currentWorkspace: string | null = $state(null)
	let confirmBuilderOpen = $state(false)
	// Saving sends every key, so saving before the load, or over a late response for another
	// workspace, would write defaults over the rights stored there.
	let loadedWorkspace: string | null = $state(null)

	const settingsPayload = $derived({
		...operatorWorkspaceSettings,
		builder_flows: builderFlows,
		builder_apps: builderApps,
		manage_schedules: manageSchedules,
		manage_triggers: manageTriggers
	})

	// The seat cost lands on the first right granted, so confirm only when going from neither to
	// either. Adding the second one later is already paid for.
	const grantsFirstBuilderRight = $derived(
		(builderFlows || builderApps) &&
			!(originalSettings.builder_flows || originalSettings.builder_apps)
	)

	function onSaveClicked() {
		if (grantsFirstBuilderRight) {
			confirmBuilderOpen = true
		} else {
			saveSettings()
		}
	}

	async function saveSettings() {
		try {
			await WorkspaceService.updateOperatorSettings({
				workspace: $workspaceStore!,
				requestBody: settingsPayload
			})
			originalSettings = { ...settingsPayload }
			isChanged = false
			sendUserToast('Operator settings saved successfully!', false)
		} catch (error) {
			console.error('Error updating operator settings:', error)
			sendUserToast(`Failed to save operator settings: ${error?.body ?? error}`, true)
		}
	}

	const descriptions = {
		runs: { title: 'Runs', description: 'View runs' },
		schedules: { title: 'Schedules', description: 'View schedules' },
		resources: { title: 'Resources', description: 'View resources' },
		variables: { title: 'Variables', description: 'View variables' },
		assets: { title: 'Assets', description: 'View assets' },
		triggers: { title: 'Triggers', description: 'View all triggers (HTTP, Websocket, Kafka)' },
		audit_logs: { title: 'Audit Logs', description: 'View audit logs' },
		groups: { title: 'Groups', description: 'View groups and group members' },
		folders: { title: 'Folders', description: 'View folders' },
		workers: { title: 'Workers', description: 'View workers and worker groups' }
	}

	$effect(() => {
		if ($workspaceStore && $workspaceStore !== currentWorkspace) {
			;(async () => {
				const ws = $workspaceStore
				currentWorkspace = ws
				const settings = await WorkspaceService.getSettings({ workspace: ws })
				if (ws !== currentWorkspace) return
				// Every value is reset, null settings included: one kept from the previous workspace
				// would be saved here with the next unrelated change, builder rights among them.
				const {
					builder_flows: remoteFlows,
					builder_apps: remoteApps,
					manage_schedules: remoteSchedules,
					manage_triggers: remoteTriggers,
					...remoteVisibility
				} = settings.operator_settings ?? {}
				operatorWorkspaceSettings = { ...defaultVisibility, ...remoteVisibility }
				builderFlows = remoteFlows ?? false
				builderApps = remoteApps ?? false
				manageSchedules = remoteSchedules ?? true
				manageTriggers = remoteTriggers ?? true
				originalSettings = {
					...operatorWorkspaceSettings,
					builder_flows: builderFlows,
					builder_apps: builderApps,
					manage_schedules: manageSchedules,
					manage_triggers: manageTriggers
				}
				loadedWorkspace = ws
			})()
		}
	})

	$effect(() => {
		isChanged = JSON.stringify(settingsPayload) !== JSON.stringify(originalSettings)
	})

	const allDisabled = $derived(
		Object.values(operatorWorkspaceSettings).every((value) => value === false)
	)
	const allEnabled = $derived(
		Object.values(operatorWorkspaceSettings).every((value) => value === true)
	)
</script>

<Section
	label="Operator settings"
	collapsable={true}
	tooltip="Operators run what is shared with them. Choose what else they can change and see in this workspace."
	description="Operators run what is shared with them. Choose what else they can change and see in this workspace."
>
	{#snippet action()}
		<Button
			on:click={onSaveClicked}
			startIcon={{ icon: SaveIcon }}
			disabled={!isChanged || loadedWorkspace !== $workspaceStore}
			variant="accent"
		>
			Save operator settings
		</Button>
	{/snippet}

	<Section
		small
		label="Build flows and apps"
		description="Let operators build flows out of deployed scripts and flows, and full-code apps whose frontend code they write. Apps always run sandboxed, and operators still cannot write backend scripts. Granting either right makes each operator consume a full seat instead of half a seat."
		wrapperClass="mb-6"
		class="flex flex-col gap-y-1"
	>
		<Toggle
			bind:checked={builderFlows}
			options={{ right: 'Operators can build flows' }}
			size="xs"
		/>
		<Toggle
			bind:checked={builderApps}
			options={{ right: 'Operators can build full-code apps' }}
			size="xs"
		/>
	</Section>

	<Section
		small
		label="Change schedules and triggers"
		description="Operators can create, edit and delete schedules and triggers wherever their folder permissions let them write. When turned off, the server refuses these changes, including through the API and the CLI."
		wrapperClass="mb-6"
		class="flex flex-col gap-y-1"
	>
		<Toggle
			bind:checked={manageSchedules}
			options={{ right: 'Operators can manage schedules' }}
			size="xs"
		/>
		<Toggle
			bind:checked={manageTriggers}
			options={{ right: 'Operators can manage triggers' }}
			size="xs"
		/>
	</Section>

	<Section
		small
		label="Pages in their menu"
		description="Hides pages from an operator's menu. It does not block access through the API: use folder and item permissions to restrict what they can read."
	>
		<DataTable tableFixed={true} size="xs">
			<Head>
				<tr>
					<Cell head first>Section</Cell>
					<Cell head>Description</Cell>
					<Cell head last>
						<ToggleButtonGroup
							bind:selected={
								() => (allDisabled ? 'false' : allEnabled ? 'true' : ''),
								(v) => {
									Object.keys(operatorWorkspaceSettings).forEach((key) => {
										if (v === 'true') operatorWorkspaceSettings[key] = true
										if (v === 'false') operatorWorkspaceSettings[key] = false
									})
								}
							}
						>
							{#snippet children({ item })}
								<ToggleButton
									icon={EyeIcon}
									small={true}
									value={'true'}
									label="Enable All"
									{item}
								/>
								<ToggleButton
									icon={EyeOffIcon}
									small={true}
									value={'false'}
									label="Disable All"
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>
					</Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface">
				{#each Object.entries(descriptions) as [key, { title, description }]}
					<tr>
						<Cell first>{title}</Cell>
						<Cell>{description}</Cell>
						<Cell last class="pl-8">
							<ToggleButtonGroup
								selected={operatorWorkspaceSettings[key] ? 'on' : 'off'}
								on:selected={({ detail }) => (operatorWorkspaceSettings[key] = detail === 'on')}
							>
								{#snippet children({ item })}
									<ToggleButton icon={EyeIcon} small={true} value={'on'} label="On" {item} />
									<ToggleButton icon={EyeOffIcon} small={true} value={'off'} label="Off" {item} />
								{/snippet}
							</ToggleButtonGroup>
						</Cell>
					</tr>
				{/each}
			</tbody>
		</DataTable>
	</Section>
</Section>

<ConfirmationModal
	open={confirmBuilderOpen}
	title="Give operators builder rights"
	type="info"
	confirmationText="Enable builder rights"
	onCanceled={() => (confirmBuilderOpen = false)}
	onConfirmed={async () => {
		confirmBuilderOpen = false
		await saveSettings()
	}}
>
	<div class="flex flex-col gap-2 text-sm">
		<span>This applies to every operator of this workspace, not to a chosen few.</span>
		<span>
			Each of them then consumes a full seat instead of half a seat, which changes what this
			instance is billed.
		</span>
		<span>
			They can create, edit and delete flows and full-code apps wherever their folder permissions
			already let them write. Review those permissions before enabling.
		</span>
	</div>
</ConfirmationModal>
