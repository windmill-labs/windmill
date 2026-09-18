<script lang="ts">
	import { Button } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Section from '$lib/components/Section.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { SaveIcon, EyeIcon, EyeOffIcon } from 'lucide-svelte'
	import { untrack } from 'svelte'

	let operatorWorkspaceSettings = $state({
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
	})

	// Kept out of `operatorWorkspaceSettings` so the visibility table's "Enable all" never flips a
	// write right, and so these rows stay out of that table. Withdrawable rather than granted:
	// operators hold them until an admin turns them off.
	let manageSchedules = $state(true)
	let manageTriggers = $state(true)

	let originalSettings = $state({
		...untrack(() => operatorWorkspaceSettings),
		manage_schedules: true,
		manage_triggers: true
	})
	let isChanged = $state(false)
	let currentWorkspace: string | null = $state(null)

	const settingsPayload = $derived({
		...operatorWorkspaceSettings,
		manage_schedules: manageSchedules,
		manage_triggers: manageTriggers
	})

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
			sendUserToast('Failed to save operator settings.', true)
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
				currentWorkspace = $workspaceStore
				const settings = await WorkspaceService.getSettings({
					workspace: $workspaceStore
				})
				if (settings.operator_settings !== null) {
					const {
						manage_schedules: remoteSchedules,
						manage_triggers: remoteTriggers,
						...remoteVisibility
					} = settings.operator_settings ?? {}
					operatorWorkspaceSettings = { ...operatorWorkspaceSettings, ...remoteVisibility }
					manageSchedules = remoteSchedules ?? true
					manageTriggers = remoteTriggers ?? true
					originalSettings = {
						...operatorWorkspaceSettings,
						manage_schedules: manageSchedules,
						manage_triggers: manageTriggers
					}
				}
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
	tooltip="Configure the operator visibility settings for your workspace. Toggle the settings you want to enable."
	description="Configure the operator visibility settings for your workspace. Toggle the settings you want to enable."
>
	{#snippet action()}
		<Button
			on:click={saveSettings}
			startIcon={{ icon: SaveIcon }}
			disabled={!isChanged}
			variant="accent"
		>
			Save operator settings
		</Button>
	{/snippet}

	<div class="flex flex-col gap-y-1 mb-4">
		<span class="text-xs font-semibold text-emphasis">Schedules and triggers</span>
		<span class="text-xs font-normal text-secondary">
			Operators can create, edit and delete schedules and triggers wherever their folder permissions
			let them write. Turn these off to withdraw that.
		</span>
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
	</div>

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
							<ToggleButton icon={EyeIcon} small={true} value={'true'} label="Enable All" {item} />
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
